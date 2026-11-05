//! Connection configuration: what the user typed, validated and normalised.
//!
//! One flat struct is used instead of a tagged enum because the frontend form
//! is shared across backend kinds and only shows the relevant fields. Unknown
//! fields are ignored so old saved profiles keep working after an upgrade.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

/// Every middleware Cube Studio speaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackendKind {
    #[serde(alias = "Redis")]
    Redis,
    #[serde(alias = "Memcached", alias = "memcache", alias = "Memcache")]
    Memcached,
    #[serde(alias = "Kafka")]
    Kafka,
    #[serde(
        rename = "rabbitmq",
        alias = "rabbit_mq",
        alias = "RabbitMQ",
        alias = "amqp"
    )]
    RabbitMq,
    #[serde(alias = "NATS", alias = "Nats")]
    Nats,
    #[serde(alias = "MQTT", alias = "Mqtt")]
    Mqtt,
}

impl BackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BackendKind::Redis => "redis",
            BackendKind::Memcached => "memcached",
            BackendKind::Kafka => "kafka",
            BackendKind::RabbitMq => "rabbitmq",
            BackendKind::Nats => "nats",
            BackendKind::Mqtt => "mqtt",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            BackendKind::Redis => "Redis",
            BackendKind::Memcached => "Memcached",
            BackendKind::Kafka => "Kafka",
            BackendKind::RabbitMq => "RabbitMQ",
            BackendKind::Nats => "NATS",
            BackendKind::Mqtt => "MQTT",
        }
    }

    pub fn default_port(self) -> u16 {
        match self {
            BackendKind::Redis => 6379,
            BackendKind::Memcached => 11211,
            BackendKind::Kafka => 9092,
            BackendKind::RabbitMq => 5672,
            BackendKind::Nats => 4222,
            BackendKind::Mqtt => 1883,
        }
    }

    pub fn all() -> &'static [BackendKind] {
        &[
            BackendKind::Redis,
            BackendKind::Memcached,
            BackendKind::Kafka,
            BackendKind::RabbitMq,
            BackendKind::Nats,
            BackendKind::Mqtt,
        ]
    }
}

impl std::fmt::Display for BackendKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// How to reach a single node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Endpoint {
    pub host: String,
    pub port: u16,
}

impl Endpoint {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
        }
    }

    pub fn authority(&self) -> String {
        if self.host.contains(':') && !self.host.starts_with('[') {
            // Bare IPv6 literal.
            format!("[{}]:{}", self.host, self.port)
        } else {
            format!("{}:{}", self.host, self.port)
        }
    }
}

impl std::fmt::Display for Endpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.authority())
    }
}

/// Which Redis deployment flavour we are talking to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedisMode {
    #[default]
    Standalone,
    Cluster,
    Sentinel,
}

impl RedisMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RedisMode::Standalone => "standalone",
            RedisMode::Cluster => "cluster",
            RedisMode::Sentinel => "sentinel",
        }
    }
}

/// How to start TLS with the server.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    #[default]
    Disabled,
    /// Verify the server certificate against the platform trust store.
    Enabled,
    /// Encrypt but accept any certificate. Only for throwaway dev servers.
    Insecure,
}

impl TlsMode {
    pub fn is_enabled(self) -> bool {
        !matches!(self, TlsMode::Disabled)
    }
}

