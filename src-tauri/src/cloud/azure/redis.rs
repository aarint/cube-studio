//! Azure Cache for Redis discovery.
//!
//! One ARM list call returns every cache in the subscription, including its
//! hostname, TLS port, non-TLS port and shard topology.

use std::collections::BTreeMap;

use crate::cloud::azure;
use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::config::Endpoint;
use crate::error::AppResult;

const API_VERSION: &str = "2024-03-01";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let token = azure::access_token(credentials, timeout).await?;
    let subscription = azure::subscription(credentials)?;

    let url = format!(
        "https://management.azure.com/subscriptions/{subscription}/providers/Microsoft.Cache/redis?api-version={API_VERSION}"
    );
    let request = crate::cloud::http::Request::get(&url).bearer(&token);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    let json = response.json()?;

    Ok(parse_caches(&json))
}

fn parse_caches(json: &serde_json::Value) -> Vec<DiscoveredResource> {
    let mut out = Vec::new();
    let Some(items) = json.get("value").and_then(|value| value.as_array()) else {
        return out;
    };

    for item in items {
        let id = item
            .get("id")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        let name = item
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("azure-redis")
            .to_string();
        let location = item
            .get("location")
            .and_then(|value| value.as_str())
            .map(str::to_string);

        let props = item.get("properties").unwrap_or(&serde_json::Value::Null);
        let host = props
            .get("hostName")
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string();
        if host.is_empty() {
            continue;
        }
        // 6380 is the TLS port on every Azure Cache tier; 6379 is plaintext and
        // disabled on most newer instances.
        let tls_port = props
            .get("sslPort")
            .and_then(|value| value.as_u64())
            .unwrap_or(6380) as u16;
        let plain_port = props
            .get("port")
            .and_then(|value| value.as_u64())
            .unwrap_or(6379) as u16;
        let non_ssl_enabled = props
            .get("enableNonSslPort")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);

        let endpoints = if non_ssl_enabled {
            vec![Endpoint::new(host.clone(), plain_port), Endpoint::new(host, tls_port)]
        } else {
            vec![Endpoint::new(host, tls_port)]
        };

        let mut meta = BTreeMap::new();
        if let Some(sku) = props
            .pointer("/sku/name")
            .and_then(|value| value.as_str())
        {
            meta.insert("sku".into(), sku.to_string());
        }
        if let Some(capacity) = props.pointer("/sku/capacity").and_then(|v| v.as_u64()) {
            meta.insert("capacity".into(), capacity.to_string());
        }
        if let Some(shards) = props
            .pointer("/redisConfiguration/maxmemory-policy")
            .and_then(|value| value.as_str())
        {
            meta.insert("maxmemory_policy".into(), shards.to_string());
        }
        if let Some(version) = props.get("redisVersion").and_then(|value| value.as_str()) {
            meta.insert("redis_version".into(), version.to_string());
        }
        if let Some(provisioning) = props
            .get("provisioningState")
            .and_then(|value| value.as_str())
        {
            meta.insert("state".into(), provisioning.to_string());
        }

        out.push(azure::resource(
            id,
            name,
            "azure_cache_redis",
            crate::config::BackendKind::Redis,
            location,
            endpoints,
            // Azure Cache always supports TLS on 6380, so prefer it.
            true,
            None,
            meta,
        ));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_basic_cache() {
        let json = serde_json::json!({
            "value": [{
                "id": "/subscriptions/s/resourceGroups/rg/providers/Microsoft.Cache/redis/orders",
                "name": "orders",
                "location": "westeurope",
                "properties": {
                    "hostName": "orders.redis.cache.windows.net",
                    "sslPort": 6380,
                    "port": 6379,
                    "enableNonSslPort": false,
                    "redisVersion": "6.0",
                    "provisioningState": "Succeeded",
                    "sku": { "name": "Standard", "capacity": 1 }
                }
            }]
        });
        let found = parse_caches(&json);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "orders");
        assert_eq!(found[0].endpoints.len(), 1, "non-TLS port is disabled");
        assert_eq!(found[0].endpoints[0].port, 6380);
        assert_eq!(found[0].meta.get("sku").map(String::as_str), Some("Standard"));
    }

    #[test]
    fn includes_the_plaintext_port_when_enabled() {
        let json = serde_json::json!({
            "value": [{
                "name": "legacy",
                "properties": {
                    "hostName": "legacy.redis.cache.windows.net",
                    "sslPort": 6380,
                    "port": 6379,
                    "enableNonSslPort": true
                }
            }]
        });
        let found = parse_caches(&json);
        assert_eq!(found[0].endpoints.len(), 2);
    }

    #[test]
    fn skips_entries_without_a_hostname() {
        let json = serde_json::json!({ "value": [{ "name": "pending", "properties": {} }] });
        assert!(parse_caches(&json).is_empty());
    }
}
