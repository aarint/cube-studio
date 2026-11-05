//! Redis Cloud (Redis Ltd.) discovery.
//!
//! The API key and secret are exchanged for a short-lived session token, then
//! subscriptions and their databases are listed.

use std::collections::BTreeMap;

use crate::cloud::{CloudCredentials, CloudProvider, DiscoveredResource};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

const BASE: &str = "https://api.redislabs.com/v1";

pub async fn discover(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    let key = credentials
        .api_key
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Redis Cloud API key is required"))?;
    let secret = credentials
        .api_secret
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Redis Cloud API secret is required"))?;

    let token = session_token(&key, &secret, timeout).await?;
    let mut out = Vec::new();

    let subscriptions =
        get(&format!("{BASE}/subscriptions?limit=100"), &key, &token, timeout).await?;
    let Some(list) = subscriptions
        .get("subscriptions")
        .and_then(|value| value.as_array())
    else {
        return Ok(out);
    };

    for subscription in list {
        let Some(sub_id) = as_id(subscription.get("id")) else {
            continue;
        };
        let sub_name = subscription
            .get("name")
            .and_then(|value| value.as_str())
            .unwrap_or("redis-cloud");

        // Each subscription holds one or more databases.
        let databases = match get(
            &format!("{BASE}/subscriptions/{sub_id}/databases?limit=100"),
            &key,
            &token,
            timeout,
        )
        .await
        {
            Ok(json) => json,
            Err(_) => continue,
        };
        let Some(dbs) = databases.get("databases").and_then(|value| value.as_array()) else {
            continue;
        };

        for database in dbs {
            // The list endpoint returns only ids; the detail endpoint has the
            // public endpoint, which is what we need.
            let Some(db_id) = as_id(database.get("databaseId")) else {
                continue;
            };
            let Ok(detail) = get(
                &format!("{BASE}/subscriptions/{sub_id}/databases/{db_id}"),
                &key,
                &token,
                timeout,
            )
            .await
            else {
                continue;
            };
            if let Some(resource) = parse_database(&detail, sub_name, &sub_id) {
                out.push(resource);
            }
        }
    }

    Ok(out)
}

fn parse_database(
    json: &serde_json::Value,
    subscription_name: &str,
    subscription_id: &str,
) -> Option<DiscoveredResource> {
    let name = json
        .get("name")
        .and_then(|value| value.as_str())
        .unwrap_or("redis-cloud-db")
        .to_string();
    let host = json
        .get("publicEndpoint")
        .and_then(|value| value.as_str())
        .or_else(|| json.get("privateEndpoint").and_then(|value| value.as_str()))
        .unwrap_or_default()
        .to_string();
    if host.is_empty() {
        return None;
    }
    let tls_port = json
        .get("sslPort")
        .and_then(|value| value.as_u64())
        .or_else(|| json.get("port").and_then(|value| value.as_u64()))
        .unwrap_or(0) as u16;
    let port = if tls_port > 0 { tls_port } else { 17000 };

    let mut meta = BTreeMap::new();
    meta.insert("subscription".into(), subscription_name.to_string());
    meta.insert("subscription_id".into(), subscription_id.to_string());
    for field in ["memoryLimitInGb", "replication", "dataPersistence"] {
        if let Some(value) = json.get(field) {
            let text = value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string());
            if text != "null" {
                meta.insert(field.to_string(), text);
            }
        }
    }
    // Redis Cloud enables TLS on `sslPort`; a database without one is plaintext.
    let tls = json.get("sslPort").and_then(|v| v.as_u64()).unwrap_or(0) > 0
        || json
            .get("tls")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
    meta.insert("tls".into(), tls.to_string());

    let id = as_id(json.get("databaseId")).unwrap_or_else(|| name.clone());

    Some(DiscoveredResource {
        id: format!("{subscription_id}/{id}"),
        name: format!("{name} ({subscription_name})"),
        provider: CloudProvider::RedisCloud,
        service: "redis_cloud".into(),
        backend: BackendKind::Redis,
        region: json
            .get("regionName")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        endpoints: vec![Endpoint::new(host, port)],
        tls,
        username: None,
        database: None,
        meta,
        needs_password: true,
    })
}

fn as_id(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

async fn session_token(
    key: &str,
    secret: &str,
    timeout: std::time::Duration,
) -> AppResult<String> {
    let request = crate::cloud::http::Request::post(format!("{BASE}/sessions"), Vec::new())
        .basic(key, secret);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    let json = response.json()?;
    json.get("apiKey")
        .and_then(|value| value.as_str())
        .or_else(|| json.get("token").and_then(|value| value.as_str()))
        .map(str::to_string)
        .ok_or_else(|| AppError::unauthorized("Redis Cloud returned no session token"))
}

async fn get(
    url: &str,
    key: &str,
    token: &str,
    timeout: std::time::Duration,
) -> AppResult<serde_json::Value> {
    let request = crate::cloud::http::Request::get(url)
        .header("x-api-key", key)
        .header("x-api-secret-key", token);
    let response = crate::cloud::http::execute(&request, timeout)
        .await?
        .into_result()?;
    response.json()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_database_with_a_tls_port() {
        let json = serde_json::json!({
            "databaseId": 123,
            "name": "cart",
            "publicEndpoint": "redis-1234.c1.eu-west-1-1.ec2.redns.redis-cloud.com",
            "sslPort": 17345,
            "memoryLimitInGb": 1.5,
            "regionName": "eu-west-1"
        });
        let found = parse_database(&json, "prod", "99").unwrap();
        assert_eq!(found.endpoints[0].port, 17345);
        assert!(found.tls);
        assert_eq!(found.id, "99/123");
        assert!(found.name.contains("prod"));
    }

    #[test]
    fn falls_back_to_the_private_endpoint() {
        let json = serde_json::json!({
            "databaseId": 1,
            "name": "internal",
            "privateEndpoint": "10.0.0.5",
            "port": 17000
        });
        let found = parse_database(&json, "s", "1").unwrap();
        assert_eq!(found.endpoints[0].host, "10.0.0.5");
    }

    #[test]
    fn skips_databases_without_any_endpoint() {
        let json = serde_json::json!({ "databaseId": 1, "name": "pending" });
        assert!(parse_database(&json, "s", "1").is_none());
    }
}
