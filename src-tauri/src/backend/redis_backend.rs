//! Redis driver.
//!
//! Supports standalone, cluster and sentinel deployments over plain TCP or
//! TLS. Values are read with type awareness so hashes, lists, sets and sorted
//! sets all render usefully in the UI instead of showing an opaque blob.
//!
//! Cursor handling: Redis `SCAN` returns a `u64` cursor that is encoded into
//! the opaque `next_cursor` string handed to the UI.

use std::collections::BTreeMap;

use async_trait::async_trait;
use redis::aio::ConnectionManager;
use redis::TlsCertificates;
use tokio::sync::{mpsc, Mutex};

use crate::backend::{payload_of, Backend, ConnectionInfo, TailHandle};
use crate::config::{ConnectionConfig, RedisMode};
use crate::error::{AppError, AppResult, ErrorKind};
use crate::model::{
    Capabilities, ConsoleResult, Container, ContainerKind, Entry, EntryPage, MutationResult,
    StatItem, StreamMessage, WriteOptions,
};

/// Redis connection flavours differ enough that we keep two clients.
enum Client {
    /// Standalone/sentinel: a single auto-reconnecting multiplexed connection.
    Single(Mutex<ConnectionManager>),
    /// Cluster: the cluster client routes commands per slot.
    Cluster(redis::cluster_async::ClusterConnection),
}

pub struct RedisBackend {
    config: ConnectionConfig,
    client: Client,
    /// Database index currently selected (standalone only; cluster has one DB).
    db: Mutex<i64>,
    /// Server version, captured on connect.
    server_version: Mutex<Option<String>>,
}

