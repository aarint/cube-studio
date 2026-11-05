//! Amazon MQ discovery (managed RabbitMQ and ActiveMQ).
//!
//! Amazon MQ exposes its RabbitMQ AMQP endpoint plus web console over the `mq`
//! REST API. The credentials for the broker are the broker's own RabbitMQ user,
//! created by AWS at provisioning time and retrieved through a separate call.

use std::collections::BTreeMap;

use crate::cloud::aws::{self, sigv4};
use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::config::Endpoint;
use crate::error::AppResult;

const SERVICE: &str = "mq";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    aws::require_keys(credentials)?;
    let region = aws::region(credentials);

    let url = format!("https://mq.{region}.amazonaws.com/v1/brokers");
    let body = call(credentials, &region, &url, timeout).await?;

    let brokers = body
        .get("brokerSummaries")
        .and_then(|value| value.as_array())
        .cloned()
        .unwrap_or_default();

    let mut out = Vec::new();
    for broker in brokers {
        let id = broker
            .get("brokerId")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if id.is_empty() {
            continue;
        }
        let name = broker
            .get("brokerName")
            .and_then(|value| value.as_str())
            .unwrap_or("mq-broker")
            .to_string();
        let engine = broker
            .get("engineType")
            .and_then(|value| value.as_str())
            .unwrap_or("RABBITMQ")
            .to_string();
        let state = broker
            .get("brokerState")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();

        let mut meta = BTreeMap::new();
        meta.insert("engine".into(), engine.clone());
        if !state.is_empty() {
            meta.insert("state".into(), state.clone());
        }

        // Amazon MQ only speaks AMQP for RabbitMQ brokers; ActiveMQ is a
        // different protocol we do not implement, so skip those.
        let mut endpoints = Vec::new();
        if engine.eq_ignore_ascii_case("RABBITMQ") && state == "RUNNING" {
            match broker_endpoints(credentials, &region, &id, timeout).await {
                Ok(found) => endpoints = found,
                Err(err) => {
                    meta.insert("endpoint_error".into(), err.message);
                }
            }
        }

        let mut resource = aws::resource(
            format!("arn:aws:mq:{region}:broker:{id}"),
            name,
            "aws_mq",
            crate::config::BackendKind::RabbitMq,
            region.clone(),
            endpoints,
            true,
            Some("".into()),
            meta,
        );
        // The AMQP user/password is issued per broker and must be supplied by
        // the operator; AWS does not return it through the list API.
        resource.needs_password = true;
        resource.username = None;
        out.push(resource);
    }

    Ok(out)
}

async fn broker_endpoints(
    credentials: &CloudCredentials,
    region: &str,
    broker_id: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<Endpoint>> {
    let url = format!("https://mq.{region}.amazonaws.com/v1/brokers/{broker_id}");
    let body = call(credentials, region, &url, timeout).await?;

    let mut out = Vec::new();
    if let Some(instances) = body.get("brokerInstances").and_then(|v| v.as_array()) {
        for instance in instances {
            if let Some(endpoints) = instance.get("endpoints").and_then(|v| v.as_array()) {
                for endpoint in endpoints {
                    let Some(text) = endpoint.as_str() else { continue };
                    // Prefer AMQP over the other protocols listed.
                    let Some(port) = amqp_port(text) else { continue };
                    if let Some(host) = host_of(text) {
                        out.push(Endpoint::new(host, port));
                    }
                }
            }
        }
    }
    Ok(out)
}

/// Amazon MQ lists endpoints like `amqps://b-1.mq.us-east-1.amazonaws.com:5671`.
fn host_of(url: &str) -> Option<String> {
    let rest = url.split_once("://").map(|(_, rest)| rest).unwrap_or(url);
    let authority = rest.split('/').next()?;
    let host = authority.rsplit_once(':').map(|(host, _)| host).unwrap_or(authority);
    if host.is_empty() {
        None
    } else {
        Some(host.to_string())
    }
}

fn amqp_port(url: &str) -> Option<u16> {
    if !url.starts_with("amqp") {
        return None;
    }
    url.rsplit_once(':')
        .and_then(|(_, port)| port.trim_end_matches('/').parse::<u16>().ok())
}

async fn call(
    credentials: &CloudCredentials,
    region: &str,
    url: &str,
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let signed = sigv4::sign(sigv4::SignRequest {
        method: "GET",
        url,
        region,
        service: SERVICE,
        access_key_id: &credentials.access_key_id,
        secret_access_key: &credentials.secret_access_key,
        session_token: credentials.session_token.as_deref(),
        body: b"",
        headers: vec![],
        timestamp: chrono::Utc::now().timestamp(),
    })?;

    let mut request = crate::cloud::http::Request::get(url);
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
    fn extracts_amqp_endpoints() {
        assert_eq!(
            host_of("amqps://b-1.mq.eu-west-1.amazonaws.com:5671").as_deref(),
            Some("b-1.mq.eu-west-1.amazonaws.com")
        );
        assert_eq!(amqp_port("amqps://b-1.mq.eu-west-1.amazonaws.com:5671"), Some(5671));
    }

    #[test]
    fn ignores_non_amqp_protocols() {
        assert_eq!(amqp_port("https://b-1.mq.eu-west-1.amazonaws.com:443"), None);
        assert_eq!(amqp_port("ssl://b-1.mq.eu-west-1.amazonaws.com:61617"), None);
    }
}
