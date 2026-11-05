//! The backend abstraction.
//!
//! Every middleware Cube Studio supports is wrapped in a type implementing
//! [`Backend`]. The Tauri command layer only ever talks to `dyn Backend`, so
//! adding a seventh protocol means adding one module and one match arm.
//!
//! Design notes:
//! * All methods are `async` and `&self` so a session can serve concurrent
//!   calls (the UI pages a big keyspace while a TTL refresh runs).
//! * Anything that streams (tail/subscribe) hands back a [`tokio::sync::mpsc`]
//!   receiver; the command layer pumps it into Tauri events.
//! * Errors are always [`AppError`], never driver-specific types.

use async_trait::async_trait;
use tokio::sync::mpsc;

use crate::config::{BackendKind, ConnectionConfig};
use crate::error::{AppError, AppResult};
use crate::model::{
    Capabilities, ConsoleResult, Container, Entry, EntryPage, MutationResult, Payload, StatItem,
    StreamMessage, WriteOptions,
};

pub mod kafka_backend;
pub mod memcached_backend;
pub mod memcached_proto;
pub mod mqtt_backend;
pub mod nats_backend;
pub mod rabbit_backend;
pub mod redis_backend;

pub use kafka_backend::KafkaBackend;
pub use memcached_backend::MemcachedBackend;
pub use mqtt_backend::MqttBackend;
pub use nats_backend::NatsBackend;
pub use rabbit_backend::RabbitBackend;
pub use redis_backend::RedisBackend;

/// Connect to whatever the config describes.
///
/// The whole handshake is bounded by the profile's timeout, because drivers
/// differ in how eagerly they retry (rskafka, for one, backs off and retries
/// broker discovery indefinitely) and a UI must never hang on a bad endpoint.
pub async fn connect(config: &ConnectionConfig) -> AppResult<Box<dyn Backend>> {
    let config = config.clone();
    let timeout = config.timeout();
    let budget = timeout * 3;

    let attempt = async {
        let backend: Box<dyn Backend> = match config.kind {
            BackendKind::Redis => Box::new(RedisBackend::connect(config.clone()).await?),
            BackendKind::Memcached => Box::new(MemcachedBackend::connect(config.clone()).await?),
            BackendKind::Kafka => Box::new(KafkaBackend::connect(config.clone()).await?),
            BackendKind::RabbitMq => Box::new(RabbitBackend::connect(config.clone()).await?),
            BackendKind::Nats => Box::new(NatsBackend::connect(config.clone()).await?),
            BackendKind::Mqtt => Box::new(MqttBackend::connect(config.clone()).await?),
        };
        Ok::<_, AppError>(backend)
    };

    tokio::time::timeout(budget, attempt)
        .await
        .map_err(|_| {
            AppError::new(
                crate::error::ErrorKind::Network,
                format!(
                    "could not reach {} at {}:{} within {}s",
                    config.kind.display_name(),
                    config.host,
                    config.port,
                    budget.as_secs()
                ),
            )
        })?
}

/// What a backend can report about itself right after connecting.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ConnectionInfo {
    pub kind: crate::config::BackendKind,
    /// Server greeting string ("7.2.4", "RabbitMQ 3.13.7", ...).
    pub server_version: Option<String>,
    /// Capabilities, so the UI can hide unsupported controls.
    pub capabilities: Capabilities,
    /// Default container to select once connected.
    pub default_container: Option<String>,
}

/// Live subscription handle returned by [`Backend::tail`].
///
/// Dropping the receiver is the cancellation signal: the backend's forwarding
/// task notices the closed channel and stops its upstream subscription.
pub struct TailHandle {
    pub subscription_id: String,
    pub receiver: mpsc::Receiver<StreamMessage>,
}

/// One protocol implementation.
#[async_trait]
pub trait Backend: Send + Sync + 'static {
    /// Round-trip a trivial request to prove the endpoint is usable.
    async fn ping(&self) -> AppResult<()>;

    /// Describe the server and its capabilities.
    async fn info(&self) -> AppResult<ConnectionInfo>;

    /// List the browsable namespaces (Redis DBs, Kafka topics, queues...).
    async fn containers(&self) -> AppResult<Vec<Container>>;

    /// Page through the entries of one container.
    ///
    /// `cursor` is whatever this backend returned as `next_cursor` before;
    /// `search` is a backend-native pattern (Redis glob, Kafka substring...).
    async fn entries(
        &self,
        container: &str,
        cursor: Option<&str>,
        limit: usize,
        search: Option<&str>,
    ) -> AppResult<EntryPage>;

    /// Read one entry in full.
    async fn read_entry(&self, container: &str, entry: &str) -> AppResult<Entry>;

    /// Create or overwrite an entry.
    async fn write_entry(
        &self,
        container: &str,
        entry: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult>;

    /// Delete one entry (or a batch when the backend supports it cheaply).
    async fn delete_entry(&self, container: &str, entries: &[String]) -> AppResult<MutationResult>;

    /// Change the TTL of one entry. `ttl <= 0` means "persist forever".
    async fn set_ttl(&self, container: &str, entry: &str, ttl_secs: i64) -> AppResult<()> {
        let _ = (container, entry, ttl_secs);
        Err(crate::error::AppError::unsupported(
            "this backend does not support per-entry TTL",
        ))
    }

    /// Publish a message without storing it as a browsable entry.
    async fn publish(
        &self,
        container: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        // Default: publishing is the same as writing an entry.
        let target = options
            .routing_key
            .clone()
            .unwrap_or_else(|| container.to_string());
        self.write_entry(container, &target, payload, options).await
    }

    /// Start streaming messages. The default implementation reports that the
    /// backend has no subscription support.
    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        let _ = container;
        Err(crate::error::AppError::unsupported(
            "this backend does not support live tailing",
        ))
    }

    /// Run a raw command in the backend's own dialect.
    async fn console(&self, command: &str) -> AppResult<ConsoleResult> {
        let _ = command;
        Err(crate::error::AppError::unsupported(
            "this backend has no command console",
        ))
    }

    /// Server statistics, grouped into sections.
    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        Ok(Vec::new())
    }

    /// Remove every entry in a container.
    async fn flush(&self, container: &str) -> AppResult<MutationResult> {
        let _ = container;
        Err(crate::error::AppError::unsupported(
            "this backend cannot be flushed",
        ))
    }

    /// Called when the session is closed. Backends should drop connections here
    /// rather than in `Drop`, because `Drop` cannot await.
    async fn shutdown(&self) {}

    /// The config this session was built from.
    fn config(&self) -> &ConnectionConfig;
}

/// Convenience: turn a byte payload into a [`Payload`] honouring the session
/// payload limit.
pub fn payload_of(config: &ConnectionConfig, bytes: &[u8]) -> Payload {
    Payload::from_bytes(bytes, config.max_payload())
}
