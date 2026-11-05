//! Memcached backend, built on the local ASCII-protocol client.
//!
//! Memcached deliberately offers no key enumeration command. The backend
//! reconstructs the key list from the introspection facilities the server does
//! have, and reports the limitation instead of pretending the list is complete.

use async_trait::async_trait;

use crate::backend::memcached_proto::{MemcachedProto, DiscoveredKey};
use crate::backend::{Backend, ConnectionInfo};
use crate::config::{BackendKind, ConnectionConfig};
use crate::error::{AppError, AppResult};
use crate::model::{
    Capabilities, ConsoleResult, Container, ContainerKind, Entry, EntryPage, MutationResult,
    Payload, StatItem, WriteOptions,
};

pub struct MemcachedBackend {
    config: ConnectionConfig,
    proto: MemcachedProto,
    server_version: String,
}

impl MemcachedBackend {
    pub async fn connect(config: ConnectionConfig) -> AppResult<Self> {
        let config = config.normalize()?;
        let nodes: Vec<String> = config
            .endpoints()?
            .iter()
            .map(|endpoint| endpoint.authority())
            .collect();
        let proto = MemcachedProto::new(nodes, config.timeout());
        let server_version = proto.version().await?;
        Ok(Self {
            config,
            proto,
            server_version,
        })
    }

    async fn discover(&self) -> AppResult<(Vec<DiscoveredKey>, bool)> {
        // `true` in the second slot means the list came from the partial
        // cachedump fallback and is therefore incomplete.
        match self.proto.metadump().await {
            Ok(keys) => Ok((keys, false)),
            Err(_) => self.proto.cachedump().await.map(|keys| (keys, true)),
        }
    }
}

#[async_trait]
impl Backend for MemcachedBackend {
    async fn ping(&self) -> AppResult<()> {
        self.proto.version().await.map(|_| ())
    }

    async fn info(&self) -> AppResult<ConnectionInfo> {
        let mut capabilities = Capabilities::for_kind(BackendKind::Memcached);
        match self.discover().await {
            Ok((_, partial)) if partial => {
                capabilities.notes.push(
                    "Key discovery fell back to `stats cachedump`, which the server caps \
                     at 1 MiB. Only a subset of keys is listed."
                        .into(),
                );
            }
            Err(_) => {
                capabilities.list_entries = false;
                capabilities.notes.push(
                    "This server exposes neither `lru_crawler metadump` nor slab statistics, \
                     so keys cannot be listed. You can still open keys by name."
                        .into(),
                );
            }
            _ => {}
        }
        Ok(ConnectionInfo {
            kind: BackendKind::Memcached,
            server_version: Some(self.server_version.clone()),
            capabilities,
            default_container: Some("server".into()),
        })
    }

    async fn containers(&self) -> AppResult<Vec<Container>> {
        let stats = self.proto.stats("").await.unwrap_or_default();
        let mut meta = std::collections::BTreeMap::new();
        for key in [
            "curr_items",
            "total_items",
            "curr_connections",
            "bytes",
            "limit_maxbytes",
            "evictions",
        ] {
            if let Some(value) = stats.get(key) {
                meta.insert(key.to_string(), value.clone());
            }
        }
        let size = stats.get("curr_items").and_then(|v| v.parse::<i64>().ok());
        Ok(vec![Container {
            id: "server".into(),
            name: self.config.display_name(),
            kind: ContainerKind::MemcachedServer,
            meta,
            size,
        }])
    }

    async fn entries(
        &self,
        _container: &str,
        _cursor: Option<&str>,
        limit: usize,
        search: Option<&str>,
    ) -> AppResult<EntryPage> {
        let (keys, _partial) = self.discover().await?;
        let needle = search.unwrap_or("").trim().to_lowercase();
        let filtered: Vec<&DiscoveredKey> = if needle.is_empty() {
            keys.iter().collect()
        } else {
            keys.iter()
                .filter(|key| key.name.to_lowercase().contains(&needle))
                .collect()
        };

        let total = filtered.len() as i64;
        let entries = filtered
            .into_iter()
            .take(limit.max(1))
            .map(|info| {
                let mut entry = Entry::new(info.name.clone(), info.name.clone(), "string");
                entry.ttl = info.ttl;
                entry.size = info.size;
                entry
            })
            .collect();

        Ok(EntryPage {
            entries,
            next_cursor: None,
            total: Some(total),
        })
    }

