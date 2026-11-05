//! AWS MSK (Managed Streaming for Kafka) discovery.
//!
//! Two calls: list the clusters, then ask for their bootstrap brokers. Both use
//! the JSON protocol, which SigV4 treats the same way as the Query API.

use std::collections::BTreeMap;

use crate::cloud::aws::{self, sigv4};
use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::config::Endpoint;
use crate::error::AppResult;

const SERVICE: &str = "kafka";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    aws::require_keys(credentials)?;
    let region = aws::region(credentials);

    // 1. Which clusters exist.
    let list_url = format!("https://kafka.{region}.amazonaws.com/v1/clusters");
    let list = call(credentials, &region, "GET", &list_url, None, timeout).await?;
    let clusters = list
        .get("clusterInfoList")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for cluster in clusters {
        let arn = cluster
            .get("clusterArn")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if arn.is_empty() {
            continue;
        }
        let name = cluster
            .get("clusterName")
            .and_then(|value| value.as_str())
            .unwrap_or("msk-cluster")
            .to_string();
        let state = cluster
            .get("state")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();

        let mut meta = BTreeMap::new();
        if !state.is_empty() {
            meta.insert("state".into(), state.clone());
        }
        if let Some(version) = cluster.get("kafkaVersion").and_then(|v| v.as_str()) {
            meta.insert("kafka_version".into(), version.to_string());
        }
        if let Some(brokers) = cluster.get("numberOfBrokerNodes").and_then(|v| v.as_i64()) {
            meta.insert("brokers".into(), brokers.to_string());
        }
        // MSK supports several auth modes; record the one that is switched on.
        let encryption_in_transit = cluster
            .pointer("/encryptionInfo/encryptionInTransit/clientBroker")
            .and_then(|v| v.as_str())
            .unwrap_or("PLAINTEXT")
            .to_string();
        meta.insert(
            "encryption_in_transit".into(),
            encryption_in_transit.clone(),
        );

        // Bootstrap brokers are only available once the cluster is ACTIVE.
        let mut endpoints = Vec::new();
        if state == "ACTIVE" {
            match bootstrap_brokers(credentials, &region, &arn, timeout).await {
                Ok(found) => endpoints = found,
                Err(err) => {
                    meta.insert("endpoint_error".into(), err.message);
                }
            }
        }

        let mut resource = aws::resource(
            arn.clone(),
            name,
            "aws_msk",
            crate::config::BackendKind::Kafka,
            region.clone(),
            endpoints,
            encryption_in_transit != "PLAINTEXT",
            None,
            meta,
        );
        // MSK uses IAM (SASL/OAUTHBEARER) or no auth at all; our Kafka driver
        // supports SASL/PLAIN, so an IAM-only cluster needs a token broker.
        resource.needs_password = false;
        out.push(resource);
    }

    Ok(out)
}

async fn bootstrap_brokers(
    credentials: &CloudCredentials,
    region: &str,
    arn: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<Endpoint>> {
    // The ARN must be URL-encoded in the path.
    let encoded = arn.replace(':', "%3A").replace('/', "%2F");
    let url = format!(
        "https://kafka.{region}.amazonaws.com/v1/clusters/{encoded}/bootstrap-brokers"
    );
    let body = call(credentials, region, "GET", &url, None, timeout).await?;

    let mut out = Vec::new();
    if let Some(list) = body.get("bootstrapBrokerStringTls").and_then(|v| v.as_str()) {
        out.extend(parse_broker_list(list, 9094));
    }
    if out.is_empty() {
        if let Some(list) = body.get("bootstrapBrokerString").and_then(|v| v.as_str()) {
            out.extend(parse_broker_list(list, 9092));
        }
    }
    Ok(out)
}

/// `a:9094,b:9094` -> endpoints.
fn parse_broker_list(list: &str, fallback_port: u16) -> Vec<Endpoint> {
    list.split(',')
        .filter_map(|entry| {
            let entry = entry.trim();
            if entry.is_empty() {
                return None;
            }
            match entry.rsplit_once(':') {
                Some((host, port)) => port
                    .parse::<u16>()
                    .ok()
                    .map(|port| Endpoint::new(host, port)),
                None => Some(Endpoint::new(entry, fallback_port)),
            }
        })
        .collect()
}

/// Sign and execute one MSK REST call.
async fn call(
    credentials: &CloudCredentials,
    region: &str,
    method: &'static str,
    url: &str,
    body: Option<&[u8]>,
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let body = body.unwrap_or(b"");
    let signed = sigv4::sign(sigv4::SignRequest {
        method,
        url,
        region,
        service: SERVICE,
        access_key_id: &credentials.access_key_id,
        secret_access_key: &credentials.secret_access_key,
        session_token: credentials.session_token.as_deref(),
        body,
        headers: vec![],
        timestamp: chrono::Utc::now().timestamp(),
    })?;

    let mut request = if method == "POST" {
        crate::cloud::http::Request::post(url, body.to_vec())
    } else {
        crate::cloud::http::Request::get(url)
    };
    for (key, value) in signed.headers {
        request = request.header(key, value);
    }

    let response = crate::cloud::http::execute(&request, timeout).await?;
    if !response.is_success() {
        return Err(aws::aws_error(
            &String::from_utf8_lossy(&response.body),
            response.status,
        ));
    }
    response.json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_broker_list() {
        let found = parse_broker_list("b-1.msk:9094,b-2.msk:9094", 9092);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].host, "b-1.msk");
        assert_eq!(found[0].port, 9094);
    }

    #[test]
    fn uses_the_fallback_port_when_absent() {
        let found = parse_broker_list("b-1.msk", 9092);
        assert_eq!(found[0].port, 9092);
    }

    #[test]
    fn ignores_blank_entries() {
        assert!(parse_broker_list(" , ,", 9092).is_empty());
    }
}