/// A saved connection profile.
///
/// `password` is never persisted in this struct on disk; the store strips it
/// into the encrypted secret section. It is present here because the same
/// struct travels over IPC when connecting.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ConnectionConfig {
    /// Stable id; generated on creation, preserved on edit.
    pub id: String,
    /// Friendly name shown in the sidebar and tabs.
    pub name: String,
    pub kind: BackendKind,
    /// Primary host used for display and single-node backends.
    pub host: String,
    pub port: u16,
    /// Extra nodes for cluster / multi-broker setups. `host:port` strings.
    pub extra_nodes: Vec<String>,
    pub username: String,
    pub password: String,

    // ---- TLS -----------------------------------------------------------
    pub tls: TlsMode,
    /// Optional path to a custom CA bundle (PEM).
    pub tls_ca_file: Option<String>,
    /// Skip hostname verification but still encrypt.
    pub tls_skip_verify: bool,

    // ---- Redis ---------------------------------------------------------
    pub redis_mode: RedisMode,
    /// Logical database index used on connect.
    pub redis_db: i64,
    /// Sentinel master name when `redis_mode == sentinel`.
    pub redis_sentinel_master: Option<String>,

    // ---- Kafka ---------------------------------------------------------
    /// SASL mechanism: none | plain | scram-sha-256 | scram-sha-512.
    pub kafka_sasl_mechanism: String,
    /// Consumer group used when tailing topics.
    pub kafka_group_id: Option<String>,

    // ---- RabbitMQ / NATS / MQTT ---------------------------------------
    /// AMQP virtual host.
    pub vhost: String,
    /// MQTT client id; generated when empty.
    pub client_id: Option<String>,
    /// MQTT keep-alive in seconds.
    pub keep_alive: Option<u16>,
    /// NATS auth token (alternative to user/password).
    pub token: Option<String>,

    // ---- behaviour -----------------------------------------------------
    /// Timeout applied to connect and per-operation calls, in seconds.
    pub timeout_secs: Option<u64>,
    /// Payloads larger than this are truncated before crossing IPC.
    pub max_payload_bytes: Option<usize>,
    /// Free-form profile metadata (colour, group, notes, ...).
    pub meta: BTreeMap<String, String>,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: String::new(),
            kind: BackendKind::Redis,
            host: "127.0.0.1".into(),
            port: BackendKind::Redis.default_port(),
            extra_nodes: Vec::new(),
            username: String::new(),
            password: String::new(),
            tls: TlsMode::Disabled,
            tls_ca_file: None,
            tls_skip_verify: false,
            redis_mode: RedisMode::Standalone,
            redis_db: 0,
            redis_sentinel_master: None,
            kafka_sasl_mechanism: "none".into(),
            kafka_group_id: None,
            vhost: "/".into(),
            client_id: None,
            keep_alive: Some(30),
            token: None,
            timeout_secs: Some(10),
            max_payload_bytes: Some(512 * 1024),
            meta: BTreeMap::new(),
        }
    }
}

impl ConnectionConfig {
    /// Endpoints to try, primary first.
    pub fn endpoints(&self) -> AppResult<Vec<Endpoint>> {
        let host = self.host.trim();
        if host.is_empty() {
            return Err(AppError::invalid("host is required"));
        }
        if self.port == 0 {
            return Err(AppError::invalid("port must be between 1 and 65535"));
        }
        let mut out = vec![Endpoint::new(host, self.port)];
        for raw in &self.extra_nodes {
            let raw = raw.trim();
            if raw.is_empty() {
                continue;
            }
            out.push(parse_endpoint(raw, self.kind.default_port())?);
        }
        Ok(out)
    }

    pub fn timeout(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_secs.unwrap_or(10).clamp(1, 600))
    }

    pub fn max_payload(&self) -> usize {
        self.max_payload_bytes
            .unwrap_or(512 * 1024)
            .clamp(1024, 64 * 1024 * 1024)
    }

    /// Label used when the user did not give the profile a name.
    pub fn display_name(&self) -> String {
        if !self.name.trim().is_empty() {
            return self.name.trim().to_string();
        }
        format!("{} ({})", self.host.trim(), self.port)
    }

    /// Validate and fill in defaults. Called before every connect.
    pub fn normalize(mut self) -> AppResult<Self> {
        self.host = self.host.trim().to_string();
        if self.host.is_empty() {
            return Err(AppError::invalid("host is required"));
        }
        // Accept "host:port" typed into the host field, which is a very common
        // paste pattern from config files and dashboards.
        if self.host.contains(':') && !self.host.starts_with('[') && self.host.matches(':').count() == 1
        {
            let (h, p) = self.host.split_once(':').unwrap();
            if let Ok(port) = p.parse::<u16>() {
                if port > 0 {
                    self.host = h.to_string();
                    self.port = port;
                }
            }
        }
        if self.port == 0 {
            self.port = self.kind.default_port();
        }
        if self.name.trim().is_empty() {
            self.name = self.display_name();
        }
        if self.id.trim().is_empty() {
            self.id = uuid::Uuid::new_v4().to_string();
        }
        if self.timeout_secs.is_none() {
            self.timeout_secs = Some(10);
        }
        if self.vhost.trim().is_empty() {
            self.vhost = "/".into();
        }
        if self.kind != BackendKind::Redis {
            self.redis_mode = RedisMode::Standalone;
            self.redis_db = 0;
        } else {
            // Only standalone/redis-mode supports extra nodes for cluster.
            if self.redis_mode == RedisMode::Standalone {
                self.extra_nodes.clear();
            }
        }
        if self.redis_db < 0 {
            self.redis_db = 0;
        }
        self.kafka_sasl_mechanism = self.kafka_sasl_mechanism.to_lowercase();
        if self.kafka_sasl_mechanism.trim().is_empty() {
            self.kafka_sasl_mechanism = "none".into();
        }
        Ok(self)
    }

    /// Same profile but with the password removed, for logs and listing.
    pub fn redacted(&self) -> Self {
        let mut clone = self.clone();
        if !clone.password.is_empty() {
            clone.password = "***".into();
        }
        if let Some(token) = clone.token.as_mut() {
            if !token.is_empty() {
                *token = "***".into();
            }
        }
        clone
    }
}

