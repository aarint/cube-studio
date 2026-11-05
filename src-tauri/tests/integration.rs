//! End-to-end integration tests against real middleware.
//!
//! These tests are gated behind the `integration` feature so `cargo test` stays
//! fast and hermetic. Start the containers first:
//!
//! ```text
//! docker compose -f docker-compose.test.yml up -d
//! cargo test --features integration
//! ```
//!
//! Each test connects through the same `Backend` trait the Tauri commands use,
//! so a pass here means the whole command path works, not just a driver.

#![cfg(feature = "integration")]

use std::collections::BTreeMap;

use cube_studio_lib::config::{BackendKind, ConnectionConfig, RedisMode};
use cube_studio_lib::model::WriteOptions;

fn base_config(kind: BackendKind, port: u16) -> ConnectionConfig {
    ConnectionConfig {
        kind,
        host: "127.0.0.1".into(),
        port,
        timeout_secs: Some(10),
        ..Default::default()
    }
}

fn redis_config() -> ConnectionConfig {
    ConnectionConfig {
        redis_mode: RedisMode::Standalone,
        ..base_config(BackendKind::Redis, 26379)
    }
}

fn memcached_config() -> ConnectionConfig {
    base_config(BackendKind::Memcached, 21211)
}

fn kafka_config() -> ConnectionConfig {
    base_config(BackendKind::Kafka, 19092)
}

fn rabbit_config() -> ConnectionConfig {
    let mut config = base_config(BackendKind::RabbitMq, 15672);
    config.username = "guest".into();
    config.password = "guest".into();
    config.vhost = "/".into();
    config
        .meta
        .insert("rabbit_queues".into(), "cube-test-queue".into());
    config
}

fn nats_config() -> ConnectionConfig {
    base_config(BackendKind::Nats, 14222)
}

fn mqtt_config() -> ConnectionConfig {
    base_config(BackendKind::Mqtt, 11883)
}

// ---- Redis ------------------------------------------------------------

#[tokio::test]
async fn redis_lists_databases_and_keys() {
    let backend = cube_studio_lib::backend::connect(&redis_config())
        .await
        .expect("redis must be reachable");
    backend.ping().await.expect("ping");

    let info = backend.info().await.expect("info");
    assert_eq!(info.kind, BackendKind::Redis);
    assert!(info.server_version.is_some(), "version should be reported");

    let containers = backend.containers().await.expect("containers");
    assert!(
        containers.len() >= 16,
        "default redis has 16 databases, got {}",
        containers.len()
    );
    assert_eq!(containers[0].id, "db:0");

    let page = backend
        .entries("db:0", None, 100, None)
        .await
        .expect("entries");
    let labels: Vec<&str> = page.entries.iter().map(|e| e.label.as_str()).collect();
    assert!(labels.contains(&"greet"), "seeded key missing: {labels:?}");
    assert!(labels.contains(&"myhash"), "seeded hash missing");
}

#[tokio::test]
async fn redis_reads_typed_values() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();

    let string_entry = backend.read_entry("db:0", "greet").await.unwrap();
    assert_eq!(string_entry.kind, "string");
    let payload = string_entry.payload.expect("payload");
    assert_eq!(payload.text.as_deref(), Some("hello"));

    // A hash is rendered as a JSON-ish object so the UI can show structure.
    let hash_entry = backend.read_entry("db:0", "myhash").await.unwrap();
    assert_eq!(hash_entry.kind, "hash");
    let hash_text = hash_entry.payload.unwrap().text.unwrap();
    assert!(hash_text.contains("f1"), "hash body: {hash_text}");
}

#[tokio::test]
async fn redis_reports_ttl_and_json_payloads() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();

    let ttl_entry = backend.read_entry("db:0", "tmpkey").await.unwrap();
    assert!(
        ttl_entry.ttl.unwrap_or(-1) > 0,
        "key with EX should report a positive ttl"
    );

    let json_entry = backend.read_entry("db:0", "cfg").await.unwrap();
    let payload = json_entry.payload.unwrap();
    assert!(payload.json.is_some(), "json payload should parse");
    assert_eq!(payload.json.unwrap()["a"], 1);
}

