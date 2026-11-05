//! Kafka driver.
//!
//! Built on `rskafka`, a pure-Rust client that speaks the Kafka wire protocol
//! without the Java/librdkafka dependency chain — important for a desktop app
//! that has to ship a single small binary.
//!
//! Browsing model: one container per topic. Entries are messages identified by
//! `partition:offset`, paged backwards from the high watermark so the newest
//! messages appear first, which is what an operator usually wants.

use std::collections::BTreeMap;
use std::sync::Arc;

use async_trait::async_trait;
use rskafka::client::partition::{Compression, OffsetAt, PartitionClient, UnknownTopicHandling};
use rskafka::client::{Client, ClientBuilder, Credentials, SaslConfig};
use rskafka::record::Record;
use tokio::sync::mpsc;

use crate::backend::{Backend, ConnectionInfo, TailHandle};
use crate::config::{BackendKind, ConnectionConfig, TlsMode};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::model::{
    Capabilities, Container, ContainerKind, Entry, EntryPage, MutationResult, Payload, StatItem,
    StreamMessage, WriteOptions,
};

/// How many messages to pull from the broker per page request.
const FETCH_BYTES: std::ops::Range<i32> = 0..4 * 1024 * 1024;

pub struct KafkaBackend {
    config: ConnectionConfig,
    client: Arc<Client>,
    /// Cached partition counts per topic, so paging does not re-query metadata.
    topics: tokio::sync::Mutex<BTreeMap<String, Vec<i32>>>,
}

impl KafkaBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let brokers: Vec<String> = config
            .endpoints()?
            .iter()
            .map(|endpoint| endpoint.authority())
            .collect();

        let mut builder = ClientBuilder::new(brokers);

        // SASL only when the user asked for it: brokers commonly run PLAINTEXT
        // on a trusted network and sending credentials would just fail.
        let mechanism = config.kafka_sasl_mechanism.as_str();
        if mechanism != "none" && !mechanism.is_empty() {
            if config.username.is_empty() {
                return Err(AppError::invalid(format!(
                    "SASL mechanism {mechanism} requires a username"
                )));
            }
            let credentials = Credentials::new(config.username.clone(), config.password.clone());
            let sasl = match mechanism {
                "plain" => SaslConfig::Plain(credentials),
                "scram-sha-256" | "scram_sha_256" => SaslConfig::ScramSha256(credentials),
                "scram-sha-512" | "scram_sha_512" => SaslConfig::ScramSha512(credentials),
                other => {
                    return Err(AppError::invalid(format!(
                        "unsupported Kafka SASL mechanism: {other}"
                    )))
                }
            };
            builder = builder.sasl_config(sasl);
        }

        if matches!(config.tls, TlsMode::Enabled | TlsMode::Insecure) {
            // rskafka builds on rustls. A permissive verifier is only used when
            // the user explicitly opted into skipping verification.
            let tls = if matches!(config.tls, TlsMode::Insecure) || config.tls_skip_verify {
                insecure_rustls_config()?
            } else if let Some(ca_file) = config
                .tls_ca_file
                .as_ref()
                .filter(|p| !p.trim().is_empty())
            {
                let pem = std::fs::read(ca_file).map_err(|e| {
                    AppError::invalid(format!("cannot read TLS CA file {ca_file}: {e}"))
                })?;
                tls_config_with_ca(&pem)?
            } else {
                default_rustls_config()?
            };
            builder = builder.tls_config(Arc::new(tls));
        }

        if let Some(group) = config.kafka_group_id.as_ref().filter(|g| !g.is_empty()) {
            builder = builder.client_id(group.clone());
        }

        let client = builder
            .build()
            .await
            .map_err(|e| AppError::driver(format!("cannot build Kafka client: {e}")))?;

        let backend = Self {
            config,
            client: Arc::new(client),
            topics: tokio::sync::Mutex::new(BTreeMap::new()),
        };
        backend.ping().await?;
        Ok(backend)
    }

    /// Refresh the topic -> partitions cache.
    async fn refresh_topics(&self) -> AppResult<BTreeMap<String, Vec<i32>>> {
        let topics = self
            .client
            .list_topics()
            .await
            .map_err(|e| map_kafka_error(&e))?;
        let map: BTreeMap<String, Vec<i32>> = topics
            .into_iter()
            .map(|topic| (topic.name, topic.partitions.into_iter().collect()))
            .collect();
        *self.topics.lock().await = map.clone();
        Ok(map)
    }

    async fn partitions_of(&self, topic: &str) -> AppResult<Vec<i32>> {
        if let Some(partitions) = self.topics.lock().await.get(topic).cloned() {
            return Ok(partitions);
        }
        let map = self.refresh_topics().await?;
        map.get(topic)
            .cloned()
            .ok_or_else(|| AppError::invalid(format!("unknown Kafka topic {topic}")))
    }

    async fn partition_client(&self, topic: &str, partition: i32) -> AppResult<PartitionClient> {
        self.client
            .partition_client(topic, partition, UnknownTopicHandling::Retry)
            .await
            .map_err(|e| map_kafka_error(&e))
    }
}

