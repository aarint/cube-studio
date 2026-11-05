//! CloudAMQP discovery.
//!
//! CloudAMQP exposes one endpoint per instance; the AMQP URL it returns already
//! contains the credentials, so this module splits it into parts rather than
//! making the user copy them out of a dashboard.

use std::collections::BTreeMap;

use crate::cloud::{CloudCredentials, CloudProvider, DiscoveredResource};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

const BASE: &str = "https://customer.cloudamqp.com/api";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let key = credentials
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("CloudAMQP API key is required"))?;

    let request = crate::cloud::http::Request::get(format!("{BASE}/instances"))
        .basic("", &key);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    Ok(parse_instances(&response.json()?))
}

/// A CloudAMQP instance row, as far as we care about it.
fn parse_instances(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let Some(items) = json.as_array() else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for item in items {
        let id = item
            .get("id")
            .map(|value| value.to_string())
            .unwrap_or_default();
        let name = item
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("cloudamqp")
            .to_string();

        // `url` is the AMQPS URL including credentials; `host`/`vhost` are the
        // explicit fields. Prefer the URL because it is always present.
        let url = item
            .get("url")
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let Some(parsed) = parse_amqp_url(url) else {
            continue;
        };

        let mut meta = BTreeMap::new();
        if let Some(vhost) = item.get("vhost").and_then(|value| value.as_str()) {
            meta.insert("vhost".into(), vhost.to_string());
        }
        if let Some(plan) = item.get("plan").and_then(|value| value.as_str()) {
            meta.insert("plan".into(), plan.to_string());
        }
        if let Some(region) = item.get("region").and_then(|value| value.as_str()) {
            meta.insert("region".into(), region.to_string());
        }
        if let Some(nodes) = item.get("nodes").and_then(|value| value.as_array()) {
            meta.insert("nodes".into(), nodes.len().to_string());
        }

        out.push(DiscoveredResource {
            id,
            name,
            provider: CloudProvider::CloudAmqp,
            service: "cloudamqp".into(),
            backend: BackendKind::RabbitMq,
            region: item
                .get("region")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            endpoints: vec![Endpoint::new(parsed.host, parsed.port)],
            tls: parsed.tls,
            username: Some(parsed.username),
            database: None,
            meta,
            // The password travels in the URL, so the caller does not have to
            // type it: it is attached when the profile is built.
            needs_password: !parsed.password.is_empty(),
        });
    }
    out
}

struct AmqpUrl {
    tls: bool,
    host: String,
    port: u16,
    username: String,
    password: String,
}

/// `amqps://user:pass@host/vhost` -> parts.
fn parse_amqp_url(url: &str) -> Option<AmqpUrl> {
    let (scheme, rest) = url.split_once("://")?;
    let tls = scheme.eq_ignore_ascii_case("amqps");
    let (credentials, host_part) = match rest.rsplit_once('@') {
        Some((credentials, host)) => (Some(credentials), host),
        None => (None, rest),
    };
    let authority = host_part.split('/').next().unwrap_or(host_part);
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            let port = port.parse::<u16>().ok()?;
            (host.to_string(), port)
        }
        None => (authority.to_string(), if tls { 5671 } else { 5672 }),
    };
    if host.is_empty() {
        return None;
    }
    let (username, password) = match credentials.and_then(|c| c.split_once(':')) {
        Some((user, pass)) => (user.to_string(), pass.to_string()),
        None => (credentials.unwrap_or_default().to_string(), String::new()),
    };
    Some(AmqpUrl {
        tls,
        host,
        port,
        username,
        password,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_tls_amqp_url() {
        let parsed = parse_amqp_url("amqps://user:pw@bunny.rmq.cloudamqp.com/vhost").unwrap();
        assert!(parsed.tls);
        assert_eq!(parsed.host, "bunny.rmq.cloudamqp.com");
        assert_eq!(parsed.port, 5671);
        assert_eq!(parsed.username, "user");
        assert_eq!(parsed.password, "pw");
    }

    #[test]
    fn defaults_the_port_by_scheme() {
        let parsed = parse_amqp_url("amqp://h/v").unwrap();
        assert_eq!(parsed.port, 5672);
        assert!(!parsed.tls);
    }

    #[test]
    fn rejects_non_urls() {
        assert!(parse_amqp_url("not-a-url").is_none());
        assert!(parse_amqp_url("amqp://").is_none());
    }

    #[test]
    fn builds_resources_and_flags_the_password() {
        let json = serde_json::json!([{
            "id": 42,
            "name": "orders",
            "url": "amqps://u:p@bunny.rmq.cloudamqp.com/orders",
            "vhost": "orders",
            "plan": "bunny",
            "region": "amazon-web-services::eu-west-1",
            "nodes": [{}, {}]
        }]);
        let found = parse_instances(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].username.as_deref(), Some("u"));
        assert!(found[0].needs_password, "the URL carries a password");
        assert_eq!(found[0].meta.get("nodes").map(String::as_str), Some("2"));
    }
}