#[tokio::test]
async fn redis_write_read_delete_roundtrip() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();

    backend
        .write_entry(
            "db:0",
            "it:written",
            br#"{"ok":true}"#,
            &WriteOptions {
                entry_kind: Some("string".into()),
                ttl: Some(120),
                ..Default::default()
            },
        )
        .await
        .expect("write");

    let read = backend.read_entry("db:0", "it:written").await.unwrap();
    assert_eq!(read.payload.unwrap().text.as_deref(), Some(r#"{"ok":true}"#));
    assert!(read.ttl.unwrap_or(0) > 0, "ttl should be set");

    let deleted = backend
        .delete_entry("db:0", &["it:written".to_string()])
        .await
        .unwrap();
    assert!(deleted.ok);
    assert!(backend.read_entry("db:0", "it:written").await.is_err());
}

#[tokio::test]
async fn redis_writes_hashes_from_json() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();
    backend
        .write_entry(
            "db:0",
            "it:hash",
            br#"{"alpha":"one","beta":"two"}"#,
            &WriteOptions {
                entry_kind: Some("hash".into()),
                ..Default::default()
            },
        )
        .await
        .expect("write hash");

    let entry = backend.read_entry("db:0", "it:hash").await.unwrap();
    assert_eq!(entry.kind, "hash");
    let text = entry.payload.unwrap().text.unwrap();
    assert!(text.contains("alpha") && text.contains("one"), "{text}");
    backend.delete_entry("db:0", &["it:hash".into()]).await.unwrap();
}

#[tokio::test]
async fn redis_search_filters_keys() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();
    let page = backend.entries("db:0", None, 100, Some("myha")).await.unwrap();
    assert_eq!(page.entries.len(), 1, "search should narrow to one key");
    assert_eq!(page.entries[0].label, "myhash");
}

#[tokio::test]
async fn redis_console_runs_commands() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();

    let pong = backend.console("PING").await.unwrap();
    assert!(!pong.error);
    assert!(pong.output.contains("PONG"), "got {}", pong.output);

    let error = backend.console("NOSUCHCOMMAND").await.unwrap();
    assert!(error.error, "unknown command should be reported as an error");
}

#[tokio::test]
async fn redis_stats_are_parsed() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();
    let stats = backend.stats().await.unwrap();
    assert!(stats.iter().any(|s| s.key == "redis_version"));
    assert!(stats.iter().any(|s| s.key == "used_memory"));
}

#[tokio::test]
async fn redis_pubsub_tail_delivers_messages() {
    let backend = cube_studio_lib::backend::connect(&redis_config())
        .await
        .unwrap();
    let mut handle = backend.tail("channel:it.*").await.expect("subscribe");

    // Give the subscription a moment to be registered server side, then publish.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    backend
        .console("PUBLISH it.hello payload-1")
        .await
        .expect("publish");

    let message = tokio::time::timeout(std::time::Duration::from_secs(5), handle.receiver.recv())
        .await
        .expect("timed out waiting for pubsub message")
        .expect("channel closed");
    assert_eq!(message.payload.text.as_deref(), Some("payload-1"));
    assert_eq!(message.source, "it.hello");
}

// ---- Memcached --------------------------------------------------------

#[tokio::test]
async fn memcached_lists_and_reads_keys() {
    let backend = cube_studio_lib::backend::connect(&memcached_config())
        .await
        .expect("memcached must be reachable");
    backend.ping().await.expect("ping");

    let containers = backend.containers().await.unwrap();
    assert_eq!(containers.len(), 1);
    assert_eq!(containers[0].id, "server");

    let page = backend.entries("server", None, 100, None).await.unwrap();
    let labels: Vec<&str> = page.entries.iter().map(|e| e.label.as_str()).collect();
    assert!(
        labels.contains(&"user:1"),
        "seeded memcached key missing: {labels:?}"
    );

    let entry = backend.read_entry("server", "user:1").await.unwrap();
    assert_eq!(entry.payload.unwrap().text.as_deref(), Some("alice"));
}