/// Standard verifying rustls configuration using the platform trust store.
fn default_rustls_config() -> AppResult<rustls::ClientConfig> {
    let mut root_store = rustls::RootCertStore::empty();
    for cert in rustls_native_certs::load_native_certs().certs {
        let _ = root_store.add(cert);
    }
    // An explicitly supplied CA bundle supplements the platform store, which is
    // the common case for an internal cluster with a private CA.
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

/// Configuration that trusts a specific CA bundle instead of the platform.
pub fn tls_config_with_ca(ca_pem: &[u8]) -> AppResult<rustls::ClientConfig> {
    let mut root_store = rustls::RootCertStore::empty();
    let mut reader = std::io::BufReader::new(ca_pem);
    let certs = rustls_pemfile::certs(&mut reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| AppError::invalid(format!("cannot parse CA bundle: {e}")))?;
    if certs.is_empty() {
        return Err(AppError::invalid("the CA bundle contains no certificates"));
    }
    for cert in certs {
        root_store
            .add(cert)
            .map_err(|e| AppError::invalid(format!("unusable CA certificate: {e}")))?;
    }
    Ok(rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth())
}

/// Accept-anything configuration, for self-signed dev brokers only.
fn insecure_rustls_config() -> AppResult<rustls::ClientConfig> {
    #[derive(Debug)]
    struct NoVerify;

    impl rustls::client::danger::ServerCertVerifier for NoVerify {
        fn verify_server_cert(
            &self,
            _end_entity: &rustls::pki_types::CertificateDer<'_>,
            _intermediates: &[rustls::pki_types::CertificateDer<'_>],
            _server_name: &rustls::pki_types::ServerName<'_>,
            _ocsp: &[u8],
            _now: rustls::pki_types::UnixTime,
        ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
            Ok(rustls::client::danger::ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &rustls::pki_types::CertificateDer<'_>,
            _dss: &rustls::DigitallySignedStruct,
        ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
            Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
            vec![
                rustls::SignatureScheme::RSA_PKCS1_SHA256,
                rustls::SignatureScheme::RSA_PKCS1_SHA384,
                rustls::SignatureScheme::RSA_PKCS1_SHA512,
                rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
                rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
                rustls::SignatureScheme::ED25519,
                rustls::SignatureScheme::RSA_PSS_SHA256,
                rustls::SignatureScheme::RSA_PSS_SHA384,
                rustls::SignatureScheme::RSA_PSS_SHA512,
            ]
        }
    }

    Ok(rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify))
        .with_no_client_auth())
}

fn map_kafka_error(err: &rskafka::client::error::Error) -> AppError {
    let message = err.to_string();
    let lower = message.to_lowercase();
    let kind = if lower.contains("authentication") || lower.contains("sasl") {
        ErrorKind::Unauthorized
    } else if lower.contains("timed out") || lower.contains("timeout") {
        ErrorKind::Timeout
    } else if lower.contains("connection") || lower.contains("dns") || lower.contains("tls") {
        ErrorKind::Network
    } else if lower.contains("unknown topic") {
        ErrorKind::InvalidInput
    } else {
        ErrorKind::ServerError
    };
    AppError::new(kind, message)
}

/// Split `partition:offset` entry ids.
fn parse_offset_id(id: &str) -> AppResult<(i32, i64)> {
    let (partition, offset) = id
        .split_once(':')
        .ok_or_else(|| AppError::invalid(format!("invalid message id {id}")))?;
    Ok((
        partition
            .parse()
            .map_err(|_| AppError::invalid(format!("invalid partition in {id}")))?,
        offset
            .parse()
            .map_err(|_| AppError::invalid(format!("invalid offset in {id}")))?,
    ))
}

#[async_trait]
impl Backend for KafkaBackend {
    async fn ping(&self) -> AppResult<()> {
        self.refresh_topics().await.map(|_| ())
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        // The broker API version is not surfaced by rskafka; the topic count is
        // a useful shape indicator instead.
        let topics = self.refresh_topics().await.map(|m| m.len()).unwrap_or(0);
        Ok(ConnectionInfo {
            kind: BackendKind::Kafka,
            server_version: Some(format!("{topics} topic(s)")),
            capabilities: Capabilities::for_kind(BackendKind::Kafka),
            default_container: None,
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        let topics = self.refresh_topics().await?;
        let mut out = Vec::with_capacity(topics.len());
        for (name, partitions) in topics {
            let mut meta = BTreeMap::new();
            meta.insert("partitions".to_string(), partitions.len().to_string());
            // Total message count across partitions is not cheap to compute, so
            // report only the partition count.
            out.push(Container {
                id: name.clone(),
                name,
                kind: ContainerKind::KafkaTopic,
                meta,
                size: None,
            });
        }
        Ok(out)
    }

    async fn entries(
        &self,
        container: &str,
        cursor: Option<&str>,
        limit: usize,
        _search: Option<&str>,
    ) -> AppResult<EntryPage> {
        let partitions = self.partitions_of(container).await?;
        if partitions.is_empty() {
            return Ok(EntryPage::single(Vec::new()));
        }

        // The cursor encodes `partition:startOffset`, where startOffset is
        // exclusive (we scan backwards).
        let (partition, start_offset) = match cursor {
            Some(raw) => parse_offset_id(raw)?,
            None => {
                // Start from the newest message of partition 0.
                let partition = partitions[0];
                let client = self.partition_client(container, partition).await?;
                let high = client
                    .get_offset(OffsetAt::Latest)
                    .await
                    .map_err(|e| map_kafka_error(&e))?;
                (partition, high)
            }
        };

        let client = self.partition_client(container, partition).await?;
        let low = client
            .get_offset(OffsetAt::Earliest)
            .await
            .map_err(|e| map_kafka_error(&e))?;

        let mut entries = Vec::new();
        let mut next_cursor = None;

        if start_offset > low {
            // Read a window ending at `start_offset - 1`.
            let window = (limit.clamp(1, 500) as i64).max(1);
            let from = (start_offset - window).max(low);
            let (records, _high) = client
                .fetch_records(from, FETCH_BYTES, 0)
                .await
                .map_err(|e| map_kafka_error(&e))?;

            for item in records.into_iter().rev() {
                if item.offset >= start_offset {
                    continue;
                }
                let mut entry = Entry::new(
                    format!("{partition}:{}", item.offset),
                    format!("offset {}", item.offset),
                    "message",
                );
                entry.size = item.record.value.as_ref().map(|v| v.len());
                entry.timestamp = Some(item.record.timestamp);
                entry.meta.insert("partition".into(), partition.to_string());
                entry.meta.insert("offset".into(), item.offset.to_string());
                if let Some(key) = item.record.key.as_ref() {
                    entry
                        .meta
                        .insert("key".into(), String::from_utf8_lossy(key).to_string());
                }
                for (header, value) in &item.record.headers {
                    entry
                        .meta
                        .insert(format!("h:{header}"), String::from_utf8_lossy(value).to_string());
                }
                // Preview the first bytes so the list is informative.
                if let Some(value) = item.record.value.as_ref() {
                    let preview = Payload::from_bytes(value, 160);
                    if let Some(text) = preview.text {
                        entry
                            .meta
                            .insert("preview".into(), text.replace(['\n', '\r'], " "));
                    }
                }
                entries.push(entry);
                if entries.len() >= limit {
                    break;
                }
            }

            if from > low {
                next_cursor = Some(format!("{partition}:{from}"));
            }
        }

        // When this partition is exhausted, move to the next one.
        if next_cursor.is_none() {
            if let Some(position) = partitions.iter().position(|p| *p == partition) {
                if let Some(next_partition) = partitions.get(position + 1) {
                    let client = self.partition_client(container, *next_partition).await?;
                    let high = client
                        .get_offset(OffsetAt::Latest)
                        .await
                        .map_err(|e| map_kafka_error(&e))?;
                    if high > 0 {
                        next_cursor = Some(format!("{next_partition}:{high}"));
                    }
                }
            }
        }

        Ok(EntryPage {
            entries,
            next_cursor,
            total: None,
        })
    }

    async fn read_entry(&self, container: &str, entry: &str) -> AppResult<Entry> {
        let (partition, offset) = parse_offset_id(entry)?;
        let client = self.partition_client(container, partition).await?;
        let (records, _high) = client
            .fetch_records(offset, FETCH_BYTES, 0)
            .await
            .map_err(|e| map_kafka_error(&e))?;

        let record = records
            .into_iter()
            .find(|item| item.offset == offset)
            .ok_or_else(|| {
                AppError::server(format!(
                    "message {entry} is no longer available (compacted or past retention)"
                ))
            })?;

        let value = record.record.value.unwrap_or_default();
        let mut out = Entry::new(
            entry,
            format!("offset {offset}"),
            "message",
        );
        out.size = Some(value.len());
        out.timestamp = Some(record.record.timestamp);
        out.meta.insert("partition".into(), partition.to_string());
        out.meta.insert("offset".into(), offset.to_string());
        if let Some(key) = record.record.key.as_ref() {
            out.meta
                .insert("key".into(), String::from_utf8_lossy(key).to_string());
        }
        for (header, header_value) in &record.record.headers {
            out.meta.insert(
                format!("h:{header}"),
                String::from_utf8_lossy(header_value).to_string(),
            );
        }
        out.payload = Some(Payload::from_bytes(&value, self.config.max_payload()));
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
            "Kafka topics are append-only; use publish instead",
        ))
    }

    async fn delete_entry(&self, _container: &str, _entries: &[String]) -> AppResult<MutationResult> {
        Err(AppError::unsupported(
            "individual Kafka messages cannot be deleted; lower the topic retention instead",
        ))
    }

    async fn publish(
        &self,
        container: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        let partitions = self.partitions_of(container).await?;
        let partition = match options.partition {
            Some(partition) if partitions.contains(&partition) => partition,
            Some(partition) => {
                return Err(AppError::invalid(format!(
                    "topic {container} has no partition {partition}"
                )))
            }
            // No partition given: let the broker's default partitioning apply by
            // writing to partition 0, which mirrors what a keyless producer does
            // on a single-partition topic and keeps behaviour predictable.
            None => *partitions
                .first()
                .ok_or_else(|| AppError::invalid(format!("topic {container} has no partitions")))?,
        };

        let client = self.partition_client(container, partition).await?;
        let mut headers = BTreeMap::new();
        for (key, value) in options.headers.clone().unwrap_or_default() {
            headers.insert(key, value.into_bytes());
        }
        let record = Record {
            key: options.routing_key.clone().map(|k| k.into_bytes()),
            value: Some(payload.to_vec()),
            headers,
            timestamp: chrono::Utc::now(),
        };

        let offsets = client
            .produce(vec![record], Compression::NoCompression)
            .await
            .map_err(|e| map_kafka_error(&e))?;

        Ok(MutationResult {
            ok: true,
            message: Some(match offsets.first() {
                Some(offset) => format!("produced to {container}[{partition}] at offset {offset}"),
                None => format!("produced to {container}[{partition}]"),
            }),
            affected: offsets
                .iter()
                .map(|offset| format!("{partition}:{offset}"))
                .collect(),
        })
    }

    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        let topic = container.to_string();
        let partitions = self.partitions_of(&topic).await?;
        let client = self.client.clone();
        let subscription_id = uuid::Uuid::new_v4().to_string();
        let id_for_task = subscription_id.clone();
        let (tx, rx) = mpsc::channel(512);

        tokio::spawn(async move {
            // One cursor per partition, each starting at "latest" so the tail
            // shows new traffic rather than replaying history.
            let mut cursors: Vec<(i32, PartitionClient, i64)> = Vec::new();
            for partition in partitions {
                match client
                    .partition_client(&topic, partition, UnknownTopicHandling::Retry)
                    .await
                {
                    Ok(partition_client) => {
                        match partition_client.get_offset(OffsetAt::Latest).await {
                            Ok(offset) => cursors.push((partition, partition_client, offset)),
                            Err(err) => {
                                tracing::warn!(partition, error = %err, "cannot read tail offset");
                            }
                        }
                    }
                    Err(err) => {
                        tracing::warn!(partition, error = %err, "cannot open partition for tail");
                    }
                }
            }

            loop {
                let mut produced_any = false;
                for (partition, partition_client, offset) in cursors.iter_mut() {
                    let fetched = partition_client
                        .fetch_records(*offset, FETCH_BYTES, 500)
                        .await;
                    let Ok((records, _high)) = fetched else {
                        // Broker hiccup: keep the loop alive, it will retry.
                        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                        continue;
                    };
                    for item in records {
                        *offset = item.offset + 1;
                        produced_any = true;
                        let value = item.record.value.unwrap_or_default();
                        let message = StreamMessage {
                            subscription_id: id_for_task.clone(),
                            source: topic.clone(),
                            payload: Payload::from_bytes(&value, 512 * 1024),
                            key: item
                                .record
                                .key
                                .as_ref()
                                .map(|k| String::from_utf8_lossy(k).to_string()),
                            headers: item
                                .record
                                .headers
                                .iter()
                                .map(|(k, v)| {
                                    (k.clone(), String::from_utf8_lossy(v).to_string())
                                })
                                .collect(),
                            timestamp: item.record.timestamp,
                            position: Some(format!("{partition}:{}", item.offset)),
                        };
                        if tx.send(message).await.is_err() {
                            return;
                        }
                    }
                }
                if !produced_any {
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        });

        Ok(TailHandle {
            subscription_id,
            receiver: rx,
        })
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        let topics = self.refresh_topics().await?;
        let mut out = Vec::new();
        for (topic, partitions) in topics {
            out.push(StatItem {
                section: "topics".into(),
                key: topic.clone(),
                value: format!("{} partition(s)", partitions.len()),
            });
            for partition in partitions {
                // Message count per partition is an extra round trip each; cap
                // the work so a large cluster does not stall the UI.
                if out.len() > 200 {
                    continue;
                }
                let Ok(client) = self.partition_client(&topic, partition).await else {
                    continue;
                };
                let (low, high) = tokio::join!(
                    client.get_offset(OffsetAt::Earliest),
                    client.get_offset(OffsetAt::Latest)
                );
                if let (Ok(low), Ok(high)) = (low, high) {
                    out.push(StatItem {
                        section: format!("topic:{topic}"),
                        key: format!("partition {partition}"),
                        value: format!("{} message(s)", (high - low).max(0)),
                    });
                }
            }
        }
        Ok(out)
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_message_ids() {
        assert_eq!(parse_offset_id("2:105").unwrap(), (2, 105));
        assert!(parse_offset_id("bad").is_err());
        assert!(parse_offset_id("x:1").is_err());
    }
}
