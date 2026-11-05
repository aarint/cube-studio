//! Cloud-managed middleware.
//!
//! The insight that shapes this design: a managed Redis is still Redis. AWS
//! ElastiCache, Azure Cache and GCP Memorystore all speak RESP; Confluent Cloud
//! and MSK speak the Kafka protocol; CloudAMQP speaks AMQP. So instead of a
//! seventh and eighth protocol implementation, a provider's job here is:
//!
//! 1. **Authenticate** against the provider's control plane.
//! 2. **Discover** the user's managed instances and their endpoints.
//! 3. **Derive** the `ConnectionConfig` that the existing backends already
//!    understand (endpoint, TLS, credentials, database index...).
//!
//! That keeps one implementation of each wire protocol, and means a cloud
//! connection behaves exactly like a self-hosted one once it is open.
//!
//! Credentials are never persisted in plaintext and are not returned to the
//! frontend after submission.

pub mod aws;
pub mod azure;
pub mod gcp;
pub mod http;

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::{BackendKind, ConnectionConfig, Endpoint, RedisMode, TlsMode};
use crate::error::{AppError, AppResult};

/// A cloud platform whose control plane we can query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudProvider {
    Aws,
    Azure,
    Gcp,
    Confluent,
    Upstash,
    CloudAmqp,
    RedisCloud,
}

impl CloudProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            CloudProvider::Aws => "aws",
            CloudProvider::Azure => "azure",
            CloudProvider::Gcp => "gcp",
            CloudProvider::Confluent => "confluent",
            CloudProvider::Upstash => "upstash",
            CloudProvider::CloudAmqp => "cloudamqp",
            CloudProvider::RedisCloud => "redis_cloud",
        }
    }

    pub fn all() -> &'static [CloudProvider] {
        &[
            CloudProvider::Aws,
            CloudProvider::Azure,
            CloudProvider::Gcp,
            CloudProvider::Confluent,
            CloudProvider::Upstash,
            CloudProvider::CloudAmqp,
            CloudProvider::RedisCloud,
        ]
    }

    /// Documentation link shown next to the credential fields.
    pub fn credentials_url(self) -> &'static str {
        match self {
            CloudProvider::Aws => "https://console.aws.amazon.com/iam/home#/security_credentials",
            CloudProvider::Azure => {
                "https://portal.azure.com/#view/Microsoft_AAD_RegisteredApps/ApplicationsListBlade"
            }
            CloudProvider::Gcp => "https://console.cloud.google.com/apis/credentials",
            CloudProvider::Confluent => "https://confluent.cloud/settings/api-keys",
            CloudProvider::Upstash => "https://console.upstash.com/account/api",
            CloudProvider::CloudAmqp => "https://customer.cloudamqp.com/apikeys",
            CloudProvider::RedisCloud => "https://app.redislabs.com/#/user/api-keys",
        }
    }
}

/// How a provider authenticates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudAuthKind {
    /// Access key id + secret + optional session token + region (AWS SigV4).
    AwsAccessKey,
    /// Tenant id + client id + client secret (OAuth2 client credentials).
    AzureClientSecret,
    /// A bearer token, e.g. `gcloud auth print-access-token`.
    BearerToken,
    /// API key + secret used as HTTP basic auth.
    ApiKeySecret,
}

/// A managed service a provider offers, and the protocol it speaks.
#[derive(Debug, Clone, Serialize)]
pub struct CloudService {
    /// Stable id, also used to pick the discovery routine.
    pub id: &'static str,
    pub provider: CloudProvider,
    /// Wire protocol this service exposes, so the UI can reuse backend hints.
    pub backend: BackendKind,
    pub auth: CloudAuthKind,
    /// Fields the credential form must show.
    pub credential_fields: Vec<&'static str>,
}