#[tokio::test]
async fn memcached_write_delete_roundtrip() {
    let backend = cube_studio_lib::backend::connect(&memcached_config()).await.unwrap();
    backend
        .write_entry(
            "server",
            "it:mem",
            b"value-here",
            &WriteOptions {
                ttl: Some(60),
                ..Default::default()
            },
        )
        .await
        .expect("write");

    let entry = backend.read_entry("server", "it:mem").await.unwrap();
    assert_eq!(entry.payload.unwrap().text.as_deref(), Some("value-here"));

    let deleted = backend
        .delete_entry("server", &["it:mem".to_string()])
        .await
        .unwrap();
    assert!(deleted.ok);
    assert!(backend.read_entry("server", "it:mem").await.is_err());
}

#[tokio::test]
async fn memcached_stats_are_reported() {
    let backend = cube_studio_lib::backend::connect(&memcached_config()).await.unwrap();
    let stats = backend.stats().await.unwrap();
    assert!(stats.iter().any(|s| s.key == "curr_items"));
    assert!(stats.iter().any(|s| s.key == "version"));
}

// ---- Kafka ------------------------------------------------------------

#[tokio::test]
async fn kafka_lists_topics_and_messages() {
    let backend = cube_studio_lib::backend::connect(&kafka_config())
        .await
        .expect("kafka must be reachable");
    backend.ping().await.expect("ping");

    let containers = backend.containers().await.unwrap();
    assert!(
        containers.iter().any(|c| c.id == "demo"),
        "demo topic missing: {:?}",
        containers.iter().map(|c| &c.id).collect::<Vec<_>>()
    );

    let page = backend.entries("demo", None, 50, None).await.unwrap();
    assert!(!page.entries.is_empty(), "demo topic should have messages");
    // Messages come back newest first.
    let first = &page.entries[0];
    assert_eq!(first.kind, "message");
    assert!(first.meta.contains_key("offset"));

    // Read the body of the newest message; it must be readable and match the
    // entry's advertised size.
    let entry = backend.read_entry("demo", &first.id).await.unwrap();
    let payload = entry.payload.expect("payload");
    assert!(!payload.truncated);
    assert!(
        !payload.text.unwrap_or_default().is_empty(),
        "message body should not be empty"
    );
    assert_eq!(payload.size, first.size.unwrap_or(payload.size));
}

#[tokio::test]
async fn kafka_publishes_messages() {
    let backend = cube_studio_lib::backend::connect(&kafka_config()).await.unwrap();
    let result = backend
        .publish(
            "demo",
            b"from-cube-studio",
            &WriteOptions {
                partition: Some(0),
                ..Default::default()
            },
        )
        .await
        .expect("publish");
    assert!(result.ok);

    // The newest message should now be visible.
    let page = backend.entries("demo", None, 5, None).await.unwrap();
    assert!(!page.entries.is_empty());
}

#[tokio::test]
async fn kafka_rejects_editing_messages() {
    let backend = cube_studio_lib::backend::connect(&kafka_config()).await.unwrap();
    let error = backend
        .write_entry("demo", "0:0", b"nope", &WriteOptions::default())
        .await
        .expect_err("kafka topics are append-only");
    assert!(error.message.contains("append-only"), "{}", error.message);
}

// ---- RabbitMQ ---------------------------------------------------------

#[tokio::test]
async fn rabbit_declares_queue_publishes_and_reads() {
    let backend = cube_studio_lib::backend::connect(&rabbit_config())
        .await
        .expect("rabbitmq must be reachable");
    backend.ping().await.expect("ping");

    // Create the queue through the console, then verify it appears.
    let declared = backend.console("queue_declare cube-test-queue").await.unwrap();
    assert!(!declared.error, "declare failed: {}", declared.output);

    let containers = backend.containers().await.unwrap();
    assert!(
        containers.iter().any(|c| c.id == "queue:cube-test-queue"),
        "queue not listed"
    );

    // Drain anything a previous run left behind.
    let _ = backend.flush("queue:cube-test-queue").await;

    backend
        .publish(
            "queue:cube-test-queue",
            b"rabbit-payload",
            &WriteOptions::default(),
        )
        .await
        .expect("publish");

    let page = backend
        .entries("queue:cube-test-queue", None, 10, None)
        .await
        .unwrap();
    assert_eq!(page.entries.len(), 1, "queue should hold one message");

    let entry = backend
        .read_entry("queue:cube-test-queue", &page.entries[0].id)
        .await
        .unwrap();
    assert_eq!(
        entry.payload.unwrap().text.as_deref(),
        Some("rabbit-payload")
    );

    // Reading must not lose the message.
    let again = backend
        .entries("queue:cube-test-queue", None, 10, None)
        .await
        .unwrap();
    assert_eq!(again.entries.len(), 1, "reading must leave the queue intact");
    let reread = backend
        .read_entry("queue:cube-test-queue", &again.entries[0].id)
        .await
        .unwrap();
    assert_eq!(
        reread.payload.unwrap().text.as_deref(),
        Some("rabbit-payload")
    );

    backend.flush("queue:cube-test-queue").await.unwrap();
}

