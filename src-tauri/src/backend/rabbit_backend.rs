//! RabbitMQ / AMQP 0-9-1 driver.
//!
//! Browsing model: one container per queue. Because AMQP has no "list
//! messages" primitive, the entry list is produced with `basic.get` pulls and
//! every message is immediately `nack`-ed with `requeue`, leaving the queue
//! exactly as it was found.
//!
//! AMQP also has no "list queues" command. Queue names come from the profile's
//! `rabbit_queues` metadata, which the UI edits, so the user controls what is
//! browsed instead of the app guessing.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use lapin::message::Delivery;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicGetOptions, BasicNackOptions, BasicPublishOptions,
    BasicQosOptions, BasicRejectOptions, QueueDeclareOptions, QueueDeleteOptions, QueuePurgeOptions,
};
use lapin::types::{AMQPValue, FieldTable, ShortString};
use lapin::{BasicProperties, Connection, ConnectionProperties};
use tokio::sync::mpsc;

use crate::backend::{Backend, ConnectionInfo, TailHandle};
use crate::config::{BackendKind, ConnectionConfig, TlsMode};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::model::{
    Capabilities, ConsoleResult, Container, ContainerKind, Entry, EntryPage, MutationResult,
    Payload, StatItem, StreamMessage, WriteOptions,
};

/// Profile metadata key holding the comma separated queue list.
pub const QUEUES_META_KEY: &str = "rabbit_queues";

pub struct RabbitBackend {
    config: ConnectionConfig,
    connection: Arc<Connection>,
}

impl RabbitBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let endpoints = config.endpoints()?;
        let primary = &endpoints[0];

        let scheme = match config.tls {
            TlsMode::Disabled => "amqp",
            _ => "amqps",
        };
        let credentials = if config.username.is_empty() && config.password.is_empty() {
            String::new()
        } else {
            format!(
                "{}:{}@",
                percent_encode(&config.username),
                percent_encode(&config.password)
            )
        };
        let uri = format!(
            "{scheme}://{credentials}{}:{}/{}",
            primary.host,
            primary.port,
            percent_encode(&config.vhost)
        );

        let mut builder = lapin::ConnectionBuilder::new()
            .map_err(|e| AppError::internal(format!("cannot initialise AMQP runtime: {e}")))?
            .with_uri_str(uri)
            .with_properties(ConnectionProperties::default().enable_auto_recover());

        if config.tls.is_enabled() {
            builder = builder.with_tls_config(build_tls(&config)?);
        }

        let connection = tokio::time::timeout(config.timeout(), builder.connect())
            .await
            .map_err(|_| AppError::network("RabbitMQ connection timed out"))?
            .map_err(|e| map_amqp_error(&e))?;

        Ok(Self {
            config,
            connection: Arc::new(connection),
        })
    }

    async fn channel(&self) -> AppResult<lapin::Channel> {
        self.connection
            .create_channel()
            .await
            .map_err(|e| map_amqp_error(&e))
    }

    /// Queue names this profile knows about.
    fn configured_queues(&self) -> Vec<String> {
        self.config
            .meta
            .get(QUEUES_META_KEY)
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn build_tls(config: &ConnectionConfig) -> AppResult<lapin::tcp::OwnedTLSConfig> {
    // lapin's TLS knobs are limited to a CA chain and a client identity. There
    // is no "accept invalid certs" switch, so an insecure connection is only
    // possible when the user supplies the CA that signed the broker cert — we
    // say so explicitly rather than silently ignoring the setting.
    if matches!(config.tls, TlsMode::Insecure) || config.tls_skip_verify {
        tracing::warn!(
            "RabbitMQ TLS certificate verification cannot be disabled; \
             provide a CA bundle instead"
        );
    }
    let cert_chain = match config
        .tls_ca_file
        .as_ref()
        .filter(|path| !path.trim().is_empty())
    {
        Some(path) => Some(std::fs::read_to_string(path).map_err(|e| {
            AppError::invalid(format!("cannot read TLS CA file {path}: {e}"))
        })?),
        None => None,
    };
    Ok(lapin::tcp::OwnedTLSConfig {
        identity: None,
        cert_chain,
    })
}

fn map_amqp_error(err: &lapin::Error) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();
    // `lapin::ErrorKind` is re-exported under the same name as our own
    // `ErrorKind`, so match on the fully qualified path.
    let kind = match err.kind() {
        lapin::ErrorKind::IOError(_)
        | lapin::ErrorKind::RuntimeShutdownError(_)
        | lapin::ErrorKind::MissingHeartbeatError => ErrorKind::Network,
        lapin::ErrorKind::SerialisationError(_) | lapin::ErrorKind::ParsingError(_) => {
            ErrorKind::Internal
        }
        lapin::ErrorKind::InvalidChannel(_)
        | lapin::ErrorKind::InvalidChannelState(..)
        | lapin::ErrorKind::InvalidConnectionState(_) => ErrorKind::Network,
        lapin::ErrorKind::ChannelsLimitReached
        | lapin::ErrorKind::InvalidProtocolVersion(_) => ErrorKind::Unsupported,
        lapin::ErrorKind::AuthProviderError(_) => ErrorKind::Unauthorized,
        lapin::ErrorKind::ProtocolError(_) => {
            if lower.contains("access refused")
                || lower.contains("access_refused")
                || lower.contains("not allowed")
            {
                ErrorKind::Unauthorized
            } else if lower.contains("not found") || lower.contains("404") {
                ErrorKind::InvalidInput
            } else {
                ErrorKind::ServerError
            }
        }
        _ => ErrorKind::ServerError,
    };
    AppError::new(kind, message)
}