/// Credentials for a control-plane call.
///
/// Flat rather than an enum, for the same reason `ConnectionConfig` is flat: one
/// form, only the relevant fields visible, and adding a provider does not change
/// the IPC shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct CloudCredentials {
    // AWS
    pub access_key_id: String,
    pub secret_access_key: String,
    pub session_token: Option<String>,
    pub region: Option<String>,

    // Azure
    pub tenant_id: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub subscription_id: Option<String>,

    // GCP / generic bearer
    pub access_token: Option<String>,
    pub project_id: Option<String>,

    // API key + secret providers (Confluent, Upstash, CloudAMQP, Redis Cloud)
    pub api_key: Option<String>,
    pub api_secret: Option<String>,
}

impl CloudCredentials {
    /// Remove secrets so the struct can be logged.
    pub fn redacted(&self) -> Self {
        let mask = |value: &Option<String>| {
            value
                .as_ref()
                .filter(|v| !v.is_empty())
                .map(|_| "***".to_string())
        };
        Self {
            access_key_id: self.access_key_id.clone(),
            secret_access_key: if self.secret_access_key.is_empty() {
                String::new()
            } else {
                "***".into()
            },
            session_token: mask(&self.session_token),
            region: self.region.clone(),
            tenant_id: self.tenant_id.clone(),
            client_id: self.client_id.clone(),
            client_secret: mask(&self.client_secret),
            subscription_id: self.subscription_id.clone(),
            access_token: mask(&self.access_token),
            project_id: self.project_id.clone(),
            api_key: self.api_key.clone(),
            api_secret: mask(&self.api_secret),
        }
    }
}

/// A managed instance found in the provider's account.
///
/// Derives `Deserialize` as well as `Serialize` because the frontend holds the
/// discovery result and sends it back when the user picks one to connect to.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredResource {
    /// Provider-scoped identifier (ARN, ARM id, or API id).
    pub id: String,
    pub name: String,
    pub provider: CloudProvider,
    /// Which managed service this came from (matches `CloudService::id`).
    pub service: String,
    /// Protocol the endpoints speak.
    pub backend: BackendKind,
    pub region: Option<String>,
    /// One or more endpoints to connect to.
    pub endpoints: Vec<Endpoint>,
    /// Whether the endpoint requires TLS.
    pub tls: bool,
    /// Username, when the provider issues one.
    pub username: Option<String>,
    /// Logical database index, when meaningful.
    pub database: Option<i64>,
    /// Non-secret facts worth showing (engine version, node count, ...).
    pub meta: BTreeMap<String, String>,
    /// True when the provider will only accept a token the user must supply.
    pub needs_password: bool,
}

impl DiscoveredResource {
    /// Turn a discovered instance into a connectable profile.
    ///
    /// `secret` is the data-plane password/AUTH token, which is *not* the same
    /// as the control-plane credentials: providers deliberately separate them.
    pub fn to_connection_config(&self, secret: Option<&str>) -> AppResult<ConnectionConfig> {
        let primary = self
            .endpoints
            .first()
            .ok_or_else(|| AppError::server(format!("{} has no endpoint", self.name)))?;

        let mut extras: Vec<String> = self
            .endpoints
            .iter()
            .skip(1)
            .map(|endpoint| endpoint.authority())
            .collect();

        let (mode, port, host) = if self.backend == BackendKind::Redis && extras.len() > 0 {
            // A Redis cluster reports the configuration endpoint plus shards;
            // connecting to the configuration endpoint lets the driver discover
            // the topology itself, so only pass that one.
            extras.clear();
            (RedisMode::Cluster, primary.port, primary.host.clone())
        } else {
            (RedisMode::Standalone, primary.port, primary.host.clone())
        };

        let mut meta = self.meta.clone();
        meta.insert("cloud_provider".into(), self.provider.as_str().into());
        meta.insert("cloud_service".into(), self.service.clone());
        meta.insert("cloud_resource_id".into(), self.id.clone());
        if let Some(region) = self.region.as_ref() {
            meta.insert("cloud_region".into(), region.clone());
        }

        Ok(ConnectionConfig {
            id: String::new(),
            name: format!("{} · {}", self.name, provider_label(self.provider)),
            kind: self.backend,
            host,
            port,
            extra_nodes: extras,
            username: self.username.clone().unwrap_or_default(),
            password: secret.unwrap_or_default().to_string(),
            tls: if self.tls {
                TlsMode::Enabled
            } else {
                TlsMode::Disabled
            },
            tls_ca_file: None,
            tls_skip_verify: false,
            redis_mode: mode,
            redis_db: self.database.unwrap_or(0),
            redis_sentinel_master: None,
            kafka_sasl_mechanism: if self.backend == BackendKind::Kafka && self.username.is_some() {
                "plain".into()
            } else if self.backend == BackendKind::Kafka && self.tls {
                // MSK and Confluent commonly default to SASL/PLAIN over TLS.
                "plain".into()
            } else {
                "none".into()
            },
            kafka_group_id: None,
            vhost: "/".into(),
            client_id: None,
            keep_alive: Some(30),
            token: None,
            timeout_secs: Some(15),
            max_payload_bytes: Some(512 * 1024),
            meta,
        })
    }
}