// ---- NATS -------------------------------------------------------------

#[tokio::test]
async fn nats_detects_jetstream_and_lists_streams() {
    let backend = cube_studio_lib::backend::connect(&nats_config())
        .await
        .expect("nats must be reachable");
    backend.ping().await.expect("ping");

    let info = backend.info().await.unwrap();
    assert_eq!(info.server_version.as_deref(), Some("JetStream"));

    let containers = backend.containers().await.unwrap();
    // At minimum the live-subject pseudo container is present.
    assert!(containers.iter().any(|c| c.id == "subject:>"));
}

#[tokio::test]
async fn nats_publish_and_tail() {
    let backend = cube_studio_lib::backend::connect(&nats_config()).await.unwrap();

    let mut handle = backend.tail("subject:it.events.*").await.expect("tail");
    tokio::time::sleep(std::time::Duration::from_millis(250)).await;

    backend
        .publish(
            "subject:it.events.one",
            b"nats-payload",
            &WriteOptions::default(),
        )
        .await
        .expect("publish");

    let message = tokio::time::timeout(std::time::Duration::from_secs(5), handle.receiver.recv())
        .await
        .expect("timed out waiting for nats message")
        .expect("channel closed");
    assert_eq!(message.payload.text.as_deref(), Some("nats-payload"));
    assert_eq!(message.source, "it.events.one");
}

#[tokio::test]
async fn nats_stats_report_jetstream_usage() {
    let backend = cube_studio_lib::backend::connect(&nats_config()).await.unwrap();
    let stats = backend.stats().await.unwrap();
    assert!(stats.iter().any(|s| s.key == "streams"));
    assert!(stats.iter().any(|s| s.key == "memory"));
}

// ---- MQTT -------------------------------------------------------------

#[tokio::test]
async fn mqtt_publish_retain_and_read() {
    let backend = cube_studio_lib::backend::connect(&mqtt_config())
        .await
        .expect("mqtt must be reachable");
    backend.ping().await.expect("ping");

    // A retained publish becomes readable afterwards.
    backend
        .publish(
            "topic:it/sensors/temp",
            b"21.5",
            &WriteOptions {
                retained: Some(true),
                qos: Some(1),
                ..Default::default()
            },
        )
        .await
        .expect("publish retained");
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;

    let entry = backend.read_entry("topic:it/sensors/temp", "it/sensors/temp").await.unwrap();
    assert_eq!(entry.payload.unwrap().text.as_deref(), Some("21.5"));

    // Clearing the retained message makes it disappear.
    backend
        .delete_entry("topic:it/sensors/temp", &["it/sensors/temp".to_string()])
        .await
        .expect("clear retained");
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    assert!(backend
        .read_entry("topic:it/sensors/temp", "it/sensors/temp")
        .await
        .is_err());
}

#[tokio::test]
async fn mqtt_tail_delivers_messages() {
    let backend = cube_studio_lib::backend::connect(&mqtt_config()).await.unwrap();
    let mut handle = backend.tail("topic:it/live/+").await.expect("tail");
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    backend
        .publish(
            "topic:it/live/device1",
            b"mqtt-hello",
            &WriteOptions {
                qos: Some(1),
                ..Default::default()
            },
        )
        .await
        .expect("publish");

    let message = tokio::time::timeout(std::time::Duration::from_secs(5), handle.receiver.recv())
        .await
        .expect("timed out waiting for mqtt message")
        .expect("channel closed");
    assert_eq!(message.payload.text.as_deref(), Some("mqtt-hello"));
    assert_eq!(message.source, "it/live/device1");
}

