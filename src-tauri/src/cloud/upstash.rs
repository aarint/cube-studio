//! Upstash discovery (Redis and Kafka).
//!
//! Upstash authenticates with a single account API key in the `Authorization`
//! header, and returns every database across all regions in one call.

use std::collections::BTreeMap;

use crate::cloud::{CloudCredentials, CloudProvider, DiscoveredResource};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

const BASE: &str = "https://api.upstash.com/v2";

pub async fn discover_redis(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let key = api_key(credentials)?;
    let json = get(&format!("{BASE}/redis/databases"), &key, timeout).await?;
    Ok(parse_redis(&json))
}

pub async fn discover_kafka(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    // Upstash Kafka uses an email + API key pair for the cluster list.
    let key = api_key(credentials)?;
    let email = credentials
        .api_secret
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::invalid("Upstash Kafka requires the account email as the API secret field")
        })?;

    let request = crate::cloud::http::Request::get(format!("{BASE}/kafka/cluster"))
        .basic(&email, &key);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    Ok(parse_kafka(&response.json()?))
}

fn api_key(credentials: &CloudCredentials) -> AppResult<String> {
    credentials
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Upstash management API key is required"))
}

fn parse_redis(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let items = json
        .as_array()
        .cloned()
        .or_else(|| json.get("databases").and_then(|v| v.as_array()).cloned())
        .unwrap_or_default();

    let mut out = Vec::new();
    for item in items {
        let id = item
            .get("database_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let name = item
            .get("database_name")
            .and_then(|value| value.as_str())
            .unwrap_or("upstash-redis")
            .to_string();
        let endpoint = item
            .get("endpoint")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if endpoint.is_empty() {
            continue;
        }
        let port = item
            .get("port")
            .and_then(|value| value.as_u64())
            .unwrap_or(6379) as u16;

        let mut meta = BTreeMap::new();
        for field in ["region", "type", "state", "db_max_clients"] {
            if let Some(value) = item.get(field) {
                let text = value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string());
                if text != "null" && !text.is_empty() {
                    meta.insert(field.to_string(), text);
                }
            }
        }
        // Upstash requires TLS unless the database is explicitly marked
        // `tls: false` (only possible on custom instances).
        let tls = item
            .get("tls")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        meta.insert("tls".into(), tls.to_string());

        out.push(DiscoveredResource {
            id: if id.is_empty() { name.clone() } else { id },
            name,
            provider: CloudProvider::Upstash,
            service: "upstash_redis".into(),
            backend: BackendKind::Redis,
            region: item
                .get("region")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            endpoints: vec![Endpoint::new(endpoint, port)],
            tls,
            username: Some("default".into()),
            database: None,
            meta,
            needs_password: true,
        });
    }
    out
}

fn parse_kafka(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let items = json
        .get("clusters")
        .and_then(|v| v.as_array())
        .cloned()
        .or_else(|| json.as_array().cloned())
        .unwrap_or_default();

    let mut out = Vec::new();
    for item in items {
        let cluster_id = item
            .get("cluster_id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let name = item
            .get("cluster_name")
            .and_then(|value| value.as_str())
            .unwrap_or("upstash-kafka")
            .to_string();
        let endpoint = item
            .get("endpoint")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let Some(endpoints) = parse_bootstrap(&endpoint) else {
            continue;
        };

        let mut meta = BTreeMap::new();
        if let Some(region) = item.get("region").and_then(|value| value.as_str()) {
            meta.insert("region".into(), region.to_string());
        }
        if let Some(topic) = item.get("topic").and_then(|value| value.as_str()) {
            meta.insert("topic".into(), topic.to_string());
        }

        out.push(DiscoveredResource {
            id: if cluster_id.is_empty() {
                name.clone()
            } else {
                cluster_id
            },
            name,
            provider: CloudProvider::Upstash,
            service: "upstash_kafka".into(),
            backend: BackendKind::Kafka,
            region: item
                .get("region")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            endpoints,
            tls: true,
            username: None,
            database: None,
            meta,
            needs_password: true,
        });
    }
    out
}

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
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let request = crate::cloud::http::Request::get(url).header("Authorization", format!("Basic {key}"));
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    response.json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_redis_database() {
        let json = serde_json::json!([{
            "database_id": "db-1",
            "database_name": "cart",
            "endpoint": "cart-1234.upstash.io",
            "port": 6379,
            "region": "eu-west-1",
            "type": "regional",
            "state": "active",
            "tls": true
        }]);
        let found = parse_redis(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "cart");
        assert!(found[0].tls);
        assert_eq!(found[0].username.as_deref(), Some("default"));
        assert_eq!(found[0].meta.get("region").map(String::as_str), Some("eu-west-1"));
    }

    #[test]
    fn skips_databases_without_an_endpoint() {
        let json = serde_json::json!([{ "database_id": "x", "database_name": "y" }]);
        assert!(parse_redis(&json).is_empty());
    }

    #[test]
    fn parses_a_kafka_cluster() {
        let json = serde_json::json!({
            "clusters": [{
                "cluster_id": "k-1",
                "cluster_name": "events",
                "endpoint": "k-1.upstash.io:9092",
                "region": "eu-west-1",
                "topic": "orders"
            }]
        });
        let found = parse_kafka(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].endpoints[0].port, 9092);
        assert!(found[0].tls);
    }
}