/// Display label for a provider.
pub fn provider_label(provider: CloudProvider) -> &'static str {
    match provider {
        CloudProvider::Aws => "AWS",
        CloudProvider::Azure => "Azure",
        CloudProvider::Gcp => "Google Cloud",
        CloudProvider::Confluent => "Confluent Cloud",
        CloudProvider::Upstash => "Upstash",
        CloudProvider::CloudAmqp => "CloudAMQP",
        CloudProvider::RedisCloud => "Redis Cloud",
    }
}

/// The full catalogue of managed services we can discover.
pub fn catalog() -> Vec<CloudService> {
    vec![
        CloudService {
            id: "aws_elasticache",
            provider: CloudProvider::Aws,
            backend: BackendKind::Redis,
            auth: CloudAuthKind::AwsAccessKey,
            credential_fields: vec!["access_key_id", "secret_access_key", "region"],
        },
        CloudService {
            id: "aws_msk",
            provider: CloudProvider::Aws,
            backend: BackendKind::Kafka,
            auth: CloudAuthKind::AwsAccessKey,
            credential_fields: vec!["access_key_id", "secret_access_key", "region"],
        },
        CloudService {
            id: "aws_mq",
            provider: CloudProvider::Aws,
            backend: BackendKind::RabbitMq,
            auth: CloudAuthKind::AwsAccessKey,
            credential_fields: vec!["access_key_id", "secret_access_key", "region"],
        },
        CloudService {
            id: "azure_cache_redis",
            provider: CloudProvider::Azure,
            backend: BackendKind::Redis,
            auth: CloudAuthKind::AzureClientSecret,
            credential_fields: vec![
                "tenant_id",
                "client_id",
                "client_secret",
                "subscription_id",
            ],
        },
        CloudService {
            id: "gcp_memorystore",
            provider: CloudProvider::Gcp,
            backend: BackendKind::Redis,
            auth: CloudAuthKind::BearerToken,
            credential_fields: vec!["access_token", "project_id"],
        },
        CloudService {
            id: "confluent_kafka",
            provider: CloudProvider::Confluent,
            backend: BackendKind::Kafka,
            auth: CloudAuthKind::ApiKeySecret,
            credential_fields: vec!["api_key", "api_secret"],
        },
        CloudService {
            id: "upstash_redis",
            provider: CloudProvider::Upstash,
            backend: BackendKind::Redis,
            auth: CloudAuthKind::ApiKeySecret,
            credential_fields: vec!["api_key"],
        },
        CloudService {
            id: "upstash_kafka",
            provider: CloudProvider::Upstash,
            backend: BackendKind::Kafka,
            auth: CloudAuthKind::ApiKeySecret,
            credential_fields: vec!["api_key", "api_secret"],
        },
        CloudService {
            id: "cloudamqp",
            provider: CloudProvider::CloudAmqp,
            backend: BackendKind::RabbitMq,
            auth: CloudAuthKind::ApiKeySecret,
            credential_fields: vec!["api_key"],
        },
        CloudService {
            id: "redis_cloud",
            provider: CloudProvider::RedisCloud,
            backend: BackendKind::Redis,
            auth: CloudAuthKind::ApiKeySecret,
            credential_fields: vec!["api_key", "api_secret"],
        },
    ]
}