/// Percent-encode a URI component (userinfo and vhost segments).
fn percent_encode(value: &str) -> String {
    if value == "/" {
        // The default vhost is the path segment "/", i.e. "%2F".
        return "%2F".into();
    }
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

/// Container ids are `queue:<name>`.
fn parse_queue_container(id: &str) -> AppResult<String> {
    let name = id
        .strip_prefix("queue:")
        .ok_or_else(|| AppError::invalid(format!("not a queue container: {id}")))?;
    if name.is_empty() {
        return Err(AppError::invalid("queue name is required"));
    }
    Ok(name.to_string())
}

/// Build an entry from a delivery.
///
/// The id is the message's **position** in the queue, not its delivery tag:
/// tags are per-channel and change every time the queue is peeked, so they
/// cannot be used to refer back to a message.
fn delivery_to_entry(queue: &str, position: usize, delivery: &Delivery) -> Entry {
    let mut entry = Entry::new(
        position.to_string(),
        format!("message {}", position + 1),
        "message",
    );
    entry.size = Some(delivery.data.len());
    entry.meta.insert("queue".into(), queue.to_string());
    entry
        .meta
        .insert("exchange".into(), delivery.exchange.to_string());
    entry
        .meta
        .insert("routing_key".into(), delivery.routing_key.to_string());
    if delivery.redelivered {
        entry.meta.insert("redelivered".into(), "true".into());
    }
    if let Some(id) = delivery.properties.message_id().as_ref() {
        entry
            .meta
            .insert("message_id".into(), id.as_str().to_string());
    }
    if let Some(preview) = Payload::from_bytes(&delivery.data, 160).text {
        entry
            .meta
            .insert("preview".into(), preview.replace(['\n', '\r'], " "));
    }
    entry
}

fn headers_of(delivery: &Delivery) -> BTreeMap<String, String> {
    delivery
        .properties
        .headers()
        .as_ref()
        .map(|table| {
            table
                .inner()
                .iter()
                .map(|(key, value)| (key.to_string(), amqp_value_to_string(value)))
                .collect()
        })
        .unwrap_or_default()
}

fn amqp_value_to_string(value: &AMQPValue) -> String {
    match value {
        AMQPValue::LongString(bytes) => String::from_utf8_lossy(bytes.as_bytes()).to_string(),
        AMQPValue::ShortString(value) => value.as_str().to_string(),
        AMQPValue::Boolean(flag) => flag.to_string(),
        AMQPValue::ShortShortInt(number) => number.to_string(),
        AMQPValue::ShortShortUInt(number) => number.to_string(),
        AMQPValue::ShortInt(number) => number.to_string(),
        AMQPValue::ShortUInt(number) => number.to_string(),
        AMQPValue::LongInt(number) => number.to_string(),
        AMQPValue::LongUInt(number) => number.to_string(),
        AMQPValue::LongLongInt(number) => number.to_string(),
        AMQPValue::Float(number) => number.to_string(),
        AMQPValue::Double(number) => number.to_string(),
        AMQPValue::Timestamp(ts) => ts.to_string(),
        AMQPValue::DecimalValue(decimal) => format!("{decimal:?}"),
        AMQPValue::ByteArray(bytes) => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.encode(bytes.as_slice())
        }
        AMQPValue::FieldArray(array) => {
            let parts: Vec<String> = array.as_slice().iter().map(amqp_value_to_string).collect();
            format!("[{}]", parts.join(", "))
        }
        AMQPValue::FieldTable(table) => format!("{table:?}"),
        AMQPValue::Void => "null".into(),
    }
}

