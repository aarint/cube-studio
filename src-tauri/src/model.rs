//! Transport-agnostic data model shared by all backends.
//!
//! Redis, Memcached, Kafka, RabbitMQ, NATS and MQTT have very different
//! vocabularies, but every one of them can be browsed as
//! `container -> entries -> payload`. Normalising to that shape lets the
//! frontend render all six with one set of components while still exposing
//! backend specific operations.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::config::BackendKind;

/// A top level thing you can browse inside a connection.
///
/// * Redis      -> one per logical database (DB0, DB1, ...)
/// * Memcached  -> the server itself (keys are discovered via `stats cachedump` or lru crawler)
/// * Kafka      -> one per topic
/// * RabbitMQ   -> one per queue, plus one per exchange
/// * NATS       -> one per JetStream stream, or the whole core-NATS subject space
/// * MQTT       -> one per topic filter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Container {
    /// Stable identifier, unique within the connection.
    pub id: String,
    pub name: String,
    pub kind: ContainerKind,
    /// Free-form extra information shown as a subtitle/badge.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
    /// Total number of entries, when cheaply known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ContainerKind {
    RedisDb,
    MemcachedServer,
    KafkaTopic,
    RabbitQueue,
    RabbitExchange,
    NatsStream,
    NatsSubject,
    MqttTopic,
}

/// What a backend can do. Drives which toolbar buttons the UI enables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub kind: BackendKind,
    /// Multiple browsable namespaces (Redis DBs, Kafka topics, ...).
    pub containers: bool,
    /// Entries can be enumerated up front.
    pub list_entries: bool,
    /// Entries can be created or overwritten by the user.
    pub write_entries: bool,
    /// Entries can be deleted.
    pub delete_entries: bool,
    /// Entries carry a TTL that can be edited.
    pub ttl: bool,
    /// Arbitrary command/query console is available (Redis, NATS subject, ...).
    pub console: bool,
    /// Live message tailing over a subscription.
    pub tail: bool,
    /// A single "publish" action that appends a message.
    pub publish: bool,
    /// Server statistics / info endpoint.
    pub stats: bool,
    /// Key space can be flushed in bulk.
    pub flush: bool,
    /// Free-form notes shown in the UI (e.g. "Memcached cannot enumerate keys").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
}

impl Capabilities {
    pub fn for_kind(kind: BackendKind) -> Self {
        use BackendKind::*;
        let mut caps = Self {
            kind,
            containers: true,
            list_entries: true,
            write_entries: true,
            delete_entries: true,
            ttl: false,
            console: false,
            tail: false,
            publish: false,
            stats: true,
            flush: false,
            notes: Vec::new(),
        };
        match kind {
            Redis => {
                caps.ttl = true;
                caps.console = true;
                caps.tail = true;
                caps.flush = true;
            }
            Memcached => {
                caps.containers = false;
                caps.ttl = true;
                caps.flush = true;
                caps.console = true;
                caps.notes.push(
                    "Memcached has no key enumeration command. Keys are collected from \
                     `stats items`/`stats cachedump`, so only keys still resident in \
                     memory appear here."
                        .into(),
                );
            }
            Kafka => {
                caps.list_entries = true;
                caps.write_entries = false;
                caps.delete_entries = false;
                caps.publish = true;
                caps.tail = true;
                caps.notes.push(
                    "Kafka topics are append-only: messages cannot be edited or deleted \
                     individually."
                        .into(),
                );
            }
            RabbitMq => {
                caps.write_entries = false;
                caps.delete_entries = false;
                caps.publish = true;
                caps.tail = true;
                caps.notes.push(
                    "Publishing goes through an exchange with a routing key; the queue \
                     view shows messages as the broker hands them out."
                        .into(),
                );
            }
            Nats => {
                caps.list_entries = false;
                caps.write_entries = false;
                caps.delete_entries = false;
                caps.publish = true;
                caps.tail = true;
                caps.notes.push(
                    "Core NATS is fire-and-forget: there is no history to list. Use the \
                     tail panel to watch a subject live."
                        .into(),
                );
            }
            Mqtt => {
                caps.list_entries = false;
                caps.write_entries = false;
                caps.delete_entries = false;
                caps.publish = true;
                caps.tail = true;
            }
        }
        caps
    }
}

/// How a payload should be presented.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Payload {
    /// UTF-8 rendering when the bytes are valid UTF-8 and printable enough.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Base64 of the raw bytes, always present.
    pub base64: String,
    /// Byte length before truncation.
    pub size: usize,
    /// True when the payload was cut down to `max_bytes`.
    pub truncated: bool,
    /// True when the payload is probably not human text.
    pub binary: bool,
    /// Pretty-printed JSON when the text parses as JSON.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json: Option<serde_json::Value>,
}