impl RedisBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let endpoints = config.endpoints()?;
        let timeout = config.timeout();

        let client = match config.redis_mode {
            RedisMode::Cluster => {
                let nodes: Vec<String> = endpoints.iter().map(|e| e.authority()).collect();
                let mut builder = redis::cluster::ClusterClientBuilder::new(nodes)
                    .username(&config.username)
                    .password(&config.password)
                    .connection_timeout(timeout);
                if config.tls.is_enabled() {
                    builder = builder.tls(redis::TlsMode::Secure);
                    if let Some(ca) = config.tls_ca_file.as_ref().filter(|p| !p.trim().is_empty())
                    {
                        let pem = std::fs::read(ca).map_err(|e| {
                            AppError::invalid(format!("cannot read TLS CA file {ca}: {e}"))
                        })?;
                        builder = builder.certs(TlsCertificates {
                            client_tls: None,
                            root_cert: Some(pem),
                        });
                    }
                }
                let cluster = builder
                    .build()
                    .map_err(|e| AppError::driver(format!("cannot build cluster client: {e}")))?;
                let conn = tokio::time::timeout(timeout, cluster.get_async_connection())
                    .await
                    .map_err(|_| AppError::network("cluster connection timed out"))?
                    .map_err(map_redis_error)?;
                Client::Cluster(conn)
            }
            RedisMode::Sentinel => {
                // Resolve the current master, then connect to it directly. The
                // redis crate's sentinel module is sync-only, so failover is
                // re-resolved on the next connect rather than inside the
                // connection manager.
                let master = config
                    .redis_sentinel_master
                    .clone()
                    .filter(|m| !m.trim().is_empty())
                    .ok_or_else(|| AppError::invalid("sentinel mode requires a master name"))?;
                let primary = endpoints[0].clone();
                let (host, port) =
                    resolve_sentinel_master(&primary, &master, &config, timeout).await?;
                let client = build_client(&config, &host, port, 0)?;
                let manager = tokio::time::timeout(timeout, ConnectionManager::new(client))
                    .await
                    .map_err(|_| AppError::network("sentinel master connection timed out"))?
                    .map_err(map_redis_error)?;
                Client::Single(Mutex::new(manager))
            }
            RedisMode::Standalone => {
                let primary = endpoints[0].clone();
                let client = build_client(&config, &primary.host, primary.port, config.redis_db)?;
                let manager = tokio::time::timeout(timeout, ConnectionManager::new(client))
                    .await
                    .map_err(|_| AppError::network("connection timed out"))?
                    .map_err(map_redis_error)?;
                Client::Single(Mutex::new(manager))
            }
        };

        let backend = Self {
            config,
            client,
            db: Mutex::new(0),
            server_version: Mutex::new(None),
        };

        // Fail fast if the server is unusable, and capture its version.
        backend.ping().await?;
        if let Ok(info) = backend.server_info().await {
            if let Some(version) = info.get("redis_version") {
                *backend.server_version.lock().await = Some(version.to_string());
            }
        }
        Ok(backend)
    }

    /// SELECT the given DB on the single connection. No-op for cluster.
    async fn select(&self, db: i64) -> AppResult<()> {
        let mut current = self.db.lock().await;
        if *current == db {
            return Ok(());
        }
        match &self.client {
            Client::Single(manager) => {
                let mut conn = manager.lock().await.clone();
                redis::cmd("SELECT")
                    .arg(db)
                    .query_async::<()>(&mut conn)
                    .await
                    .map_err(map_redis_error)?;
                *current = db;
                Ok(())
            }
            Client::Cluster(_) => Ok(()),
        }
    }

    async fn run_command<T: redis::FromRedisValue>(&self, cmd: &mut redis::Cmd) -> AppResult<T> {
        match &self.client {
            Client::Single(manager) => {
                // Cloning the manager is cheap (an Arc bump) and avoids holding
                // the mutex across the await.
                let mut conn = manager.lock().await.clone();
                cmd.query_async(&mut conn).await.map_err(map_redis_error)
            }
            Client::Cluster(cluster) => {
                let mut conn = cluster.clone();
                cmd.query_async(&mut conn).await.map_err(map_redis_error)
            }
        }
    }

    /// A raw connection config, used for sentinel/master targets.
    fn conn_args(config: &ConnectionConfig, host: &str, port: u16, db: i64) -> String {
        let mut url = String::new();
        url.push_str(if config.tls.is_enabled() {
            "rediss://"
        } else {
            "redis://"
        });
        if !config.username.is_empty() || !config.password.is_empty() {
            if !config.username.is_empty() {
                url.push_str(&percent_encode(&config.username));
            }
            if !config.password.is_empty() {
                url.push(':');
                url.push_str(&percent_encode(&config.password));
            }
            url.push('@');
        }
        url.push_str(host);
        url.push(':');
        url.push_str(&port.to_string());
        url.push('/');
        url.push_str(&db.to_string());
        if matches!(config.tls, crate::config::TlsMode::Insecure) || config.tls_skip_verify {
            // The redis crate understands this fragment to skip verification.
            url.push_str("/#insecure");
        }
        url
    }

    async fn server_info(&self) -> AppResult<BTreeMap<String, String>> {
        let raw: String = self.run_command(&mut redis::cmd("INFO")).await?;
        Ok(parse_info(&raw))
    }

    /// The number of logical databases, which decides whether the DB picker is
    /// useful at all.
    async fn databases_count(&self) -> i64 {
        match self
            .run_command::<BTreeMap<String, String>>(&mut redis::cmd("CONFIG").arg("GET").arg("databases"))
            .await
        {
            Ok(map) => map
                .get("databases")
                .and_then(|v| v.parse().ok())
                .unwrap_or(16),
            Err(_) => 1,
        }
    }

    /// Fetch the value of a key rendered for display, based on its type.
    async fn read_value(&self, key: &str) -> AppResult<(String, Vec<u8>)> {
        let kind: String = self.run_command(&mut redis::cmd("TYPE").arg(key)).await?;
        let bytes: Vec<u8> = match kind.as_str() {
            "none" => {
                return Err(AppError::new(
                    ErrorKind::ServerError,
                    "key no longer exists",
                ))
            }
            "string" => self.run_command(&mut redis::cmd("GET").arg(key)).await?,
            "hash" => {
                let flat: Vec<Vec<u8>> = self
                    .run_command(&mut redis::cmd("HGETALL").arg(key))
                    .await?;
                let mut out = String::from("{\n");
                for pair in flat.chunks(2) {
                    if pair.len() == 2 {
                        out.push_str(&format!(
                            "  {}: {},\n",
                            quote_if_needed(&String::from_utf8_lossy(&pair[0])),
                            quote_if_needed(&String::from_utf8_lossy(&pair[1]))
                        ));
                    }
                }
                if out.ends_with(",\n") {
                    out.truncate(out.len() - 2);
                    out.push('\n');
                }
                out.push('}');
                out.into_bytes()
            }
            "list" => {
                let items: Vec<Vec<u8>> = self
                    .run_command(&mut redis::cmd("LRANGE").arg(key).arg(0).arg(-1))
                    .await?;
                let mut out = String::from("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&format!("  \"{}\"", String::from_utf8_lossy(item)));
                    if index + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push(']');
                out.into_bytes()
            }
            "set" => {
                let items: Vec<Vec<u8>> = self
                    .run_command(&mut redis::cmd("SMEMBERS").arg(key))
                    .await?;
                let mut out = String::from("[\n");
                for (index, item) in items.iter().enumerate() {
                    out.push_str(&format!("  \"{}\"", String::from_utf8_lossy(item)));
                    if index + 1 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                }
                out.push(']');
                out.into_bytes()
            }
            "zset" => {
                let items: Vec<Vec<u8>> = self
                    .run_command(&mut redis::cmd("ZRANGE").arg(key).arg(0).arg(-1).arg("WITHSCORES"))
                    .await?;
                let mut out = String::from("[\n");
                let mut index = 0;
                while index + 1 < items.len() {
                    out.push_str(&format!(
                        "  {{ \"member\": \"{}\", \"score\": {} }}",
                        String::from_utf8_lossy(&items[index]),
                        String::from_utf8_lossy(&items[index + 1])
                    ));
                    if index + 2 < items.len() {
                        out.push(',');
                    }
                    out.push('\n');
                    index += 2;
                }
                out.push(']');
                out.into_bytes()
            }
            "stream" => {
                let items: Vec<Vec<u8>> = self
                    .run_command(&mut redis::cmd("XRANGE").arg(key).arg("-").arg("+").arg("COUNT").arg(100))
                    .await?;
                serde_json::to_vec_pretty(&items.iter().map(|i| String::from_utf8_lossy(i).to_string()).collect::<Vec<_>>())
                    .unwrap_or_else(|_| b"[]".to_vec())
            }
            _ => self.run_command(&mut redis::cmd("DUMP").arg(key)).await?,
        };
        Ok((kind, bytes))
    }

    /// Write a value honouring the requested Redis type.
    async fn write_value(
        &self,
        key: &str,
        kind: &str,
        payload: &[u8],
        ttl: Option<i64>,
    ) -> AppResult<()> {
        // Overwriting a key of a different type requires removing it first,
        // because most Redis writes refuse to clobber a mismatched type.
        let existing: String = self.run_command(&mut redis::cmd("TYPE").arg(key)).await?;
        if existing != "none" && existing != kind {
            self.run_command::<i64>(&mut redis::cmd("DEL").arg(key)).await?;
        }

        let text = String::from_utf8_lossy(payload).to_string();
        match kind {
            "hash" => {
                let map = parse_json_object(&text).map_err(|e| {
                    AppError::invalid(format!(
                        "hash values must be a JSON object of string fields: {e}"
                    ))
                })?;
                if existing != "none" {
                    // Replace the whole hash to keep semantics predictable.
                    self.run_command::<i64>(&mut redis::cmd("DEL").arg(key)).await?;
                }
                let mut cmd = redis::cmd("HSET");
                cmd.arg(key);
                for (field, value) in map {
                    cmd.arg(field).arg(value);
                }
                if cmd.get_packed_command().len() > 4 {
                    let _: i64 = self.run_command(&mut cmd).await?;
                }
            }
            "list" => {
                let items = parse_json_array(&text).map_err(|e| {
                    AppError::invalid(format!("list values must be a JSON array: {e}"))
                })?;
                self.run_command::<i64>(&mut redis::cmd("DEL").arg(key)).await?;
                if !items.is_empty() {
                    let mut cmd = redis::cmd("RPUSH");
                    cmd.arg(key);
                    for item in items {
                        cmd.arg(item);
                    }
                    self.run_command::<i64>(&mut cmd).await?;
                }
            }
            "set" => {
                let items = parse_json_array(&text).map_err(|e| {
                    AppError::invalid(format!("set values must be a JSON array: {e}"))
                })?;
                self.run_command::<i64>(&mut redis::cmd("DEL").arg(key)).await?;
                if !items.is_empty() {
                    let mut cmd = redis::cmd("SADD");
                    cmd.arg(key);
                    for item in items {
                        cmd.arg(item);
                    }
                    self.run_command::<i64>(&mut cmd).await?;
                }
            }
            "zset" => {
                // Accept [{"member":..,"score":..}] or a flat [member, score, ...].
                let pairs = parse_zset(&text).map_err(AppError::invalid)?;
                self.run_command::<i64>(&mut redis::cmd("DEL").arg(key)).await?;
                if !pairs.is_empty() {
                    let mut cmd = redis::cmd("ZADD");
                    cmd.arg(key);
                    for (member, score) in pairs {
                        cmd.arg(score).arg(member);
                    }
                    self.run_command::<i64>(&mut cmd).await?;
                }
            }
            _ => {
                let mut cmd = redis::cmd("SET");
                cmd.arg(key).arg(payload);
                self.run_command::<()>(&mut cmd).await?;
            }
        }

        match ttl {
            Some(seconds) if seconds > 0 => {
                self.run_command::<i64>(&mut redis::cmd("EXPIRE").arg(key).arg(seconds))
                    .await?;
            }
            Some(seconds) if seconds <= 0 => {
                self.run_command::<i64>(&mut redis::cmd("PERSIST").arg(key))
                    .await?;
            }
            _ => {}
        }
        Ok(())
    }
}