#[async_trait]
impl Backend for RabbitBackend {
    async fn ping(&self) -> AppResult<()> {
        self.channel().await.map(|_| ())
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            kind: BackendKind::RabbitMq,
            server_version: if self.connection.status().connected() {
                Some("connected".into())
            } else {
                None
            },
            capabilities: Capabilities::for_kind(BackendKind::RabbitMq),
            default_container: self
                .configured_queues()
                .first()
                .map(|queue| format!("queue:{queue}")),
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        let channel = self.channel().await?;
        let mut out = Vec::new();

        for name in self.configured_queues() {
            // Passive declare: reports depth without creating or modifying.
            let declared = channel
                .queue_declare(
                    ShortString::from(name.clone()),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await;
            match declared {
                Ok(queue) => {
                    let mut meta = BTreeMap::new();
                    meta.insert(
                        "messages".into(),
                        queue.message_count().to_string(),
                    );
                    meta.insert(
                        "consumers".into(),
                        queue.consumer_count().to_string(),
                    );
                    out.push(Container {
                        id: format!("queue:{name}"),
                        name: name.clone(),
                        kind: ContainerKind::RabbitQueue,
                        meta,
                        size: Some(queue.message_count() as i64),
                    });
                }
                Err(err) => {
                    let mut meta = BTreeMap::new();
                    meta.insert("error".into(), err.to_string());
                    out.push(Container {
                        id: format!("queue:{name}"),
                        name: name.clone(),
                        kind: ContainerKind::RabbitQueue,
                        meta,
                        size: None,
                    });
                }
            }
        }

        // The default exchange always exists and is the usual way to publish
        // straight to a queue by name.
        out.push(Container {
            id: "exchange:".into(),
            name: "(default exchange)".into(),
            kind: ContainerKind::RabbitExchange,
            meta: BTreeMap::from([("type".into(), "direct".into())]),
            size: None,
        });

        Ok(out)
    }

    async fn entries(
        &self,
        container: &str,
        _cursor: Option<&str>,
        limit: usize,
        _search: Option<&str>,
    ) -> AppResult<EntryPage> {
        // An exchange is a routing rule, not a message store: it holds no
        // messages of its own, so browsing one yields an empty page rather than
        // an error. This keeps the generic "list whatever container you have"
        // flow working for every backend.
        if container.starts_with("exchange:") {
            return Ok(EntryPage {
                entries: Vec::new(),
                next_cursor: None,
                total: Some(0),
            });
        }

        let queue = parse_queue_container(container)?;
        let channel = self.channel().await?;
        let mut entries = Vec::new();
        let mut replay = Vec::new();

        for _ in 0..limit.clamp(1, 200) {
            let message = channel
                .basic_get(
                    ShortString::from(queue.clone()),
                    BasicGetOptions { no_ack: false },
                )
                .await
                .map_err(|e| map_amqp_error(&e))?;

            let Some(message) = message else { break };
            entries.push(delivery_to_entry(&queue, entries.len(), &message.delivery));
            replay.push(message.delivery);
        }

        // `nack(requeue)` returns a message to the head, so requeueing inside
        // the loop would hand the same message back forever. Take the whole
        // batch first, then restore it in reverse — which puts it back in the
        // original order and leaves the queue untouched.
        self.replay_messages(replay).await?;

        Ok(EntryPage {
            entries,
            next_cursor: None,
            total: None,
        })
    }

    async fn read_entry(&self, container: &str, entry: &str) -> AppResult<Entry> {
        let queue = parse_queue_container(container)?;
        let wanted: usize = entry
            .parse()
            .map_err(|_| AppError::invalid(format!("invalid message position {entry}")))?;

        // Walk the queue to the requested position, collecting everything we
        // pass so it can be put back afterwards.
        let channel = self.channel().await?;
        let mut found = None;
        let mut passed = Vec::new();

        for position in 0..=wanted.min(499) {
            let message = channel
                .basic_get(
                    ShortString::from(queue.clone()),
                    BasicGetOptions { no_ack: false },
                )
                .await
                .map_err(|e| map_amqp_error(&e))?;
            let Some(message) = message else { break };

            if position == wanted {
                found = Some(message.delivery);
                break;
            }
            passed.push(message.delivery);
        }

        let delivery = match found {
            Some(delivery) => delivery,
            None => {
                // Not found: everything we took must go back before returning.
                self.replay_messages(passed).await?;
                return Err(AppError::server(format!(
                    "message {entry} is no longer available in queue {queue}"
                )));
            }
        };

        let mut out = Entry::new(entry, format!("message {}", wanted + 1), "message");
        out.size = Some(delivery.data.len());
        out.meta.insert("queue".into(), queue.clone());
        out.meta
            .insert("exchange".into(), delivery.exchange.to_string());
        out.meta
            .insert("routing_key".into(), delivery.routing_key.to_string());
        if let Some(id) = delivery.properties.message_id().as_ref() {
            out.meta
                .insert("message_id".into(), id.as_str().to_string());
        }
        if let Some(ts) = delivery.properties.timestamp() {
            out.timestamp = chrono::DateTime::from_timestamp(*ts as i64, 0);
        }
        for (key, value) in headers_of(&delivery) {
            out.meta.insert(format!("h:{key}"), value);
        }
        out.payload = Some(Payload::from_bytes(&delivery.data, self.config.max_payload()));

        // Put everything back, including the message we read: inspecting a
        // queue must never consume messages that belong to other consumers.
        // The payload is already copied into the response, so nothing is lost.
        passed.push(delivery);
        self.replay_messages(passed).await?;

        Ok(out)
    }

    async fn write_entry(
        &self,
        container: &str,
        entry: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        // Writing to a queue means publishing through the default exchange with
        // the queue name as the routing key.
        let queue = parse_queue_container(container)?;
        let routing_key = if entry.is_empty() {
            queue
        } else {
            entry.to_string()
        };
        self.publish_inner("", &routing_key, payload, options).await
    }

    async fn delete_entry(&self, container: &str, entries: &[String]) -> AppResult<MutationResult> {
        let queue = parse_queue_container(container)?;
        let channel = self.channel().await?;
        let mut discarded = 0usize;

        for entry in entries {
            let Ok(wanted) = entry.parse::<usize>() else {
                continue;
            };
            let mut passed = Vec::new();
            let mut matched = false;

            for position in 0..=wanted.min(499) {
                let message = channel
                    .basic_get(
                        ShortString::from(queue.clone()),
                        BasicGetOptions { no_ack: false },
                    )
                    .await
                    .map_err(|e| map_amqp_error(&e))?;
                let Some(message) = message else { break };

                if position == wanted {
                    // `reject` without requeue is the AMQP way to discard.
                    message
                        .delivery
                        .reject(BasicRejectOptions { requeue: false })
                        .await
                        .map_err(|e| map_amqp_error(&e))?;
                    discarded += 1;
                    matched = true;
                    break;
                }
                passed.push(message.delivery);
            }

            // Everything inspected goes back exactly once, whether or not the
            // target was found.
            self.replay_messages(passed).await?;
            if !matched {
                return Err(AppError::server(format!(
                    "message {entry} is no longer available in queue {queue}"
                )));
            }
        }

        Ok(MutationResult {
            ok: true,
            message: Some(format!("discarded {discarded} message(s) from {queue}")),
            affected: entries.to_vec(),
        })
    }

    async fn publish(
        &self,
        container: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        // Accept either `queue:<name>` or `exchange:<name>` as the target.
        if let Ok(queue) = parse_queue_container(container) {
            let routing_key = options.routing_key.clone().unwrap_or(queue);
            return self
                .publish_inner(
                    options.exchange.as_deref().unwrap_or(""),
                    &routing_key,
                    payload,
                    options,
                )
                .await;
        }
        let exchange = container
            .strip_prefix("exchange:")
            .ok_or_else(|| AppError::invalid(format!("invalid container {container}")))?;
        let routing_key = options.routing_key.clone().unwrap_or_default();
        self.publish_inner(exchange, &routing_key, payload, options)
            .await
    }

    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        let queue = parse_queue_container(container)?;
        let channel = self.channel().await?;
        channel
            .basic_qos(50, BasicQosOptions::default())
            .await
            .map_err(|e| map_amqp_error(&e))?;

        let consumer = channel
            .basic_consume(
                ShortString::from(queue.clone()),
                ShortString::from(format!("cube-studio-{}", uuid::Uuid::new_v4())),
                BasicConsumeOptions {
                    no_ack: false,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await
            .map_err(|e| map_amqp_error(&e))?;

        let subscription_id = uuid::Uuid::new_v4().to_string();
        let id_for_task = subscription_id.clone();
        let queue_for_task = queue.clone();
        let (tx, rx) = mpsc::channel(512);

        tokio::spawn(async move {
            use futures_util::StreamExt;
            let mut stream = consumer;
            while let Some(delivery) = stream.next().await {
                let Ok(delivery) = delivery else { break };

                let message = StreamMessage {
                    subscription_id: id_for_task.clone(),
                    source: queue_for_task.clone(),
                    payload: Payload::from_bytes(&delivery.data, 512 * 1024),
                    key: Some(delivery.routing_key.to_string()),
                    headers: headers_of(&delivery),
                    timestamp: delivery
                        .properties
                        .timestamp()
                        .and_then(|ts| chrono::DateTime::from_timestamp(ts as i64, 0))
                        .unwrap_or_else(chrono::Utc::now),
                    position: Some(delivery.delivery_tag.to_string()),
                };

                if tx.send(message).await.is_err() {
                    // The UI stopped listening: put the message back so no data
                    // is lost, then stop the consumer.
                    let _ = delivery
                        .nack(BasicNackOptions {
                            requeue: true,
                            ..Default::default()
                        })
                        .await;
                    break;
                }
                let _ = delivery.ack(BasicAckOptions::default()).await;
            }
        });

        Ok(TailHandle {
            subscription_id,
            receiver: rx,
        })
    }

    async fn console(&self, command: &str) -> AppResult<ConsoleResult> {
        // AMQP has no textual console, so expose the administrative verbs that
        // map cleanly onto channel methods.
        let started = std::time::Instant::now();
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(AppError::invalid("empty command"));
        }

        let result = match parts[0].to_lowercase().as_str() {
            "queue_declare" | "declare" if parts.len() >= 2 => {
                let channel = self.channel().await?;
                channel
                    .queue_declare(
                        ShortString::from(parts[1].to_string()),
                        QueueDeclareOptions {
                            durable: true,
                            ..Default::default()
                        },
                        FieldTable::default(),
                    )
                    .await
                    .map(|queue| {
                        format!(
                            "declared queue {} ({} message(s), {} consumer(s))",
                            queue.name(),
                            queue.message_count(),
                            queue.consumer_count()
                        )
                    })
                    .map_err(|e| map_amqp_error(&e))
            }
            "queue_purge" | "purge" if parts.len() >= 2 => {
                let channel = self.channel().await?;
                channel
                    .queue_purge(
                        ShortString::from(parts[1].to_string()),
                        QueuePurgeOptions::default(),
                    )
                    .await
                    .map(|count| format!("purged {count} message(s) from {}", parts[1]))
                    .map_err(|e| map_amqp_error(&e))
            }
            "queue_delete" if parts.len() >= 2 => {
                let channel = self.channel().await?;
                channel
                    .queue_delete(
                        ShortString::from(parts[1].to_string()),
                        QueueDeleteOptions::default(),
                    )
                    .await
                    .map(|count| format!("deleted {}, dropping {count} message(s)", parts[1]))
                    .map_err(|e| map_amqp_error(&e))
            }
            "publish" if parts.len() >= 4 => {
                let body = parts[3..].join(" ");
                self.publish_inner(
                    parts[1],
                    parts[2],
                    body.as_bytes(),
                    &WriteOptions::default(),
                )
                .await
                .map(|result| result.message.unwrap_or_else(|| "published".into()))
            }
            _ => Err(AppError::invalid(
                "supported commands: queue_declare <name>, queue_purge <name>, \
                 queue_delete <name>, publish <exchange> <routing-key> <body>",
            )),
        };

        let elapsed_ms = started.elapsed().as_millis() as u64;
        Ok(match result {
            Ok(output) => ConsoleResult {
                output,
                error: false,
                elapsed_ms,
            },
            Err(err) => ConsoleResult {
                output: err.message,
                error: true,
                elapsed_ms,
            },
        })
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        let mut out = Vec::new();
        let channel = self.channel().await?;
        for name in self.configured_queues() {
            if let Ok(queue) = channel
                .queue_declare(
                    ShortString::from(name.clone()),
                    QueueDeclareOptions {
                        passive: true,
                        ..Default::default()
                    },
                    FieldTable::default(),
                )
                .await
            {
                out.push(StatItem {
                    section: "queues".into(),
                    key: name,
                    value: format!(
                        "{} ready, {} consumer(s)",
                        queue.message_count(),
                        queue.consumer_count()
                    ),
                });
            }
        }
        if out.is_empty() {
            out.push(StatItem {
                section: "connection".into(),
                key: "status".into(),
                value: if self.connection.status().connected() {
                    "connected".into()
                } else {
                    "disconnected".into()
                },
            });
        }
        Ok(out)
    }

    async fn flush(&self, container: &str) -> AppResult<MutationResult> {
        let queue = parse_queue_container(container)?;
        let channel = self.channel().await?;
        let purged = channel
            .queue_purge(
                ShortString::from(queue.clone()),
                QueuePurgeOptions::default(),
            )
            .await
            .map_err(|e| map_amqp_error(&e))?;
        Ok(MutationResult::with_message(format!(
            "purged {purged} message(s) from {queue}"
        )))
    }

    async fn shutdown(&self) {
        let _ = self
            .connection
            .close(200, ShortString::from("cube studio shutting down"))
            .await;
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

impl RabbitBackend {
    /// Put peeked messages back on the queue in their original order.
    ///
    /// AMQP has no "peek" primitive, so reading takes ownership of messages.
    /// Each message is returned with `nack(requeue)`, which per the AMQP spec
    /// places it back at the *head* of the queue. Requiring in reverse order
    /// therefore restores the original sequence exactly, and unlike republishing
    /// it preserves message identity (messages are not duplicated on the
    /// exchange, so no extra copies reach other bound queues).
    async fn replay_messages(&self, deliveries: Vec<Delivery>) -> AppResult<()> {
        for delivery in deliveries.into_iter().rev() {
            delivery
                .nack(BasicNackOptions {
                    requeue: true,
                    ..Default::default()
                })
                .await
                .map_err(|e| map_amqp_error(&e))?;
        }
        Ok(())
    }

    async fn publish_inner(
        &self,
        exchange: &str,
        routing_key: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        let channel = self.channel().await?;
        let mut properties = BasicProperties::default().with_delivery_mode(2);

        if let Some(headers) = options.headers.clone().filter(|h| !h.is_empty()) {
            let mut table = FieldTable::default();
            for (key, value) in headers {
                table.insert(ShortString::from(key), AMQPValue::LongString(value.into()));
            }
            properties = properties.with_headers(table);
        }

        channel
            .basic_publish(
                ShortString::from(exchange.to_string()),
                ShortString::from(routing_key.to_string()),
                BasicPublishOptions::default(),
                payload,
                properties,
            )
            .await
            .map_err(|e| map_amqp_error(&e))?
            .await
            .map_err(|e| map_amqp_error(&e))?;

        Ok(MutationResult::with_message(format!(
            "published {} byte(s) via exchange '{}' with routing key '{}'",
            payload.len(),
            if exchange.is_empty() {
                "(default)"
            } else {
                exchange
            },
            routing_key
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percent_encodes_uri_components() {
        assert_eq!(percent_encode("user"), "user");
        assert_eq!(percent_encode("p@ss word"), "p%40ss%20word");
        assert_eq!(percent_encode("/"), "%2F");
        assert_eq!(percent_encode("my/vhost"), "my%2Fvhost");
    }

    #[test]
    fn parses_queue_containers() {
        assert_eq!(parse_queue_container("queue:orders").unwrap(), "orders");
        assert!(parse_queue_container("exchange:amq.direct").is_err());
        assert!(parse_queue_container("queue:").is_err());
    }
}