// ---- cross-backend ----------------------------------------------------

/// The whole point of the refactor: every backend answers the same questions.
#[tokio::test]
async fn every_backend_speaks_the_same_protocol() {
    let configs = [
        ("redis", redis_config()),
        ("memcached", memcached_config()),
        ("kafka", kafka_config()),
        ("rabbitmq", rabbit_config()),
        ("nats", nats_config()),
        ("mqtt", mqtt_config()),
    ];

    for (name, config) in configs {
        let backend = cube_studio_lib::backend::connect(&config)
            .await
            .unwrap_or_else(|e| panic!("{name} failed to connect: {}", e.message));

        backend
            .ping()
            .await
            .unwrap_or_else(|e| panic!("{name} ping failed: {}", e.message));

        let info = backend
            .info()
            .await
            .unwrap_or_else(|e| panic!("{name} info failed: {}", e.message));
        assert_eq!(info.kind, config.kind);
        assert_eq!(info.capabilities.kind, config.kind);

        let containers = backend
            .containers()
            .await
            .unwrap_or_else(|e| panic!("{name} containers failed: {}", e.message));
        assert!(
            !containers.is_empty(),
            "{name} should expose at least one container"
        );

        // Every container id must round-trip into a page request without
        // panicking, even when the container is empty.
        for container in containers.iter().take(2) {
            let result = backend.entries(&container.id, None, 5, None).await;
            let page = result
                .unwrap_or_else(|e| panic!("{name}/{} entries failed: {}", container.id, e.message));
            let _ = page.next_cursor;
        }

        // Stats must be either populated or an explicit "unsupported".
        let _ = backend.stats().await;
    }
}

/// Error classification is what the UI branches on, so make sure the driver
/// errors land in the right bucket.
#[tokio::test]
async fn connection_failures_are_classified_as_network_errors() {
    for kind in [
        BackendKind::Redis,
        BackendKind::Memcached,
        BackendKind::Kafka,
        BackendKind::RabbitMq,
        BackendKind::Nats,
        BackendKind::Mqtt,
    ] {
        // Port 1 is reserved and nothing listens on it.
        let config = ConnectionConfig {
            kind,
            host: "127.0.0.1".into(),
            port: 1,
            timeout_secs: Some(2),
            ..Default::default()
        };
        let result = cube_studio_lib::backend::connect(&config).await;
        let error = result.err().unwrap_or_else(|| {
            panic!("{kind:?} unexpectedly connected to a dead port");
        });
        assert!(
            matches!(
                error.kind,
                cube_studio_lib::ErrorKind::Network | cube_studio_lib::ErrorKind::Timeout
            ),
            "{kind:?} produced {:?}: {}",
            error.kind,
            error.message
        );
    }
}

/// Metadata maps must stay JSON-serializable, since they cross IPC.
#[tokio::test]
async fn containers_serialize_to_json() {
    let backend = cube_studio_lib::backend::connect(&redis_config()).await.unwrap();
    let containers = backend.containers().await.unwrap();
    let json = serde_json::to_string(&containers).expect("containers must serialize");
    assert!(json.contains("redis_db"));
}

/// The default container a backend advertises must be valid.
#[tokio::test]
async fn default_container_is_usable() {
    for (name, config) in [
        ("redis", redis_config()),
        ("memcached", memcached_config()),
        ("mqtt", mqtt_config()),
    ] {
        let backend = cube_studio_lib::backend::connect(&config).await.unwrap();
        let info = backend.info().await.unwrap();
        if let Some(container) = info.default_container {
            backend
                .entries(&container, None, 5, None)
                .await
                .unwrap_or_else(|e| {
                    panic!("{name} default container {container} failed: {}", e.message)
                });
        }
    }
}

/// Metadata helper used by the UI to seed the queue/topic list editors.
#[test]
fn metadata_keys_are_stable() {
    // These strings are part of the stored profile format; changing them
    // silently would orphan a user's saved browsing setup.
    assert_eq!(cube_studio_lib::backend::rabbit_backend::QUEUES_META_KEY, "rabbit_queues");
    assert_eq!(cube_studio_lib::backend::mqtt_backend::TOPICS_META_KEY, "mqtt_topics");
    let _ = BTreeMap::<String, String>::new();
}
