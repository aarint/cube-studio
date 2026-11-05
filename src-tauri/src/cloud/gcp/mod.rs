//! Google Cloud discovery (Memorystore for Redis / Redis Cluster).
//!
//! Authenticates with a bearer token — the operator obtains one with
//! `gcloud auth print-access-token`, which keeps us out of the service-account
//! key-file business entirely.

pub mod memorystore;

use crate::cloud::{CloudCredentials, CloudProvider};
use crate::config::{BackendKind, Endpoint};
use crate::error::{AppError, AppResult};

/// Token and project, both required for any Memorystore call.
pub(crate) fn credentials(credentials: &CloudCredentials) -> AppResult<(String, String)> {
    let token = credentials
        .access_token
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::invalid(
                "Google Cloud access token is required (run `gcloud auth print-access-token`)",
            )
        })?;
    let project = credentials
        .project_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::invalid("Google Cloud project id is required"))?;
    Ok((token, project))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn resource(
    id: String,
    name: String,
    service: &str,
    backend: BackendKind,
    region: Option<String>,
    endpoints: Vec<Endpoint>,
    tls: bool,
    username: Option<String>,
    meta: std::collections::BTreeMap<String, String>,
) -> crate::cloud::DiscoveredResource {
    crate::cloud::DiscoveredResource {
        id,
        name,
        provider: CloudProvider::Gcp,
        service: service.to_string(),
        backend,
        region,
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
    fn requires_a_token_and_a_project() {
        let mut creds = CloudCredentials::default();
        assert!(credentials(&creds).unwrap_err().message.contains("access token"));
        creds.access_token = Some("ya29.token".into());
        assert!(credentials(&creds).unwrap_err().message.contains("project"));
        creds.project_id = Some("proj".into());
        let (token, project) = credentials(&creds).unwrap();
        assert_eq!(token, "ya29.token");
        assert_eq!(project, "proj");
    }
}
