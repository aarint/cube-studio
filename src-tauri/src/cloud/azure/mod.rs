//! Azure discovery (Azure Cache for Redis).
//!
//! Authentication is an OAuth2 client-credentials exchange against the tenant's
//! Entra ID endpoint; the resulting bearer token authorises the ARM (Azure
//! Resource Manager) call that lists caches.

pub mod redis;

use crate::cloud::{CloudCredentials, CloudProvider};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

const ARM_SCOPE: &str = "https://management.azure.com/.default";

/// Fetch an ARM bearer token using the client-credentials grant.
pub(crate) async fn access_token(
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<String> {
    let tenant = credentials
        .tenant_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Azure tenant id is required"))?;
    let client_id = credentials
        .client_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Azure client id is required"))?;
    let client_secret = credentials
        .client_secret
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Azure client secret is required"))?;

    let url = format!("https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token");
    let body = format!(
        "grant_type=client_credentials&client_id={}&client_secret={}&scope={}",
        url_encode(&client_id),
        url_encode(&client_secret),
        url_encode(ARM_SCOPE)
    );

    let request = crate::cloud::http::Request::post(&url, body.into_bytes())
        .header("Content-Type", "application/x-www-form-urlencoded");
    let response = crate::cloud::http::execute(&request, timeout).await?;

    if !response.is_success() {
        let text = String::from_utf8_lossy(&response.body).to_string();
        let message = serde_json::from_str::<serde_json::Value>(&text)
            .ok()
            .and_then(|value| {
                value
                    .get("error_description")
                    .or_else(|| value.get("error"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string)
            })
            .unwrap_or(text);
        return Err(AppError::unauthorized(format!(
            "Azure rejected the credentials: {message}"
        )));
    }

    let json = response.json()?;
    json.get("access_token")
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| AppError::unauthorized("Azure returned no access token"))
}

pub(crate) fn subscription(credentials: &CloudCredentials) -> AppResult<String> {
    credentials
        .subscription_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Azure subscription id is required"))
}

fn url_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Shared resource builder for ARM results.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resource(
    id: String,
    name: String,
    service: &str,
    backend: BackendKind,
    location: Option<String>,
    endpoints: Vec<Endpoint>,
    tls: bool,
    username: Option<String>,
    meta: std::collections::BTreeMap<String, String>,
) -> crate::cloud::DiscoveredResource {
    crate::cloud::DiscoveredResource {
        id,
        name,
        provider: CloudProvider::Azure,
        service: service.to_string(),
        backend,
        region: location,
        endpoints,
        tls,
        username,
        database: None,
        meta,
        needs_password: true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encodes_form_values() {
        assert_eq!(url_encode("a b/c"), "a%20b%2Fc");
        assert_eq!(url_encode("plain-1.2_~"), "plain-1.2_~");
    }

    #[tokio::test]
    async fn requires_every_credential() {
        // Validation runs before any request, so nothing is sent.
        let mut creds = CloudCredentials::default();
        let err = access_token(&creds, std::time::Duration::from_millis(1))
            .await
            .unwrap_err();
        assert!(err.message.contains("tenant"));

        creds.tenant_id = Some("t".into());
        creds.client_id = Some("c".into());
        let err = access_token(&creds, std::time::Duration::from_millis(1))
            .await
            .unwrap_err();
        assert!(err.message.contains("client secret"));
    }

    #[test]
    fn requires_a_subscription() {
        assert!(subscription(&CloudCredentials::default()).is_err());
    }
}
