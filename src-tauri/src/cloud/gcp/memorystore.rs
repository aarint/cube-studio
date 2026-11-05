//! Memorystore discovery.
//!
//! Covers both classic Memorystore for Redis (`projects/*/locations/*/instances`)
//! and Memorystore for Redis Cluster, whose endpoint lives in a different field.

use std::collections::BTreeMap;

use crate::cloud::gcp;
use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::config::Endpoint;
use crate::error::AppResult;

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let (token, project) = gcp::credentials(credentials)?;
    let mut out = list_instances(&token, &project, timeout).await?;
    // Redis Cluster is a separate API. A project that uses only classic
    // instances gets a 403/404 here, which must not hide the first result.
    if let Ok(mut clusters) = list_clusters(&token, &project, timeout).await {
        out.append(&mut clusters);
    }
    Ok(out)
}

async fn list_instances(
    token: &str,
    project: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let url = format!(
        "https://redis.googleapis.com/v1/projects/{project}/locations/-/instances"
    );
    let json = get_json(token, &url, timeout).await?;
    Ok(parse_instances(&json))
}

async fn list_clusters(
    token: &str,
    project: &str,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let url = format!(
        "https://redis.googleapis.com/v1/projects/{project}/locations/-/clusters"
    );
    let json = get_json(token, &url, timeout).await?;
    Ok(parse_clusters(&json))
}

async fn get_json(
    token: &str,
    url: &str,
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let request = crate::cloud::http::Request::get(url).bearer(token);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    response.json()
}

fn parse_instances(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let mut out = Vec::new();
    let Some(items) = json.get("instances").and_then(|value| value.as_array()) else {
        return out;
    };

    for item in items {
        let name = item
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let short_name = name.rsplit('/').next().unwrap_or(&name).to_string();
        let host = item
            .get("host")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let port = item.get("port").and_then(|value| value.as_u64()).unwrap_or(6379) as u16;
        if host.is_empty() {
            continue;
        }

        let region = name
            .split('/')
            .collect::<Vec<_>>()
            .windows(2)
            .find(|pair| pair[0] == "locations")
            .map(|pair| pair[1].to_string());

        let mut meta = BTreeMap::new();
        for key in ["tier", "redisVersion", "state", "memorySizeGb", "locationId"] {
            if let Some(value) = item.get(key) {
                let text = value
                    .as_str()
                    .map(str::to_string)
                    .unwrap_or_else(|| value.to_string());
                if !text.is_empty() && text != "null" {
                    meta.insert(key.to_string(), text);
                }
            }
        }
        // `transitEncryptionMode` is DISABLED or SERVER_AUTHENTICATION.
        let tls = item
            .get("transitEncryptionMode")
            .and_then(|value| value.as_str())
            .map(|mode| !mode.eq_ignore_ascii_case("DISABLED"))
            .unwrap_or(false);
        meta.insert("transit_encryption".into(), tls.to_string());

        out.push(gcp::resource(
            name,
            short_name,
            "gcp_memorystore",
            crate::config::BackendKind::Redis,
            region,
            vec![Endpoint::new(host, port)],
            tls,
            None,
            meta,
        ));
    }

    out
}

fn parse_clusters(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let mut out = Vec::new();
    let Some(items) = json.get("clusters").and_then(|value| value.as_array()) else {
        return out;
    };

    for item in items {
        let name = item
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if name.is_empty() {
            continue;
        }
        let short_name = name.rsplit('/').next().unwrap_or(&name).to_string();

        // Cluster endpoints live under `discoveryEndpoints`.
        let mut endpoints = Vec::new();
        if let Some(list) = item.get("discoveryEndpoints").and_then(|v| v.as_array()) {
            for entry in list {
                let host = entry.get("address").and_then(|v| v.as_str()).unwrap_or_default();
                let port = entry.get("port").and_then(|v| v.as_u64()).unwrap_or(6379) as u16;
                if !host.is_empty() {
                    endpoints.push(Endpoint::new(host, port));
                }
            }
        }
        if endpoints.is_empty() {
            continue;
        }

        let region = name
            .split('/')
            .collect::<Vec<_>>()
            .windows(2)
            .find(|pair| pair[0] == "locations")
            .map(|pair| pair[1].to_string());

        let mut meta = BTreeMap::new();
        meta.insert("kind".into(), "redis-cluster".into());
        if let Some(shards) = item
            .get("shardCount")
            .and_then(|value| value.as_u64())
        {
            meta.insert("shards".into(), shards.to_string());
        }
        if let Some(version) = item.get("redisVersion").and_then(|v| v.as_str()) {
            meta.insert("redis_version".into(), version.to_string());
        }

        let mut resource = gcp::resource(
            name,
            short_name,
            "gcp_memorystore",
            crate::config::BackendKind::Redis,
            region,
            endpoints,
            true,
            None,
            meta,
        );
        // A cluster endpoint is a configuration endpoint: the driver performs
        // the slot discovery, so report it as such.
        resource.meta.insert("cluster".into(), "true".into());
        out.push(resource);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_classic_instance() {
        let json = serde_json::json!({
            "instances": [{
                "name": "projects/shop/locations/europe-west1/instances/orders",
                "host": "10.0.0.4",
                "port": 6379,
                "tier": "STANDARD_HA",
                "redisVersion": "7.0",
                "state": "READY",
                "transitEncryptionMode": "SERVER_AUTHENTICATION"
            }]
        });
        let found = parse_instances(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "orders");
        assert_eq!(found[0].region.as_deref(), Some("europe-west1"));
        assert!(found[0].tls);
        assert_eq!(found[0].meta.get("tier").map(String::as_str), Some("STANDARD_HA"));
    }

    #[test]
    fn plaintext_instances_report_no_tls() {
        let json = serde_json::json!({
            "instances": [{ "name": "p/l/instances/a", "host": "h", "port": 6379, "transitEncryptionMode": "DISABLED" }]
        });
        assert!(!parse_instances(&json)[0].tls);
    }

    #[test]
    fn parses_a_cluster_with_discovery_endpoints() {
        let json = serde_json::json!({
            "clusters": [{
                "name": "projects/shop/locations/us-central1/clusters/c1",
                "shardCount": 3,
                "discoveryEndpoints": [
                    { "address": "10.1.0.2", "port": 6379 },
                    { "address": "10.1.0.3", "port": 6379 }
                ]
            }]
        });
        let found = parse_clusters(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].endpoints.len(), 2);
        assert_eq!(found[0].meta.get("shards").map(String::as_str), Some("3"));
        assert_eq!(found[0].meta.get("cluster").map(String::as_str), Some("true"));
    }

    #[test]
    fn skips_instances_without_addresses() {
        let json = serde_json::json!({
            "clusters": [{ "name": "p/l/clusters/c", "discoveryEndpoints": [] }]
        });
        assert!(parse_clusters(&json).is_empty());
    }
}