#[async_trait]
impl Backend for RedisBackend {
    async fn ping(&self) -> AppResult<()> {
        self.run_command::<String>(&mut redis::cmd("PING")).await.map(|_| ())
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        let version = self.server_version.lock().await.clone();
        let total_dbs = self.databases_count().await;
        Ok(ConnectionInfo {
            kind: crate::config::BackendKind::Redis,
            server_version: version,
            capabilities: Capabilities::for_kind(crate::config::BackendKind::Redis),
            default_container: Some(if total_dbs > 1 {
                format!("db:{}", self.config.redis_db.max(0))
            } else {
                "db:0".to_string()
            }),
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        if matches!(self.client, Client::Cluster(_)) {
            // A cluster only has DB 0.
            return Ok(vec![Container {
                id: "db:0".into(),
                name: "DB0".into(),
                kind: ContainerKind::RedisDb,
                meta: BTreeMap::new(),
                size: None,
            }]);
        }
        let total = self.databases_count().await;
        let mut out = Vec::new();
        for index in 0..total {
            out.push(Container {
                id: format!("db:{index}"),
                name: format!("DB{index}"),
                kind: ContainerKind::RedisDb,
                meta: BTreeMap::new(),
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
        search: Option<&str>,
    ) -> AppResult<EntryPage> {
        let db = container_db_index(container)?;
        self.select(db).await?;

        let start_cursor: u64 = cursor.and_then(|c| c.parse().ok()).unwrap_or(0);
        let pattern = search.filter(|s| !s.is_empty()).unwrap_or("*");
        let pattern = if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            pattern.to_string()
        } else {
            format!("*{pattern}*")
        };

        let mut cmd = redis::cmd("SCAN");
        cmd.arg(start_cursor)
            .arg("MATCH")
            .arg(&pattern)
            .arg("COUNT")
            .arg(limit.clamp(10, 5000));
        let (next, keys): (u64, Vec<Vec<u8>>) = self.run_command(&mut cmd).await?;

        let mut entries = Vec::with_capacity(keys.len());
        for key in keys.iter().take(limit) {
            let key_str = String::from_utf8_lossy(key).to_string();
            let mut entry = Entry::new(key_str.clone(), key_str, "unknown");
            // TYPE and TTL in one round trip keeps big scans responsive.
            if let Ok(kind) = self
                .run_command::<String>(&mut redis::cmd("TYPE").arg(key))
                .await
            {
                entry.kind = kind;
            }
            if let Ok(ttl) = self
                .run_command::<i64>(&mut redis::cmd("TTL").arg(key))
                .await
            {
                entry.ttl = Some(ttl);
            }
            if let Ok(size) = self
                .run_command::<i64>(&mut redis::cmd("MEMORY").arg("USAGE").arg(key))
                .await
            {
                entry.size = Some(size.max(0) as usize);
            }
            entries.push(entry);
        }

        Ok(EntryPage {
            entries,
            next_cursor: if next == 0 {
                None
            } else {
                Some(next.to_string())
            },
            total: None,
        })
    }

    async fn read_entry(&self, container: &str, entry: &str) -> AppResult<Entry> {
        let db = container_db_index(container)?;
        self.select(db).await?;

        let (kind, bytes) = self.read_value(entry).await?;
        let ttl = self
            .run_command::<i64>(&mut redis::cmd("TTL").arg(entry))
            .await
            .unwrap_or(-1);
        let size = self
            .run_command::<i64>(&mut redis::cmd("MEMORY").arg("USAGE").arg(entry))
            .await
            .ok()
            .map(|v| v.max(0) as usize);

        let mut item = Entry::new(entry, entry, kind);
        item.ttl = Some(ttl);
        item.size = size.or(Some(bytes.len()));
        item.payload = Some(payload_of(&self.config, &bytes));
        Ok(item)
    }

    async fn write_entry(
        &self,
        container: &str,
        entry: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        let db = container_db_index(container)?;
        self.select(db).await?;
        let kind = options
            .entry_kind
            .clone()
            .unwrap_or_else(|| "string".to_string());
        self.write_value(entry, &kind, payload, options.ttl).await?;
        Ok(MutationResult::with_message(format!(
            "saved {kind} key {entry}"
        )))
    }

    async fn delete_entry(&self, container: &str, entries: &[String]) -> AppResult<MutationResult> {
        if entries.is_empty() {
            return Err(AppError::invalid("no keys selected"));
        }
        let db = container_db_index(container)?;
        self.select(db).await?;
        let mut cmd = redis::cmd("DEL");
        for key in entries {
            cmd.arg(key);
        }
        let removed: i64 = self.run_command(&mut cmd).await?;
        Ok(MutationResult {
            ok: true,
            message: Some(format!("deleted {removed} key(s)")),
            affected: entries.to_vec(),
        })
    }

    async fn set_ttl(&self, container: &str, entry: &str, ttl_secs: i64) -> AppResult<()> {
        let db = container_db_index(container)?;
        self.select(db).await?;
        if ttl_secs <= 0 {
            self.run_command::<i64>(&mut redis::cmd("PERSIST").arg(entry))
                .await?;
        } else {
            self.run_command::<i64>(&mut redis::cmd("EXPIRE").arg(entry).arg(ttl_secs))
                .await?;
        }
        Ok(())
    }

    async fn tail(&self, container: &str) -> AppResult<TailHandle> {
        // Two tailing modes: a channel pattern (PSUBSCRIBE) or keyspace
        // notifications on a DB, expressed as `channel:<glob>` or
        // `keyspace:<glob>@<db>`.
        let (mode, target, db) = parse_tail_target(container)?;
        let endpoints = self.config.endpoints()?;
        let primary = endpoints[0].clone();
        let client = build_client(&self.config, &primary.host, primary.port, db)?;

        let mut pubsub = client.get_async_pubsub().await.map_err(map_redis_error)?;
        match mode.as_str() {
            "channel" => {
                pubsub
                    .psubscribe(target.clone())
                    .await
                    .map_err(map_redis_error)?;
            }
            "keyspace" => {
                // Keyspace notifications must be enabled server side before the
                // subscription produces anything.
                if self.select(db).await.is_ok() {
                    let _ = self
                        .run_command::<String>(
                            redis::cmd("CONFIG")
                                .arg("SET")
                                .arg("notify-keyspace-events")
                                .arg("KEA"),
                        )
                        .await;
                }
                let pattern = if target == "*" || target.is_empty() {
                    format!("__keyevent@{db}__:*")
                } else {
                    format!("__keyevent@{db}__:{target}")
                };
                pubsub.psubscribe(pattern).await.map_err(map_redis_error)?;
            }
            other => {
                return Err(AppError::invalid(format!(
                    "unknown redis tail mode {other}"
                )))
            }
        }

        let subscription_id = uuid::Uuid::new_v4().to_string();
        let (tx, rx) = mpsc::channel(512);
        let id_for_task = subscription_id.clone();

        tokio::spawn(async move {
            use futures_util::StreamExt;
            // `into_on_message` hands back an infallible stream of messages.
            let mut stream = pubsub.into_on_message();
            while let Some(message) = stream.next().await {
                let channel = message.get_channel_name().to_string();
                let payload = message.get_payload_bytes().to_vec();
                let out = StreamMessage {
                    subscription_id: id_for_task.clone(),
                    source: channel.clone(),
                    payload: crate::model::Payload::from_bytes(&payload, 512 * 1024),
                    key: None,
                    headers: BTreeMap::new(),
                    timestamp: chrono::Utc::now(),
                    position: None,
                };
                if tx.send(out).await.is_err() {
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
        let started = std::time::Instant::now();
        let parts = split_command(command);
        if parts.is_empty() {
            return Err(AppError::invalid("empty command"));
        }
        let mut cmd = redis::cmd(&parts[0]);
        for arg in &parts[1..] {
            cmd.arg(arg);
        }
        let result: AppResult<redis::Value> = self.run_command(&mut cmd).await;
        let elapsed_ms = started.elapsed().as_millis() as u64;
        match result {
            Ok(value) => Ok(ConsoleResult {
                output: render_redis_value(&value),
                error: false,
                elapsed_ms,
            }),
            Err(err) => Ok(ConsoleResult {
                output: err.message,
                error: true,
                elapsed_ms,
            }),
        }
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        let info = self.server_info().await?;
        let mut out = Vec::new();
        let mut section = String::from("server");
        for (key, value) in info {
            // INFO sections appear as pseudo-keys with empty values only when
            // parsed naively; our parser keeps them as section separators.
            if value.is_empty() && !key.contains('_') {
                section = key;
                continue;
            }
            out.push(StatItem {
                section: section.clone(),
                key,
                value,
            });
        }
        Ok(out)
    }

    async fn flush(&self, container: &str) -> AppResult<MutationResult> {
        let db = container_db_index(container)?;
        self.select(db).await?;
        self.run_command::<String>(&mut redis::cmd("FLUSHDB")).await?;
        Ok(MutationResult::with_message(format!(
            "flushed database {db}"
        )))
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}

// ---- helpers ----------------------------------------------------------

fn map_redis_error(err: redis::RedisError) -> AppError {
    let message = err.to_string();
    let kind = if err.is_io_error() {
        ErrorKind::Network
    } else if err.is_connection_refusal() {
        ErrorKind::Network
    } else if err.is_timeout() {
        ErrorKind::Timeout
    } else if message.contains("NOAUTH")
        || message.contains("WRONGPASS")
        || message.contains("NOPERM")
    {
        ErrorKind::Unauthorized
    } else if message.contains("unknown command") {
        ErrorKind::Unsupported
    } else {
        ErrorKind::ServerError
    };
    AppError::new(kind, message)
}

/// Build a redis client from a URL, which is the only constructor that can
/// express credentials, database and TLS together without touching private
/// fields.
fn build_client(
    config: &ConnectionConfig,
    host: &str,
    port: u16,
    db: i64,
) -> AppResult<redis::Client> {
    let url = RedisBackend::conn_args(config, host, port, db.max(0));

    // A custom CA bundle is only expressible through `build_with_tls`, which
    // wants the CA chain as PEM bytes rather than a path.
    let ca_file = config
        .tls_ca_file
        .as_ref()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty());

    match ca_file.filter(|_| config.tls.is_enabled()) {
        Some(path) => {
            let pem = std::fs::read(path)
                .map_err(|e| AppError::invalid(format!("cannot read TLS CA file {path}: {e}")))?;
            redis::Client::build_with_tls(
                url,
                TlsCertificates {
                    client_tls: None,
                    root_cert: Some(pem),
                },
            )
            .map_err(|e| AppError::driver(format!("invalid TLS settings: {e}")))
        }
        None => redis::Client::open(url.as_str())
            .map_err(|e| AppError::driver(format!("invalid connection settings: {e}"))),
    }
}

/// Percent-encode a URL userinfo component so credentials containing `@`, `/`
/// or `:` do not corrupt the connection URL.
fn percent_encode(value: &str) -> String {
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

/// Reject container ids that are not `db:<n>`.
fn container_db_index(container: &str) -> AppResult<i64> {
    let raw = container
        .strip_prefix("db:")
        .or_else(|| container.strip_prefix("db"))
        .unwrap_or(container);
    raw.parse::<i64>()
        .map_err(|_| AppError::invalid(format!("invalid redis database container: {container}")))
}

/// Container id for a database index, used by the UI.
pub fn db_container_id(index: i64) -> String {
    format!("db:{index}")
}

/// Parse `channel:<pattern>` / `keyspace:<pattern>[:db]` tail targets.
fn parse_tail_target(raw: &str) -> AppResult<(String, String, i64)> {
    let (mode, rest) = raw
        .split_once(':')
        .ok_or_else(|| AppError::invalid("tail target must look like `channel:<glob>` or `keyspace:<glob>`"))?;
    let mut db = 0i64;
    let mut target = rest.to_string();
    if mode == "keyspace" {
        // Allow `keyspace:pattern@db`.
        if let Some((pattern, db_part)) = rest.rsplit_once('@') {
            if let Ok(parsed) = db_part.parse::<i64>() {
                db = parsed;
                target = pattern.to_string();
            }
        }
        if target.is_empty() {
            target = "*".into();
        }
    } else if target.is_empty() {
        return Err(AppError::invalid("tail target pattern is required"));
    }
    Ok((mode.to_string(), target, db))
}

/// Resolve the current master of a sentinel deployment.
async fn resolve_sentinel_master(
    sentinel: &crate::config::Endpoint,
    master_name: &str,
    config: &ConnectionConfig,
    timeout: std::time::Duration,
) -> AppResult<(String, u16)> {
    // Sentinel authenticates separately from the data nodes but accepts the
    // same password in the common deployment, so reuse it.
    let client = build_client(config, &sentinel.host, sentinel.port, 0)?;

    let mut conn = tokio::time::timeout(timeout, client.get_multiplexed_async_connection())
        .await
        .map_err(|_| AppError::network("sentinel connection timed out"))?
        .map_err(map_redis_error)?;

    let reply: Vec<String> = redis::cmd("SENTINEL")
        .arg("get-master-addr-by-name")
        .arg(master_name)
        .query_async(&mut conn)
        .await
        .map_err(|e| {
            AppError::server(format!(
                "sentinel does not know master {master_name}: {e}"
            ))
        })?;
    if reply.len() != 2 {
        return Err(AppError::server(format!(
            "sentinel returned an unexpected reply for {master_name}"
        )));
    }
    let port = reply[1]
        .parse::<u16>()
        .map_err(|_| AppError::server("sentinel returned an invalid port"))?;
    Ok((reply[0].clone(), port))
}

/// Parse the `key:value` lines of an INFO reply, using section headers as the
/// section key with an empty value.
pub fn parse_info(raw: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Some(section) = line.strip_prefix('#') {
            out.insert(section.trim().to_string(), String::new());
        } else if let Some((key, value)) = line.split_once(':') {
            out.insert(key.to_string(), value.to_string());
        }
    }
    out
}

fn quote_if_needed(value: &str) -> String {
    if value.chars().any(|c| c.is_whitespace() || c == '"') || value.is_empty() {
        format!("{value:?}")
    } else {
        format!("\"{value}\"")
    }
}

fn parse_json_object(text: &str) -> Result<BTreeMap<String, String>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let object = value.as_object().ok_or("not an object")?;
    Ok(object
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                match v {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                },
            )
        })
        .collect())
}

fn parse_json_array(text: &str) -> Result<Vec<String>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let array = value.as_array().ok_or("not an array")?;
    Ok(array
        .iter()
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .collect())
}

fn parse_zset(text: &str) -> Result<Vec<(String, f64)>, String> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    let array = value.as_array().ok_or("not an array")?;
    let mut out = Vec::new();
    for item in array {
        match item {
            serde_json::Value::Object(_) => {
                let member = item
                    .get("member")
                    .ok_or("zset objects require a member field")?
                    .as_str()
                    .ok_or("member must be a string")?
                    .to_string();
                let score = item
                    .get("score")
                    .ok_or("zset objects require a score field")?
                    .as_f64()
                    .ok_or("score must be a number")?;
                out.push((member, score));
            }
            serde_json::Value::Array(pair) if pair.len() == 2 => {
                let member = pair[0]
                    .as_str()
                    .ok_or("zset pair member must be a string")?
                    .to_string();
                let score = pair[1].as_f64().ok_or("zset pair score must be numeric")?;
                out.push((member, score));
            }
            _ => return Err("unsupported zset element".into()),
        }
    }
    Ok(out)
}

/// Split a command line on whitespace while honouring single and double quotes.
pub fn split_command(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut quote: Option<char> = None;
    let mut has_token = false;

    for ch in input.chars() {
        match quote {
            Some(q) => {
                if ch == q {
                    quote = None;
                } else if ch == '\\' {
                    // Keep the escape for the next iteration.
                    current.push(ch);
                } else {
                    current.push(ch);
                }
            }
            None => match ch {
                '\'' | '"' => {
                    quote = Some(ch);
                    has_token = true;
                }
                c if c.is_whitespace() => {
                    if has_token || !current.is_empty() {
                        out.push(std::mem::take(&mut current));
                        has_token = false;
                    }
                }
                c => {
                    current.push(c);
                    has_token = true;
                }
            },
        }
    }
    if !current.is_empty() || has_token {
        out.push(current);
    }
    out
}

fn render_redis_value(value: &redis::Value) -> String {
    match value {
        redis::Value::Nil => "(nil)".into(),
        redis::Value::Int(i) => format!("(integer) {i}"),
        redis::Value::BulkString(bytes) => {
            match String::from_utf8(bytes.clone()) {
                Ok(text) if text.chars().all(|c| !c.is_control() || c == '\n' || c == '\t') => {
                    format!("\"{text}\"")
                }
                _ => {
                    use base64::Engine as _;
                    format!(
                        "(binary) {}",
                        base64::engine::general_purpose::STANDARD.encode(bytes)
                    )
                }
            }
        }
        redis::Value::SimpleString(s) => s.clone(),
        redis::Value::Okay => "OK".into(),
        redis::Value::Array(items) => {
            if items.is_empty() {
                return "(empty array)".into();
            }
            let mut out = String::new();
            for (index, item) in items.iter().enumerate() {
                out.push_str(&format!("{}) {}\n", index + 1, render_redis_value(item)));
            }
            out.trim_end().to_string()
        }
        redis::Value::Map(pairs) => {
            let mut out = String::new();
            for (index, (key, val)) in pairs.iter().enumerate() {
                out.push_str(&format!(
                    "{}) {} => {}\n",
                    index + 1,
                    render_redis_value(key),
                    render_redis_value(val)
                ));
            }
            out.trim_end().to_string()
        }
        redis::Value::Set(items) => render_redis_value(&redis::Value::Array(items.clone())),
        redis::Value::Double(d) => format!("(double) {d}"),
        redis::Value::Boolean(b) => format!("#{b}"),
        redis::Value::VerbatimString { format, text } => format!("({format:?}) {text}"),
        redis::Value::BigNumber(n) => format!("(bignum) {}", String::from_utf8_lossy(n)),
        redis::Value::Push { kind, data } => {
            format!("(push {kind:?}) {}", render_redis_value(&redis::Value::Array(data.clone())))
        }
        redis::Value::Attribute { data, attributes } => format!(
            "{}\n(attributes: {})",
            render_redis_value(data),
            render_redis_value(&redis::Value::Map(attributes.clone()))
        ),
        other => format!("{other:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_container_ids() {
        assert_eq!(container_db_index("db:3").unwrap(), 3);
        assert_eq!(container_db_index("db0").unwrap(), 0);
        assert_eq!(container_db_index("7").unwrap(), 7);
        assert!(container_db_index("keyspace").is_err());
    }

    #[test]
    fn builds_container_ids() {
        assert_eq!(db_container_id(2), "db:2");
    }

    #[test]
    fn parses_info_sections() {
        let raw = "# Server\r\nredis_version:7.2.4\r\nuptime_in_seconds:1000\r\n\r\n# Memory\r\nused_memory:12345\r\n";
        let parsed = parse_info(raw);
        assert_eq!(parsed.get("redis_version").map(String::as_str), Some("7.2.4"));
        assert_eq!(parsed.get("used_memory").map(String::as_str), Some("12345"));
        assert_eq!(parsed.get("Server").map(String::as_str), Some(""));
        assert_eq!(parsed.get("Memory").map(String::as_str), Some(""));
    }

    #[test]
    fn splits_quoted_commands() {
        assert_eq!(
            split_command(r#"SET greeting "hello world""#),
            vec!["SET", "greeting", "hello world"]
        );
        assert_eq!(split_command("  GET  key  "), vec!["GET", "key"]);
        assert_eq!(split_command("SET k ''"), vec!["SET", "k", ""]);
        assert!(split_command("   ").is_empty());
    }

    #[test]
    fn parses_tail_targets() {
        let (mode, target, db) = parse_tail_target("channel:orders.*").unwrap();
        assert_eq!(mode, "channel");
        assert_eq!(target, "orders.*");
        assert_eq!(db, 0);

        let (mode, target, db) = parse_tail_target("keyspace:*@2").unwrap();
        assert_eq!(mode, "keyspace");
        assert_eq!(target, "*");
        assert_eq!(db, 2);

        assert!(parse_tail_target("nope").is_err());
    }

    #[test]
    fn parses_hash_payloads() {
        let map = parse_json_object(r#"{"a":"1","b":2}"#).unwrap();
        assert_eq!(map.get("a").map(String::as_str), Some("1"));
        assert_eq!(map.get("b").map(String::as_str), Some("2"));
        assert!(parse_json_object("[1,2]").is_err());
    }

    #[test]
    fn parses_zset_payloads() {
        let pairs = parse_zset(r#"[{"member":"a","score":1.5},["b",2]]"#).unwrap();
        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].0, "a");
        assert_eq!(pairs[0].1, 1.5);
        assert_eq!(pairs[1].0, "b");
        assert_eq!(pairs[1].1, 2.0);
    }
}
