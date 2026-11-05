//! Confluent Cloud discovery.
//!
//! The Confluent API is a two-step walk: list environments, then list the Kafka
//! clusters in each one.

use std::collections::BTreeMap;

use crate::cloud::{CloudCredentials, CloudProvider, DiscoveredResource};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

const BASE: &str = "https://api.confluent.cloud";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let key = credentials
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Confluent API key is required"))?;
    let secret = credentials
        .api_secret
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Confluent API secret is required"))?;

    let environments = get(
        &format!("{BASE}/org/v2/environments?page_size=100"),
        &key,
        &secret,
        timeout,
    )
    .await?;

    let mut out = Vec::new();
    let Some(items) = environments.get("data").and_then(|value| value.as_array()) else {
        return Ok(out);
    };

    for environment in items {
        let Some(env_id) = environment.get("id").and_then(|value| value.as_str()) else {
            continue;
        };
        let env_name = environment
            .pointer("/attributes/display_name")
            .and_then(|value| value.as_str())
            .unwrap_or(env_id);

        // One failing environment must not hide the others.
        let clusters = match get(
            &format!("{BASE}/cmk/v2/clusters?environment={env_id}&page_size=100"),
            &key,
            &secret,
            timeout,
        )
        .await
        {
            Ok(json) => json,
            Err(_) => continue,
        };
        let Some(clusters) = clusters.get("data").and_then(|value| value.as_array()) else {
            continue;
        };

        for cluster in clusters {
            let Some(id) = cluster.get("id").and_then(|value| value.as_str()) else {
                continue;
            };
            let name = cluster
                .pointer("/attributes/display_name")
                .and_then(|value| value.as_str())
                .unwrap_or("kafka-cluster");
            let endpoint = cluster
                .pointer("/spec/kafka_bootstrap_endpoint")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            if endpoint.is_empty() {
                continue;
            }
            let Some(parsed) = parse_bootstrap(endpoint) else {
                continue;
            };

            let mut meta = BTreeMap::new();
            meta.insert("environment".into(), env_name.to_string());
            meta.insert("environment_id".into(), env_id.to_string());
            if let Some(region) = cluster
                .pointer("/spec/region")
                .and_then(|value| value.as_str())
            {
                meta.insert("region".into(), region.to_string());
            }
            if let Some(cloud) = cluster
                .pointer("/spec/cloud")
                .and_then(|value| value.as_str())
            {
                meta.insert("cloud".into(), cloud.to_string());
            }
            if let Some(availability) = cluster
                .pointer("/spec/availability")
                .and_then(|value| value.as_str())
            {
                meta.insert("availability".into(), availability.to_string());
            }

            out.push(DiscoveredResource {
                id: format!("{env_id}/{id}"),
                name: format!("{name} ({env_name})"),
                provider: CloudProvider::Confluent,
                service: "confluent_kafka".into(),
                backend: BackendKind::Kafka,
                region: cluster
                    .pointer("/spec/region")
                    .and_then(|value| value.as_str())
                    .map(str::to_string),
                endpoints: parsed,
                // Bootstrap endpoints are always SASL_SSL.
                tls: true,
                // The cluster API key doubles as the data-plane SASL user, so
                // the same key is used for both calls.
                username: Some(key.clone()),
                database: None,
                meta,
                needs_password: true,
            });
        }
    }

    Ok(out)
}

/// `SASL_SSL://pkc-1.us-east-1.aws.confluent.cloud:9092` -> endpoints.
fn parse_bootstrap(endpoint: &str) -> Option<Vec<Endpoint>> {
    let rest = endpoint
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(endpoint);
    let mut out = Vec::new();
    for entry in rest.split(',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        match entry.rsplit_once(':') {
            Some((host, port)) => {
                if let Ok(port) = port.parse::<u16>() {
                    out.push(Endpoint::new(host, port));
                }
            }
            None => out.push(Endpoint::new(entry, 9092)),
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

async fn get(
    url: &str,
    key: &str,
    secret: &str,
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let request = crate::cloud::http::Request::get(url).basic(key, secret);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    response.json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_sasl_ssl_bootstrap_endpoint() {
        let found =
            parse_bootstrap("SASL_SSL://pkc-1.us-east-1.aws.confluent.cloud:9092").unwrap();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].host, "pkc-1.us-east-1.aws.confluent.cloud");
        assert_eq!(found[0].port, 9092);
    }

    #[test]
    fn parses_multiple_brokers() {
        let found = parse_bootstrap("pkc-1:9092,pkc-2:9092").unwrap();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn rejects_an_empty_endpoint() {
        assert!(parse_bootstrap("").is_none());
        assert!(parse_bootstrap("SASL_SSL://").is_none());
    }
}