/// Parse `host:port`, `[::1]:6379`, `redis://host:6379` or a bare host.
pub fn parse_endpoint(raw: &str, default_port: u16) -> AppResult<Endpoint> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::invalid("empty endpoint"));
    }
    // Strip an optional scheme.
    let without_scheme = match raw.split_once("://") {
        Some((_, rest)) => rest,
        None => raw,
    };
    // Drop any trailing path.
    let authority = without_scheme.split('/').next().unwrap_or(without_scheme);
    // Drop credentials if present.
    let authority = authority.rsplit('@').next().unwrap_or(authority);

    if let Some(rest) = authority.strip_prefix('[') {
        // IPv6 literal
        let (host, tail) = rest
            .split_once(']')
            .ok_or_else(|| AppError::invalid(format!("unterminated IPv6 address: {raw}")))?;
        let port = tail
            .strip_prefix(':')
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(default_port);
        return Ok(Endpoint::new(host, port));
    }

    match authority.rsplit_once(':') {
        Some((host, port)) => match port.parse::<u16>() {
            Ok(port) if port > 0 => Ok(Endpoint::new(host, port)),
            _ => Err(AppError::invalid(format!("invalid port in endpoint: {raw}"))),
        },
        None => Ok(Endpoint::new(authority, default_port)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_endpoint() {
        let ep = parse_endpoint("10.0.0.5:6380", 6379).unwrap();
        assert_eq!(ep, Endpoint::new("10.0.0.5", 6380));
    }

    #[test]
    fn parses_endpoint_with_scheme() {
        let ep = parse_endpoint("redis://cache.internal/0", 6379).unwrap();
        assert_eq!(ep, Endpoint::new("cache.internal", 6379));
    }

    #[test]
    fn parses_ipv6_endpoint() {
        let ep = parse_endpoint("[::1]:7000", 6379).unwrap();
        assert_eq!(ep.host, "::1");
        assert_eq!(ep.port, 7000);
    }

    #[test]
    fn splits_host_port_typed_into_host_field() {
        let cfg = ConnectionConfig {
            host: "example.com:7001".into(),
            ..Default::default()
        }
        .normalize()
        .unwrap();
        assert_eq!(cfg.host, "example.com");
        assert_eq!(cfg.port, 7001);
    }

    #[test]
    fn normalizes_display_name() {
        let cfg = ConnectionConfig {
            host: "cache".into(),
            port: 6379,
            ..Default::default()
        }
        .normalize()
        .unwrap();
        assert_eq!(cfg.name, "cache (6379)");
        assert!(!cfg.id.is_empty());
    }

    #[test]
    fn clears_extra_nodes_for_standalone_redis() {
        let cfg = ConnectionConfig {
            extra_nodes: vec!["other:6379".into()],
            ..Default::default()
        }
        .normalize()
        .unwrap();
        assert!(cfg.extra_nodes.is_empty());
    }

    #[test]
    fn redacts_secrets() {
        let mut cfg = ConnectionConfig::default();
        cfg.password = "hunter2".into();
        cfg.token = Some("tok".into());
        let red = cfg.redacted();
        assert_eq!(red.password, "***");
        assert_eq!(red.token.as_deref(), Some("***"));
    }

    #[test]
    fn backend_kind_accepts_aliases() {
        let kind: BackendKind = serde_json::from_str("\"rabbit_mq\"").unwrap();
        assert_eq!(kind, BackendKind::RabbitMq);
        let kind: BackendKind = serde_json::from_str("\"amqp\"").unwrap();
        assert_eq!(kind, BackendKind::RabbitMq);
        let kind: BackendKind = serde_json::from_str("\"RabbitMQ\"").unwrap();
        assert_eq!(kind, BackendKind::RabbitMq);
    }
}
