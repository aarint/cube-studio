//! MQTT driver.
//!
//! MQTT brokers keep no message history for ordinary topics. The only stored
//! payload is the single **retained** message per topic. The browsing model
//! therefore is:
//!
//! * container — a topic filter the user watches (e.g. `sensors/+/temp`)
//! * tail — a live subscription, which is the main working view
//! * publish — write to any topic, optionally retained
//!
//! To list retained messages we open a short-lived subscription to the filter:
//! the broker replays retained messages immediately after SUBACK and then stays
//! silent, so we read for a bounded window and disconnect.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS, TlsConfiguration, Transport};
use tokio::sync::{mpsc, Mutex};

use crate::backend::{Backend, ConnectionInfo, TailHandle};
use crate::config::{BackendKind, ConnectionConfig, TlsMode};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::model::{
    Capabilities, ConsoleResult, Container, ContainerKind, Entry, EntryPage, MutationResult,
    Payload, StatItem, StreamMessage, WriteOptions,
};

/// How long to wait for retained messages to be replayed.
const RETAINED_WINDOW: Duration = Duration::from_millis(1500);

/// Profile metadata key holding the comma separated topic filters to browse.
pub const TOPICS_META_KEY: &str = "mqtt_topics";

pub struct MqttBackend {
    config: ConnectionConfig,
    client: AsyncClient,
    /// Last transport error, surfaced in the stats panel.
    last_error: Arc<Mutex<Option<String>>>,
}