    async fn read_entry(&self, _container: &str, entry: &str) -> AppResult<Entry> {
        let item = self.proto.get(entry).await?.ok_or_else(|| {
            AppError::server(format!("key {entry} is not in the cache (expired or evicted)"))
        })?;

        // `gets`/meta would give the TTL; ask for it via a touch-free path by
        // inspecting the metadump entry. Falling back to "unknown" is fine.
        let ttl = None;

        let mut out = Entry::new(entry, entry, "string");
        out.size = Some(item.bytes.len());
        out.ttl = ttl;
        out.payload = Some(Payload::from_bytes(&item.bytes, self.config.max_payload()));
        Ok(out)
    }

    async fn write_entry(
        &self,
        _container: &str,
        entry: &str,
        payload: &[u8],
        options: &WriteOptions,
    ) -> AppResult<MutationResult> {
        let exptime = options.ttl.unwrap_or(0).max(0) as u32;
        self.proto.set(entry, payload, 0, exptime).await?;
        Ok(MutationResult::with_message(format!("stored {entry}")))
    }

    async fn delete_entry(&self, _container: &str, entries: &[String]) -> AppResult<MutationResult> {
        if entries.is_empty() {
            return Err(AppError::invalid("no keys selected"));
        }
        let mut deleted = Vec::new();
        let mut failures = Vec::new();
        for key in entries {
            match self.proto.delete(key).await {
                Ok(true) => deleted.push(key.clone()),
                Ok(false) => failures.push(format!("{key}: not found")),
                Err(err) => failures.push(format!("{key}: {}", err.message)),
            }
        }
        Ok(MutationResult {
            ok: failures.is_empty(),
            message: Some(if failures.is_empty() {
                format!("deleted {} key(s)", deleted.len())
            } else {
                format!(
                    "deleted {} key(s); problems: {}",
                    deleted.len(),
                    failures.join("; ")
                )
            }),
            affected: deleted,
        })
    }

    async fn set_ttl(&self, _container: &str, entry: &str, ttl_secs: i64) -> AppResult<()> {
        let touched = self.proto.touch(entry, ttl_secs.max(0) as u32).await?;
        if !touched {
            return Err(AppError::server(format!(
                "key {entry} is not in the cache"
            )));
        }
        Ok(())
    }

    async fn console(&self, command: &str) -> AppResult<ConsoleResult> {
        let started = std::time::Instant::now();
        let command = command.trim();
        if command.is_empty() {
            return Err(AppError::invalid("empty command"));
        }
        // `get` is by far the most common ad-hoc command and its reply contains
        // the raw payload, which is exactly what a user wants to see.
        match self.proto.raw(command).await {
            Ok(output) => Ok(ConsoleResult {
                output,
                error: false,
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
            Err(err) => Ok(ConsoleResult {
                output: err.message,
                error: true,
                elapsed_ms: started.elapsed().as_millis() as u64,
            }),
        }
    }

    async fn stats(&self) -> AppResult<Vec<StatItem>> {
        let mut out = Vec::new();
        for section in ["", "settings", "items", "slabs", "sizes"] {
            let Ok(values) = self.proto.stats(section).await else {
                continue;
            };
            let label = if section.is_empty() { "general" } else { section };
            for (key, value) in values {
                out.push(StatItem {
                    section: label.to_string(),
                    key,
                    value,
                });
            }
        }
        if out.is_empty() {
            return Err(AppError::server("the server returned no statistics"));
        }
        Ok(out)
    }

    async fn flush(&self, _container: &str) -> AppResult<MutationResult> {
        self.proto.flush_all().await?;
        Ok(MutationResult::with_message("cache flushed"))
    }

    fn config(&self) -> &ConnectionConfig {
        &self.config
    }
}