impl Payload {
    /// Build a payload, truncating at `max_bytes`.
    pub fn from_bytes(bytes: &[u8], max_bytes: usize) -> Self {
        let size = bytes.len();
        let truncated = size > max_bytes;
        let slice = if truncated { &bytes[..max_bytes] } else { bytes };

        let text = match std::str::from_utf8(slice) {
            Ok(s) if !truncated || !s.is_empty() => {
                // A truncated multi-byte sequence would end mid-character; slicing
                // above can break UTF-8, so re-validate defensively.
                Some(s.to_string())
            }
            _ => None,
        };

        let binary = text.as_ref().is_none_or(|t| t.contains('\u{0}'));
        let json = text
            .as_deref()
            .filter(|t| {
                let t = t.trim_start();
                (t.starts_with('{') || t.starts_with('[')) && !truncated
            })
            .and_then(|t| serde_json::from_str::<serde_json::Value>(t).ok());

        Self {
            text: text.filter(|_| !binary),
            base64: {
                use base64::Engine as _;
                base64::engine::general_purpose::STANDARD.encode(slice)
            },
            size,
            truncated,
            binary,
            json,
        }
    }

    pub fn empty() -> Self {
        Self {
            text: Some(String::new()),
            base64: String::new(),
            size: 0,
            truncated: false,
            binary: false,
            json: None,
        }
    }
}

/// One browsable item inside a container.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    /// Identifier used to re-fetch this entry.
    pub id: String,
    /// Display label (Redis key, Kafka offset, ...).
    pub label: String,
    /// Logical type in the backend's own vocabulary ("string", "hash", "message").
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<usize>,
    /// Seconds remaining, `-1` means no expiry, `None` means not applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub meta: BTreeMap<String, String>,
    /// Only populated when the entry was read in full.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload: Option<Payload>,
}

impl Entry {
    pub fn new(id: impl Into<String>, label: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            kind: kind.into(),
            size: None,
            ttl: None,
            timestamp: None,
            meta: BTreeMap::new(),
            payload: None,
        }
    }
}

/// A page of entries plus the cursor needed to fetch the next page.
///
/// Redis, Kafka and Memcached are all page/cursor based, so every backend
/// returns this shape even when it produced everything in one go.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryPage {
    pub entries: Vec<Entry>,
    /// Opaque cursor for the next page; `None` means "no more data".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    /// Total count when the backend knows it cheaply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<i64>,
}

impl EntryPage {
    pub fn single(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            next_cursor: None,
            total: None,
        }
    }
}

/// Options passed to `write_entry` so each backend can pick what it needs.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct WriteOptions {
    /// Redis value type to write ("string", "hash", "list", "set", "zset", "json").
    pub entry_kind: Option<String>,
    /// TTL in seconds; `0` or `None` means persist forever.
    pub ttl: Option<i64>,
    /// Message key (Kafka), routing key (RabbitMQ), reply subject (NATS).
    pub routing_key: Option<String>,
    /// Exchange to publish to (RabbitMQ). Empty means the default exchange.
    pub exchange: Option<String>,
    /// Message headers / user properties.
    pub headers: Option<BTreeMap<String, String>>,
    /// MQTT QoS (0, 1, 2), retained flag, or Kafka partition.
    pub qos: Option<u8>,
    pub retained: Option<bool>,
    pub partition: Option<i32>,
}

/// Result of a write/publish/delete operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationResult {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Ids that were affected, when meaningful.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub affected: Vec<String>,
}

impl MutationResult {
    pub fn ok() -> Self {
        Self {
            ok: true,
            message: None,
            affected: Vec::new(),
        }
    }

    pub fn with_message(message: impl Into<String>) -> Self {
        Self {
            ok: true,
            message: Some(message.into()),
            affected: Vec::new(),
        }
    }
}

/// A message pushed from a live subscription to the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamMessage {
    /// Subscription that produced this message.
    pub subscription_id: String,
    /// Channel / subject / topic / partition the message came from.
    pub source: String,
    pub payload: Payload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub headers: BTreeMap<String, String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Backend specific position (Kafka offset, MQTT packet id, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<String>,
}

/// Outcome of a raw command-console invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsoleResult {
    /// Pretty rendering of the reply, ready to display verbatim.
    pub output: String,
    /// True when the backend reported an error reply.
    pub error: bool,
    /// Wall-clock duration in milliseconds.
    pub elapsed_ms: u64,
}

/// A single server statistic row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatItem {
    pub section: String,
    pub key: String,
    pub value: String,
}
