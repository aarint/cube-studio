//! AWS control-plane discovery (ElastiCache, MSK, Amazon MQ).
//!
//! Uses the AWS Query/JSON REST APIs directly with SigV4 request signing, rather
//! than pulling in the AWS SDK: three read-only calls do not justify ~200 crates
//! in a desktop build.

pub mod elasticache;
pub mod msk;
pub mod mq;
mod sigv4;

use crate::cloud::{CloudCredentials, DiscoveredResource};
use crate::error::{AppError, AppResult};

pub(crate) const DEFAULT_REGION: &str = "us-east-1";

/// Region to query, falling back to the standard default.
pub(crate) fn region(credentials: &CloudCredentials) -> String {
    credentials
        .region
        .clone()
        .filter(|region| !region.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_REGION.to_string())
}

/// Reject obviously incomplete credentials before making a network call, so the
/// user gets a precise message instead of a 403 from AWS.
pub(crate) fn require_keys(credentials: &CloudCredentials) -> AppResult<()> {
    if credentials.access_key_id.trim().is_empty() {
        return Err(AppError::invalid("AWS access key id is required"));
    }
    if credentials.secret_access_key.trim().is_empty() {
        return Err(AppError::invalid("AWS secret access key is required"));
    }
    Ok(())
}

/// Describe an AWS failure with the service's own message.
pub(crate) fn aws_error(body: &str, status: u16) -> AppError {
    // Both the Query API (`<Message>`) and the JSON APIs (`"message"`) appear
    // across the services we call.
    let message = extract_tag(body, "Message")
        .or_else(|| extract_tag(body, "message"))
        .or_else(|| {
            serde_json::from_str::<serde_json::Value>(body)
                .ok()
                .and_then(|value| {
                    value
                        .get("message")
                        .or_else(|| value.get("Message"))
                        .and_then(|v| v.as_str())
                        .map(str::to_string)
                })
        })
        .unwrap_or_else(|| body.chars().take(300).collect());

    let kind = match status {
        401 | 403 => crate::error::ErrorKind::Unauthorized,
        400 => crate::error::ErrorKind::InvalidInput,
        429 | 503 => crate::error::ErrorKind::Unavailable,
        _ => crate::error::ErrorKind::ServerError,
    };
    AppError::new(kind, message).with_detail(format!("AWS HTTP {status}"))
}

fn extract_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].to_string())
}

/// Build the discovery result shared by all AWS services.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resource(
    id: String,
    name: String,
    service: &str,
    backend: crate::config::BackendKind,
    region: String,
    endpoints: Vec<crate::config::Endpoint>,
    tls: bool,
    username: Option<String>,
    meta: std::collections::BTreeMap<String, String>,
) -> DiscoveredResource {
    DiscoveredResource {
        id,
        name,
        provider: crate::cloud::CloudProvider::Aws,
        service: service.to_string(),
        backend,
        region: Some(region),
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
    fn extracts_a_tag_value() {
        assert_eq!(
            extract_tag("<a><Message>bad key</Message></a>", "Message").as_deref(),
            Some("bad key")
        );
    }

    #[test]
    fn classifies_aws_errors() {
        let err = aws_error(r#"{"message":"The security token included in the request is invalid."}"#, 403);
        assert_eq!(err.kind, crate::error::ErrorKind::Unauthorized);
        assert!(err.message.contains("security token"));
    }

    #[test]
    fn requires_both_keys() {
        let mut creds = CloudCredentials::default();
        assert!(require_keys(&creds).is_err());
        creds.access_key_id = "AKIA".into();
        assert!(require_keys(&creds).is_err());
        creds.secret_access_key = "secret".into();
        assert!(require_keys(&creds).is_ok());
    }
}
