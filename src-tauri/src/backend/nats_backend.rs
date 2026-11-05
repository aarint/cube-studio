//! NATS driver.
//!
//! Two modes, detected at connect time:
//!
//! * **JetStream enabled** — streams are browsable containers and stored
//!   messages can be listed and read by sequence number.
//! * **Core NATS** — there is no message history by design, so the UI shows a
//!   live tail instead and publishing is fire-and-forget.
//!
//! Subjects are the unit of everything: publishing targets a subject, tailing
//! subscribes to one (wildcards `*` and `>` work as at the server).

use std::collections::BTreeMap;

use async_trait::async_trait;
use async_nats::jetstream::Context as JetStream;
use async_nats::{Client as NatsClient, ConnectOptions, Subject};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::backend::{Backend, ConnectionInfo, TailHandle};
use crate::config::{BackendKind, ConnectionConfig};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::model::{
    Capabilities, ConsoleResult, Container, ContainerKind, Entry, EntryPage, MutationResult,
    Payload, StatItem, StreamMessage, WriteOptions,
};

pub struct NatsBackend {
    config: ConnectionConfig,
    client: NatsClient,
    /// True when the server answered a JetStream account query.
    jetstream: bool,
}

impl NatsBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let endpoint = config.endpoints()?.remove(0);
        let scheme = if config.tls.is_enabled() {
            "tls://"
        } else {
            "nats://"
        };
        let url = format!("{scheme}{}", endpoint.authority());

        let mut options = ConnectOptions::new()
            .name(format!("cube-studio-{}", uuid::Uuid::new_v4()))
            .connection_timeout(config.timeout());
        if !config.username.is_empty() || !config.password.is_empty() {
            options = options.user_and_password(config.username.clone(), config.password.clone());
        } else if let Some(token) = config.token.clone().filter(|t| !t.is_empty()) {
            options = options.token(token);
        }
        if config.tls.is_enabled() {
            options = options.require_tls(true);
        }

        let client =
            tokio::time::timeout(config.timeout(), async_nats::connect_with_options(url, options))
                .await
                .map_err(|_| AppError::network("NATS connection timed out"))?
                .map_err(|e| map_nats_error(&e))?;

        // Probing JetStream is the only reliable way to know whether a broker
        // has it enabled, and it decides the whole browsing model.
        let jetstream = async_nats::jetstream::new(client.clone())
            .query_account()
            .await
            .is_ok();

        Ok(Self {
            config,
            client,
            jetstream,
        })
    }

    fn jetstream(&self) -> JetStream {
        async_nats::jetstream::new(self.client.clone())
    }
}

fn map_nats_error(err: &async_nats::ConnectError) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();
    let kind = if lower.contains("authorization") || lower.contains("authentication") {
        ErrorKind::Unauthorized
    } else if lower.contains("timed out") || lower.contains("timeout") {
        ErrorKind::Timeout
    } else if lower.contains("dns") || lower.contains("connect") || lower.contains("tls") {
        ErrorKind::Network
    } else {
        ErrorKind::ServerError
    };
    AppError::new(kind, message)
}

fn map_jetstream_error(message: impl std::fmt::Display) -> AppError {
    let message = message.to_string();
    let lower = message.to_lowercase();
    let kind = if lower.contains("not found") || lower.contains("404") {
        ErrorKind::InvalidInput
    } else if lower.contains("authorization") || lower.contains("permission") {
        ErrorKind::Unauthorized
    } else {
        ErrorKind::ServerError
    };
    AppError::new(kind, message)
}

/// Convert the JetStream message timestamp (a `time::OffsetDateTime`) into the
/// chrono type used across the IPC boundary.
fn offset_to_chrono(value: time::OffsetDateTime) -> Option<chrono::DateTime<chrono::Utc>> {
    let nanos = value.unix_timestamp_nanos();
    chrono::DateTime::from_timestamp_nanos(nanos as i64).into()
}