/// Discover instances for one service.
pub async fn discover(
    service: &str,
    credentials: &CloudCredentials,
    timeout: std::time::Duration,
) -> AppResult<Vec<DiscoveredResource>> {
    match service {
        "aws_elasticache" => aws::elasticache::discover(credentials, timeout).await,
        "aws_msk" => aws::msk::discover(credentials, timeout).await,
        "aws_mq" => aws::mq::discover(credentials, timeout).await,
        "azure_cache_redis" => azure::redis::discover(credentials, timeout).await,
        "gcp_memorystore" => gcp::memorystore::discover(credentials, timeout).await,
        "confluent_kafka" => confluent::discover(credentials, timeout).await,
        "upstash_redis" => upstash::discover_redis(credentials, timeout).await,
        "upstash_kafka" => upstash::discover_kafka(credentials, timeout).await,
        "cloudamqp" => cloudamqp::discover(credentials, timeout).await,
        "redis_cloud" => redis_cloud::discover(credentials, timeout).await,
        other => Err(AppError::invalid(format!("unknown cloud service {other}"))),
    }
}

pub mod confluent;
pub mod cloudamqp;
pub mod redis_cloud;
pub mod upstash;

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(tls: bool) -> DiscoveredResource {
        DiscoveredResource {
            id: "arn:aws:elasticache:eu-west-1:1:cluster:cache-1".into(),
            name: "cache-1".into(),
            provider: CloudProvider::Aws,
            service: "aws_elasticache".into(),
            backend: BackendKind::Redis,
            region: Some("eu-west-1".into()),
            endpoints: vec![Endpoint::new("cache-1.abc.0001.euw1.cache.amazonaws.com", 6379)],
            tls,
            username: None,
            database: None,
            meta: BTreeMap::new(),
            needs_password: false,
        }
    }

    #[test]
    fn builds_a_redis_profile_from_a_discovered_instance() {
        let config = resource(true).to_connection_config(Some("auth-token")).unwrap();
        assert_eq!(config.kind, BackendKind::Redis);
        assert_eq!(config.port, 6379);
        assert_eq!(config.tls, TlsMode::Enabled);
        assert_eq!(config.password, "auth-token");
        assert_eq!(
            config.meta.get("cloud_provider").map(String::as_str),
            Some("aws")
        );
        assert!(config.name.contains("cache-1"));
    }

    #[test]
    fn cluster_mode_uses_only_the_configuration_endpoint() {
        let mut found = resource(true);
        found.endpoints.push(Endpoint::new("shard-2", 6379));
        let config = found.to_connection_config(Some("pw")).unwrap();
        assert_eq!(config.redis_mode, RedisMode::Cluster);
        assert!(
            config.extra_nodes.is_empty(),
            "cluster topology is discovered by the driver, not pre-seeded"
        );
    }

    #[test]
    fn credentials_are_redacted() {
        let creds = CloudCredentials {
            access_key_id: "AKIA".into(),
            secret_access_key: "topsecret".into(),
            session_token: Some("session".into()),
            api_secret: Some("apisecret".into()),
            ..Default::default()
        };
        let red = creds.redacted();
        assert_eq!(red.access_key_id, "AKIA");
        assert_eq!(red.secret_access_key, "***");
        assert_eq!(red.session_token.as_deref(), Some("***"));
        assert_eq!(red.api_secret.as_deref(), Some("***"));
    }

    #[test]
    fn catalog_covers_every_provider() {
        let services = catalog();
        for provider in CloudProvider::all() {
            assert!(
                services.iter().any(|service| service.provider == *provider),
                "{provider:?} has no discoverable service"
            );
        }
    }

    #[tokio::test]
    async fn unknown_service_is_rejected() {
        // The rejection happens before any network call, so this needs no
        // provider credentials.
        let err = discover(
            "nope",
            &CloudCredentials::default(),
            std::time::Duration::from_secs(1),
        )
        .await
        .unwrap_err();
        assert!(err.message.contains("unknown cloud service"));
    }
}