impl MqttBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let options = build_options(&config, None)?;
        let (client, mut eventloop) = AsyncClient::new(options, 128);

        // `AsyncClient::new` never touches the network: publish only enqueues.
        // Drive the event loop until CONNACK proves the broker accepted us, so a
        // dead endpoint fails here instead of silently on the first publish.
        let handshake = async {
            loop {
                match eventloop.poll().await {
                    Ok(rumqttc::Event::Incoming(Packet::ConnAck(_))) => return Ok(()),
                    Ok(_) => {}
                    Err(err) => {
                        return Err(AppError::driver(format!(
                            "MQTT handshake failed: {err}"
                        )))
                    }
                }
            }
        };
        tokio::time::timeout(config.timeout(), handshake)
            .await
            .map_err(|_| AppError::network("MQTT connection timed out"))??;

        let last_error = Arc::new(Mutex::new(None));
        let error_slot = last_error.clone();
        tokio::spawn(async move {
            loop {
                if let Err(err) = eventloop.poll().await {
                    let message = err.to_string();
                    tracing::warn!(error = %message, "mqtt event loop error");
                    *error_slot.lock().await = Some(message);
                    // Back off so a broken link does not spin the CPU.
                    tokio::time::sleep(Duration::from_secs(1)).await;
                }
            }
        });

        Ok(Self {
            config,
            client,
            last_error,
        })
    }

    /// Open a throwaway connection subscribed to `filter`, collect retained
    /// messages, then disconnect.
    async fn collect_retained(&self, filter: &str) -> AppResult<Vec<(String, Vec<u8>, bool)>> {
        let options = build_options(&self.config, Some("read"))?;
        let (client, mut eventloop) = AsyncClient::new(options, 64);

        client
            .subscribe(filter.to_string(), QoS::AtMostOnce)
            .await
            .map_err(|e| map_mqtt_error(&e))?;

        let deadline = tokio::time::Instant::now() + RETAINED_WINDOW;
        let mut messages = Vec::new();

        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            match tokio::time::timeout(remaining, eventloop.poll()).await {
                Ok(Ok(Event::Incoming(Packet::Publish(publish)))) => {
                    messages.push((publish.topic, publish.payload.to_vec(), publish.retain));
                }
                // A connack or any other packet: keep listening for retained
                // payloads until the window closes.
                Ok(Ok(_)) => {}
                Ok(Err(err)) => {
                    let _ = client.disconnect().await;
                    return Err(AppError::driver(format!(
                        "MQTT subscription to {filter} failed: {err}"
                    )));
                }
                Err(_) => break,
            }
        }

        let _ = client.disconnect().await;
        Ok(messages)
    }

    fn configured_filters(&self) -> Vec<String> {
        self.config
            .meta
            .get(TOPICS_META_KEY)
            .map(|raw| {
                raw.split(',')
                    .map(str::trim)
                    .filter(|filter| !filter.is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Build connection options, optionally overriding the client id so several
/// simultaneous connections from one session do not evict each other.
fn build_options(config: &ConnectionConfig, role: Option<&str>) -> AppResult<MqttOptions> {
    let endpoint = config.endpoints()?.remove(0);
    let client_id = match role {
        Some(role) => format!("cube-studio-{role}-{}", uuid::Uuid::new_v4()),
        None => config
            .client_id
            .clone()
            .filter(|id| !id.trim().is_empty())
            .unwrap_or_else(|| format!("cube-studio-{}", uuid::Uuid::new_v4())),
    };

    let mut options = MqttOptions::new(client_id, endpoint.host.clone(), endpoint.port);
    options.set_keep_alive(Duration::from_secs(
        config.keep_alive.unwrap_or(30).clamp(5, 3600) as u64,
    ));
    // A clean session means the broker discards our subscription state when we
    // disconnect, which is what an inspector needs.
    options.set_clean_session(true);
    if !config.username.is_empty() {
        options.set_credentials(config.username.clone(), config.password.clone());
    }
    if config.tls.is_enabled() {
        options.set_transport(Transport::Tls(build_tls(config)?));
    }
    Ok(options)
}

fn build_tls(config: &ConnectionConfig) -> AppResult<TlsConfiguration> {
    match config.tls {
        TlsMode::Disabled => Ok(TlsConfiguration::default()),
        TlsMode::Insecure => {
            // Build a rustls config that accepts any certificate. Used only when
            // the user explicitly disabled verification.
            let verifier = Arc::new(accept_any::NoVerifier);
            let client_config = rustls::ClientConfig::builder()
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth();
            Ok(TlsConfiguration::Rustls(Arc::new(client_config)))
        }
        TlsMode::Enabled => {
            if let Some(path) = config
                .tls_ca_file
                .as_ref()
                .filter(|path| !path.trim().is_empty())
            {
                let ca = std::fs::read(path).map_err(|e| {
                    AppError::invalid(format!("cannot read TLS CA file {path}: {e}"))
                })?;
                return Ok(TlsConfiguration::Simple {
                    ca,
                    alpn: None,
                    client_auth: None,
                });
            }
            // Without an explicit CA, rumqttc's default uses the webpki roots.
            Ok(TlsConfiguration::default())
        }
    }
}

fn map_mqtt_error(err: &rumqttc::ClientError) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();
    let kind = if lower.contains("not authorized") || lower.contains("bad username") {
        ErrorKind::Unauthorized
    } else if lower.contains("timeout") {
        ErrorKind::Timeout
    } else if lower.contains("connection") || lower.contains("io") {
        ErrorKind::Network
    } else {
        ErrorKind::ServerError
    };
    AppError::new(kind, message)
}

fn qos_from(value: Option<u8>) -> QoS {
    match value {
        Some(2) => QoS::ExactlyOnce,
        Some(1) => QoS::AtLeastOnce,
        _ => QoS::AtMostOnce,
    }
}

fn parse_topic_container(id: &str) -> AppResult<String> {
    // `topic:` is the canonical prefix, but a bare filter is accepted too since
    // users often type one directly.
    let topic = id.strip_prefix("topic:").unwrap_or(id);
    if topic.trim().is_empty() {
        return Err(AppError::invalid("topic filter is required"));
    }
    Ok(topic.to_string())
}

#[async_trait]
impl Backend for MqttBackend {
    async fn ping(&self) -> AppResult<()> {
        self.client
            .publish("cube-studio/$ping", QoS::AtMostOnce, false, b"ping".to_vec())
            .await
            .map_err(|e| map_mqtt_error(&e))
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        Ok(ConnectionInfo {
            kind: BackendKind::Mqtt,
            server_version: self
                .last_error
                .lock()
                .await
                .clone()
                .map(|error| format!("last error: {error}"))
                .or_else(|| Some("connected".into())),
            capabilities: Capabilities::for_kind(BackendKind::Mqtt),
            default_container: Some("topic:#".into()),
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        // MQTT has no topic discovery. Offer the wildcard plus the filters the
        // user saved on the profile.
        let mut out = vec![Container {
            id: "topic:#".into(),
            name: "# (all topics)".into(),
            kind: ContainerKind::MqttTopic,
            meta: BTreeMap::from([("filter".into(), "#".into())]),
            size: None,
        }];
        for filter in self.configured_filters() {
            out.push(Container {
                id: format!("topic:{filter}"),
                name: filter.clone(),
                kind: ContainerKind::MqttTopic,
                meta: BTreeMap::from([("filter".into(), filter)]),
                size: None,
            });
        }
        Ok(out)
    }

    async fn entries(
        &self,
        container: &str,
        _cursor: Option<&str>,
        limit: usize,
        search: Option<&str>,
    ) -> AppResult<EntryPage> {
        let filter = parse_topic_container(container)?;
        let needle = search.unwrap_or("").trim().to_lowercase();

        let mut entries: Vec<Entry> = self
            .collect_retained(&filter)
            .await?
            .into_iter()
            .filter(|(topic, _, _)| needle.is_empty() || topic.to_lowercase().contains(&needle))
            .map(|(topic, payload, retained)| {
                let mut entry = Entry::new(topic.clone(), topic, "message");
                entry.size = Some(payload.len());
                entry.meta.insert("retained".into(), retained.to_string());
                if let Some(preview) = Payload::from_bytes(&payload, 160).text {
                    entry
                        .meta
                        .insert("preview".into(), preview.replace(['\n', '\r'], " "));
                }
                entry
            })
            .collect();

        entries.sort_by(|a, b| a.label.cmp(&b.label));
        entries.truncate(limit.max(1));

        Ok(EntryPage {
            total: Some(entries.len() as i64),
            entries,
            next_cursor: None,
        })
    }

    async fn read_entry(&self, _container: &str, entry: &str) -> AppResult<Entry> {
        // `entry` is a concrete topic: read its retained payload.
        let messages = self.collect_retained(entry).await?;
        let Some((topic, payload, retained)) = messages.into_iter().next() else {
            return Err(AppError::server(format!(
                "topic {entry} has no retained message"
            )));
        };

        let mut out = Entry::new(topic.clone(), topic, "message");
        out.size = Some(payload.len());
        out.meta.insert("retained".into(), retained.to_string());
        out.payload = Some(Payload::from_bytes(&payload, self.config.max_payload()));
        Ok(out)
    }

    async fn write_entry(
        &self,
        container: &str,
        entry: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        // Writing to a topic means publishing with `retain`, otherwise the
        // message would vanish as soon as no subscriber is attached.
        let topic = if entry.is_empty() {
            parse_topic_container(container)?
        } else {
            entry.to_string()
        };
        let mut options = options.clone();
        options.retained = Some(true);
        self.publish(&format!("topic:{topic}"), payload, &options)
            .await
    }

    async fn delete_entry(&self, _container: &str, entries: &[String]) -> AppResult<MutationResult> {
        // Clearing a retained message means publishing a zero-length payload
        // with the retain flag set.
        for topic in entries {
            self.client
                .publish(topic.to_string(), QoS::AtMostOnce, true, Vec::new())
                .await
                .map_err(|e| map_mqtt_error(&e))?;
        }
        Ok(MutationResult {
            ok: true,
            message: Some(format!(
                "cleared retained message on {} topic(s)",
                entries.len()
            )),
            affected: entries.to_vec(),
        })
    }

    async fn publish(
        &self,
        container: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        let topic = match options.routing_key.clone().filter(|t| !t.trim().is_empty()) {
            Some(topic) => topic,
            None => parse_topic_container(container)?,
        };
        let qos = qos_from(options.qos);
        let retain = options.retained.unwrap_or(false);

        self.client
            .publish(topic.clone(), qos, retain, payload.to_vec())
            .await
            .map_err(|e| map_mqtt_error(&e))?;

        Ok(MutationResult::with_message(format!(
            "published {} byte(s) to {topic} (qos {qos:?}, retained {retain})",
            payload.len()
        )))
    }

    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        let filter = parse_topic_container(container)?;
        let options = build_options(&self.config, Some("tail"))?;
        let (client, mut eventloop) = AsyncClient::new(options, 128);

        client
            .subscribe(filter.clone(), QoS::AtMostOnce)
            .await
            .map_err(|e| map_mqtt_error(&e))?;

        let subscription_id = uuid::Uuid::new_v4().to_string();
        let id_for_task = subscription_id.clone();
        let (tx, rx) = mpsc::channel(512);

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let message = StreamMessage {
                            subscription_id: id_for_task.clone(),
                            source: publish.topic.clone(),
                            payload: Payload::from_bytes(&publish.payload, 512 * 1024),
                            key: None,
                            headers: if publish.retain {
                                BTreeMap::from([("retained".to_string(), "true".to_string())])
                            } else {
                                BTreeMap::new()
                            },
                            timestamp: chrono::Utc::now(),
                            position: Some(format!("{:?}", publish.qos)),
                        };
                        if tx.send(message).await.is_err() {
                            break;
                        }
                    }
                    Ok(_) => {}
                    Err(err) => {
                        tracing::warn!(error = %err, "mqtt tail loop error");
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            }
            let _ = client.disconnect().await;
        });

        Ok(TailHandle {
            subscription_id,
            receiver: rx,
        })
    }

    async fn console(&self, command: &str) -> AppResult<ConsoleResult> {
        // `<topic> <payload>` publishes once. This is the whole MQTT verb set a
        // console could usefully expose.
        let started = std::time::Instant::now();
        let trimmed = command.trim();
        if trimmed.is_empty() {
            return Err(AppError::invalid(
                "usage: <topic> <payload>  — publish a message",
            ));
        }
        let (topic, payload) = match trimmed.split_once(' ') {
            Some((topic, rest)) => (topic.trim(), rest),
            None => (trimmed, ""),
        };

        let result = self
            .client
            .publish(
                topic.to_string(),
                QoS::AtLeastOnce,
                false,
                payload.as_bytes().to_vec(),
            )
            .await;
        let elapsed_ms = started.elapsed().as_millis() as u64;

        Ok(match result {
            Ok(()) => ConsoleResult {
                output: format!("published {} byte(s) to {topic}", payload.len()),
                error: false,
                elapsed_ms,
            },
            Err(err) => ConsoleResult {
                output: map_mqtt_error(&err).message,
                error: true,
                elapsed_ms,
            },
        })
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        let mut out = vec![
            StatItem {
                section: "connection".into(),
                key: "broker".into(),
                value: format!("{}:{}", self.config.host, self.config.port),
            },
            StatItem {
                section: "connection".into(),
                key: "tls".into(),
                value: format!("{:?}", self.config.tls),
            },
        ];
        if let Some(error) = self.last_error.lock().await.clone() {
            out.push(StatItem {
                section: "connection".into(),
                key: "last_error".into(),
                value: error,
            });
        }
        Ok(out)
    }

    async fn shutdown(&self) {
        let _ = self.client.disconnect().await;
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

/// rustls verifiers that accept every certificate, for `insecure` TLS mode.
mod accept_any {
    use std::sync::Arc;

    use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
    use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
    use rustls::{DigitallySignedStruct, Error as RustlsError, SignatureScheme};

    /// Accepts any server certificate.
    ///
    /// Only reachable through an explicit "skip verification" setting; the UI
    /// warns about it. Used so a developer can inspect a self-signed broker.
    #[derive(Debug)]
    pub struct NoVerifier;

    impl ServerCertVerifier for NoVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, RustlsError> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, RustlsError> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::RSA_PKCS1_SHA384,
                SignatureScheme::RSA_PKCS1_SHA512,
                SignatureScheme::ECDSA_NISTP256_SHA256,
                SignatureScheme::ECDSA_NISTP384_SHA384,
                SignatureScheme::ECDSA_NISTP521_SHA512,
                SignatureScheme::ED25519,
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PSS_SHA384,
                SignatureScheme::RSA_PSS_SHA512,
            ]
        }
    }

    /// Silences an unused-import warning when the module is compiled without
    /// any TLS backend enabled.
    #[allow(dead_code)]
    pub type Unused = Arc<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_topic_containers() {
        assert_eq!(parse_topic_container("topic:sensors/+").unwrap(), "sensors/+");
        assert_eq!(parse_topic_container("sensors/#").unwrap(), "sensors/#");
        assert!(parse_topic_container("topic:").is_err());
        assert!(parse_topic_container("   ").is_err());
    }

    #[test]
    fn maps_qos_values() {
        assert_eq!(qos_from(Some(0)), QoS::AtMostOnce);
        assert_eq!(qos_from(Some(1)), QoS::AtLeastOnce);
        assert_eq!(qos_from(Some(2)), QoS::ExactlyOnce);
        assert_eq!(qos_from(Some(9)), QoS::AtMostOnce);
        assert_eq!(qos_from(None), QoS::AtMostOnce);
    }
}