/// Container ids are `stream:<name>` or `subject:<pattern>`.
fn parse_container(id: &str) -> (&str, &str) {
    match id.split_once(':') {
        Some((kind, rest)) => (kind, rest),
        None => ("stream", id),
    }
}

#[async_trait]
impl Backend for NatsBackend {
    async fn ping(&self) -> AppResult<()> {
        // `flush` round-trips a PING, proving the socket is usable.
        self.client
            .flush()
            .await
            .map_err(|e| AppError::driver(e.to_string()))
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        let mut capabilities = Capabilities::for_kind(BackendKind::Nats);
        if self.jetstream {
            capabilities.containers = true;
            capabilities.list_entries = true;
            capabilities.stats = true;
        } else {
            capabilities.containers = false;
            capabilities.list_entries = false;
            capabilities.stats = false;
            capabilities.notes.push(
                "This broker has JetStream disabled, so there is no message history. \
                 Use the tail panel to watch a subject live."
                    .into(),
            );
        }
        Ok(ConnectionInfo {
            kind: BackendKind::Nats,
            server_version: Some(if self.jetstream {
                "JetStream".into()
            } else {
                "core NATS".into()
            }),
            capabilities,
            default_container: None,
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        let mut out = Vec::new();

        if self.jetstream {
            let context = self.jetstream();
            let mut streams = context.streams();
            while let Some(item) = streams.next().await {
                let info = item.map_err(map_jetstream_error)?;
                let mut meta = BTreeMap::new();
                meta.insert("subjects".into(), info.config.subjects.join(", "));
                meta.insert("storage".into(), format!("{:?}", info.config.storage));
                meta.insert("retention".into(), format!("{:?}", info.config.retention));
                out.push(Container {
                    id: format!("stream:{}", info.config.name),
                    name: info.config.name.clone(),
                    kind: ContainerKind::NatsStream,
                    meta,
                    size: Some(info.state.messages as i64),
                });
            }
        }

        // Ad-hoc subject tailing is always offered.
        out.push(Container {
            id: "subject:>".into(),
            name: "> (live subject)".into(),
            kind: ContainerKind::NatsSubject,
            meta: BTreeMap::new(),
            size: None,
        });

        Ok(out)
    }

    async fn entries(
        &self,
        container: &str,
        cursor: Option<&str>,
        limit: usize,
        _search: Option<&str>,
    ) -> AppResult<EntryPage> {
        let (kind, name) = parse_container(container);
        if kind != "stream" {
            // A subject container exists so the user can tail and publish to it,
            // but core NATS keeps no history. Returning an empty page (rather
            // than an error) matches what the UI shows for "nothing stored yet".
            return Ok(EntryPage {
                entries: Vec::new(),
                next_cursor: None,
                total: Some(0),
            });
        }

        let mut stream = self
            .jetstream()
            .get_stream(name)
            .await
            .map_err(map_jetstream_error)?;
        let info = stream.info().await.map_err(map_jetstream_error)?;
        let first = info.state.first_sequence;
        let last = info.state.last_sequence;
        let message_count = info.state.messages as i64;

        if message_count == 0 || last < first {
            return Ok(EntryPage {
                entries: Vec::new(),
                next_cursor: None,
                total: Some(0),
            });
        }

        // Newest first: the cursor is the sequence to continue *below*.
        let start = match cursor.and_then(|c| c.parse::<u64>().ok()) {
            Some(sequence) => sequence.min(last),
            None => last,
        };
        let window = limit.clamp(1, 200) as u64;
        let from = start.saturating_sub(window - 1).max(first);

        let mut entries = Vec::new();
        for sequence in (from..=start).rev() {
            // A gap is normal: retention may have removed a payload while the
            // sequence number space stays intact.
            let Ok(message) = stream.get_raw_message(sequence).await else {
                continue;
            };
            let mut entry = Entry::new(
                format!("stream:{name}:{sequence}"),
                format!("sequence {sequence}"),
                "message",
            );
            entry.size = Some(message.payload.len());
            entry.timestamp = offset_to_chrono(message.time);
            entry.meta.insert("sequence".into(), sequence.to_string());
            entry
                .meta
                .insert("subject".into(), message.subject.to_string());
            if let Some(preview) = Payload::from_bytes(&message.payload, 160).text {
                entry
                    .meta
                    .insert("preview".into(), preview.replace(['\n', '\r'], " "));
            }
            entries.push(entry);
        }

        Ok(EntryPage {
            entries,
            next_cursor: (from > first).then(|| from.to_string()),
            total: Some(message_count),
        })
    }

    async fn read_entry(&self, container: &str, entry: &str) -> AppResult<Entry> {
        let (kind, name) = parse_container(container);
        if kind != "stream" {
            return Err(AppError::unsupported(
                "only JetStream streams keep readable history; use the tail panel \
                 for live subjects",
            ));
        }
        let sequence = entry
            .rsplit(':')
            .next()
            .and_then(|s| s.parse::<u64>().ok())
            .ok_or_else(|| AppError::invalid(format!("invalid message id {entry}")))?;

        let stream = self
            .jetstream()
            .get_stream(name)
            .await
            .map_err(map_jetstream_error)?;
        let message = stream
            .get_raw_message(sequence)
            .await
            .map_err(map_jetstream_error)?;

        let mut out = Entry::new(entry, format!("sequence {sequence}"), "message");
        out.size = Some(message.payload.len());
        out.timestamp = offset_to_chrono(message.time);
        out.meta.insert("sequence".into(), sequence.to_string());
        out.meta
            .insert("subject".into(), message.subject.to_string());
        for (key, values) in message.headers.iter() {
            let joined: Vec<String> = values.iter().map(|v| v.to_string()).collect();
            out.meta.insert(format!("h:{key}"), joined.join(", "));
        }
        out.payload = Some(Payload::from_bytes(
            &message.payload,
            self.config.max_payload(),
        ));
        Ok(out)
    }

    async fn write_entry(
        &self,
        _container: &str,
        _entry: &str,
        _payload: &[u8],
        _options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        Err(AppError::unsupported(
            "NATS messages are published to a subject; use publish instead",
        ))
    }

    async fn delete_entry(
        &self,
        _container: &str,
        _entries: &[String],
    ) -> AppResult<MutationResult> {
        Err(AppError::unsupported(
            "individual NATS messages cannot be deleted; purge or trim the stream instead",
        ))
    }

    async fn publish(
        &self,
        container: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        // The subject is an explicit routing key when publishing into a stream,
        // otherwise the container's own name.
        let (kind, name) = parse_container(container);
        let subject_text = options
            .routing_key
            .clone()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| name.to_string());
        if kind == "stream" && subject_text == name {
            return Err(AppError::invalid(
                "publishing into a JetStream stream requires the target subject \
                 (set it as the routing key)",
            ));
        }

        let subject: Subject = subject_text
            .clone()
            .try_into()
            .map_err(|e| AppError::invalid(format!("invalid NATS subject {subject_text}: {e}")))?;

        match self
            .client
            .publish(subject.clone(), payload.to_vec().into())
            .await
        {
            Ok(()) => {}
            Err(err) => return Err(AppError::driver(err.to_string())),
        }
        self.client
            .flush()
            .await
            .map_err(|e| AppError::driver(e.to_string()))?;

        Ok(MutationResult::with_message(format!(
            "published {} byte(s) to {subject}",
            payload.len()
        )))
    }

    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        let (kind, name) = parse_container(container);
        let subject_text = match kind {
            "subject" => name.to_string(),
            "stream" => {
                // Tailing a stream means watching all of its subjects. Use the
                // first configured subject; `>` style streams are widened below.
                let mut stream = self
                    .jetstream()
                    .get_stream(name)
                    .await
                    .map_err(map_jetstream_error)?;
                let info = stream.info().await.map_err(map_jetstream_error)?;
                info.config
                    .subjects
                    .first()
                    .cloned()
                    .ok_or_else(|| AppError::invalid(format!("stream {name} has no subjects")))?
            }
            other => {
                return Err(AppError::invalid(format!(
                    "cannot tail a {other} container"
                )))
            }
        };

        let subject: Subject = subject_text
            .clone()
            .try_into()
            .map_err(|e| AppError::invalid(format!("invalid NATS subject {subject_text}: {e}")))?;

        let mut subscriber = self
            .client
            .subscribe(subject.clone())
            .await
            .map_err(|e| AppError::driver(e.to_string()))?;

        let subscription_id = uuid::Uuid::new_v4().to_string();
        let id_for_task = subscription_id.clone();
        let (tx, rx) = mpsc::channel(512);

        tokio::spawn(async move {
            while let Some(message) = subscriber.next().await {
                let stream_message = StreamMessage {
                    subscription_id: id_for_task.clone(),
                    source: message.subject.to_string(),
                    payload: Payload::from_bytes(&message.payload, 512 * 1024),
                    key: None,
                    headers: message
                        .headers
                        .as_ref()
                        .map(|headers| {
                            headers
                                .iter()
                                .map(|(key, values)| {
                                    let joined: Vec<String> =
                                        values.iter().map(|v| v.to_string()).collect();
                                    (key.to_string(), joined.join(", "))
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                    timestamp: chrono::Utc::now(),
                    position: None,
                };
                if tx.send(stream_message).await.is_err() {
                    break;
                }
            }
        });

        Ok(TailHandle {
            subscription_id,
            receiver: rx,
        })
    }

    async fn console(&self, command: &str) -> AppResult<ConsoleResult> {
        // NATS request/reply is the natural console: send to a subject and wait
        // for one reply.
        let started = std::time::Instant::now();
        let mut parts = command.splitn(2, ' ');
        let subject_text = parts.next().unwrap_or("").trim().to_string();
        let body = parts.next().unwrap_or("").as_bytes().to_vec();
        if subject_text.is_empty() {
            return Err(AppError::invalid(
                "usage: <subject> [payload]  — send a request and wait for one reply",
            ));
        }
        let subject: Subject = subject_text
            .clone()
            .try_into()
            .map_err(|e| AppError::invalid(format!("invalid subject {subject_text}: {e}")))?;

        let result = tokio::time::timeout(
            self.config.timeout(),
            self.client.request(subject, body.into()),
        )
        .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        Ok(match result {
            Ok(Ok(message)) => ConsoleResult {
                output: String::from_utf8_lossy(&message.payload).to_string(),
                error: false,
                elapsed_ms,
            },
            Ok(Err(err)) => ConsoleResult {
                output: err.to_string(),
                error: true,
                elapsed_ms,
            },
            Err(_) => ConsoleResult {
                output: "no reply within the configured timeout".into(),
                error: true,
                elapsed_ms,
            },
        })
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        if !self.jetstream {
            return Err(AppError::unsupported(
                "JetStream is disabled on this broker, so there is no account information",
            ));
        }
        let account = self
            .jetstream()
            .query_account()
            .await
            .map_err(map_jetstream_error)?;

        let mut out = Vec::new();
        let mut push = |key: &str, value: String| {
            out.push(StatItem {
                section: "jetstream".into(),
                key: key.to_string(),
                value,
            })
        };
        push("memory", format!("{} bytes", account.memory));
        push("storage", format!("{} bytes", account.storage));
        push("streams", account.streams.to_string());
        push("consumers", account.consumers.to_string());
        push(
            "max_memory",
            account
                .limits
                .max_memory
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unlimited".into()),
        );
        push(
            "max_storage",
            account
                .limits
                .max_storage
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unlimited".into()),
        );
        push(
            "max_streams",
            account
                .limits
                .max_streams
                .map(|v| v.to_string())
                .unwrap_or_else(|| "unlimited".into()),
        );
        Ok(out)
    }

    async fn shutdown(&self) {
        let _ = self.client.flush().await;
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_container_ids() {
        assert_eq!(parse_container("stream:orders"), ("stream", "orders"));
        assert_eq!(parse_container("subject:orders.*"), ("subject", "orders.*"));
        assert_eq!(parse_container("plain"), ("stream", "plain"));
    }
}
