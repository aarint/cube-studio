//! Minimal memcached ASCII-protocol client.
//!
//! The off-the-shelf Rust clients expose `get`/`set`/`delete` but deliberately
//! hide the introspection commands (`stats items`, `stats cachedump`,
//! `lru_crawler metadump`) that a GUI browser needs in order to show which keys
//! exist. Rather than fight a higher level API, this module speaks the wire
//! protocol directly. It is intentionally small: one request per connection,
//! responses framed by `END` or a fixed byte count.
//!
//! Reference: <https://github.com/memcached/memcached/blob/master/doc/protocol.txt>

use std::collections::BTreeMap;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

use crate::error::{AppError, AppResult};

/// A pooled connection to one memcached node.
struct Conn {
    reader: BufReader<TcpStream>,
}

/// Protocol client with a tiny connection pool.
pub struct MemcachedProto {
    nodes: Vec<String>,
    timeout: std::time::Duration,
    pool: Mutex<Vec<Conn>>,
    /// Round-robin cursor so reads spread across nodes.
    next_node: std::sync::atomic::AtomicUsize,
}

/// A value read from the cache.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub bytes: Vec<u8>,
    pub flags: u32,
}

/// One key discovered by an enumeration pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredKey {
    pub name: String,
    /// Remaining lifetime in seconds, `-1` for "never".
    pub ttl: Option<i64>,
    pub size: Option<usize>,
}

impl MemcachedProto {
    pub fn new(nodes: Vec<String>, timeout: std::time::Duration) -> Self {
        Self {
            nodes,
            timeout,
            pool: Mutex::new(Vec::new()),
            next_node: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// Check out a connection, creating one if the pool is empty.
    async fn checkout(&self) -> AppResult<Conn> {
        if let Some(conn) = self.pool.lock().await.pop() {
            return Ok(conn);
        }
        self.dial().await
    }

    async fn checkin(&self, conn: Conn) {
        let mut pool = self.pool.lock().await;
        if pool.len() < 8 {
            pool.push(conn);
        }
    }

    async fn dial(&self) -> AppResult<Conn> {
        if self.nodes.is_empty() {
            return Err(AppError::invalid("no memcached node configured"));
        }
        let index = self
            .next_node
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            % self.nodes.len();
        let node = &self.nodes[index];

        let stream = tokio::time::timeout(self.timeout, TcpStream::connect(node))
            .await
            .map_err(|_| AppError::network(format!("timed out connecting to {node}")))?
            .map_err(|e| AppError::network(format!("cannot connect to {node}: {e}")))?;
        stream.set_nodelay(true).ok();
        Ok(Conn {
            reader: BufReader::new(stream),
        })
    }

    /// Round-trip one command, returning the first line and any body lines
    /// terminated by `END`.
    async fn roundtrip(&self, command: &str) -> AppResult<String> {
        let mut conn = self.checkout().await?;
        let result = self.roundtrip_on(&mut conn, command).await;
        // A protocol desync would leave the socket unusable, so on error the
        // connection is dropped rather than returned to the pool.
        if result.is_ok() {
            self.checkin(conn).await;
        }
        result
    }

    async fn roundtrip_on(&self, conn: &mut Conn, command: &str) -> AppResult<String> {
        let write = async {
            conn.reader.get_mut().write_all(command.as_bytes()).await?;
            conn.reader.get_mut().write_all(b"\r\n").await?;
            conn.reader.get_mut().flush().await
        };
        tokio::time::timeout(self.timeout, write)
            .await
            .map_err(|_| AppError::new(crate::error::ErrorKind::Timeout, "memcached write timed out"))?
            .map_err(|e| AppError::network(format!("memcached write failed: {e}")))?;

        let mut out = String::new();
        let read = async {
            loop {
                let mut line = String::new();
                let n = conn.reader.read_line(&mut line).await?;
                if n == 0 {
                    break;
                }
                let trimmed = line.trim_end_matches(['\r', '\n']).to_string();
                if trimmed == "END" {
                    break;
                }
                out.push_str(&trimmed);
                out.push('\n');
                // Error replies are always a single line, so stop there instead
                // of waiting for an END that will never arrive.
                if trimmed.starts_with("ERROR")
                    || trimmed.starts_with("CLIENT_ERROR")
                    || trimmed.starts_with("SERVER_ERROR")
                {
                    break;
                }
            }
            Ok::<(), std::io::Error>(())
        };
        tokio::time::timeout(self.timeout, read)
            .await
            .map_err(|_| AppError::new(crate::error::ErrorKind::Timeout, "memcached read timed out"))?
            .map_err(|e| AppError::network(format!("memcached read failed: {e}")))?;

        Ok(out)
    }

    /// Send a command whose reply is exactly one line.
    ///
    /// Storage, delete and touch all answer with a single status token; reading
    /// them with [`roundtrip`] would block waiting for a terminator that never
    /// comes.
    async fn roundtrip_line(&self, command: &str) -> AppResult<String> {
        let mut conn = self.checkout().await?;
        let result = async {
            let write = async {
                conn.reader.get_mut().write_all(command.as_bytes()).await?;
                conn.reader.get_mut().write_all(b"\r\n").await?;
                conn.reader.get_mut().flush().await
            };
            tokio::time::timeout(self.timeout, write)
                .await
                .map_err(|_| {
                    AppError::new(crate::error::ErrorKind::Timeout, "memcached write timed out")
                })?
                .map_err(|e| AppError::network(format!("memcached write failed: {e}")))?;

            let mut line = String::new();
            tokio::time::timeout(self.timeout, conn.reader.read_line(&mut line))
                .await
                .map_err(|_| {
                    AppError::new(crate::error::ErrorKind::Timeout, "memcached read timed out")
                })?
                .map_err(|e| AppError::network(format!("memcached read failed: {e}")))?;
            Ok::<String, AppError>(line.trim_end_matches(['\r', '\n']).to_string())
        }
        .await;

        // Status replies keep the socket in sync, so it can be reused.
        if result.is_ok() {
            self.checkin(conn).await;
        }
        result
    }

    /// `version` — also used as the connection health check.
    ///
    /// The reply is a single `VERSION <x.y.z>` line with no terminator, so it
    /// must be read with the single-line helper.
    pub async fn version(&self) -> AppResult<String> {
        let reply = self.roundtrip_line("version").await?;
        Ok(reply
            .strip_prefix("VERSION ")
            .unwrap_or(&reply)
            .trim()
            .to_string())
    }

    /// Fetch one key.
    pub async fn get(&self, key: &str) -> AppResult<Option<Item>> {
        let reply = self.roundtrip(&format!("get {key}")).await?;
        Ok(parse_value_block(&reply))
    }

    /// Store one value. `exptime` is seconds; `0` means never expire.
    pub async fn set(&self, key: &str, value: &[u8], flags: u32, exptime: u32) -> AppResult<()> {
        let header = format!("set {key} {flags} {exptime} {}\r\n", value.len());
        let mut conn = self.checkout().await?;
        let result = async {
            let write = async {
                conn.reader.get_mut().write_all(header.as_bytes()).await?;
                conn.reader.get_mut().write_all(value).await?;
                conn.reader.get_mut().write_all(b"\r\n").await?;
                conn.reader.get_mut().flush().await
            };
            tokio::time::timeout(self.timeout, write)
                .await
                .map_err(|_| {
                    AppError::new(crate::error::ErrorKind::Timeout, "memcached write timed out")
                })?
                .map_err(|e| AppError::network(format!("memcached write failed: {e}")))?;

            let mut line = String::new();
            tokio::time::timeout(self.timeout, conn.reader.read_line(&mut line))
                .await
                .map_err(|_| {
                    AppError::new(crate::error::ErrorKind::Timeout, "memcached read timed out")
                })?
                .map_err(|e| AppError::network(format!("memcached read failed: {e}")))?;
            Ok::<String, AppError>(line.trim_end_matches(['\r', '\n']).to_string())
        }
        .await;

        let status = match result {
            Ok(status) => {
                self.checkin(conn).await;
                status
            }
            Err(err) => return Err(err),
        };

        match status.as_str() {
            "STORED" => Ok(()),
            other => Err(AppError::server(map_status_error(other))),
        }
    }

    /// Delete one key. Returns `true` when it existed.
    pub async fn delete(&self, key: &str) -> AppResult<bool> {
        let status = self.roundtrip_line(&format!("delete {key}")).await?;
        match status.as_str() {
            "DELETED" => Ok(true),
            "NOT_FOUND" => Ok(false),
            other => Err(AppError::server(map_status_error(other))),
        }
    }

    /// Change the expiry of an existing key without rewriting its bytes.
    pub async fn touch(&self, key: &str, exptime: u32) -> AppResult<bool> {
        let status = self.roundtrip_line(&format!("touch {key} {exptime}")).await?;
        match status.as_str() {
            "TOUCHED" => Ok(true),
            "NOT_FOUND" => Ok(false),
            other => Err(AppError::server(map_status_error(other))),
        }
    }

    pub async fn flush_all(&self) -> AppResult<()> {
        let status = self.roundtrip_line("flush_all").await?;
        match status.as_str() {
            "OK" | "RESET" => Ok(()),
            other => Err(AppError::server(map_status_error(other))),
        }
    }

    /// `stats [section]`, flattened.
    pub async fn stats(&self, section: &str) -> AppResult<BTreeMap<String, String>> {
        let command = if section.is_empty() {
            "stats".to_string()
        } else {
            format!("stats {section}")
        };
        let reply = self.roundtrip(&command).await?;
        Ok(parse_stat_lines(&reply))
    }

    /// Preferred enumeration: complete key list with metadata, when the server
    /// has the LRU crawler enabled (memcached >= 1.5 with `-o lru_crawler`).
    pub async fn metadump(&self) -> AppResult<Vec<DiscoveredKey>> {
        let reply = self.roundtrip("lru_crawler metadump all").await?;
        if reply.trim().is_empty() {
            return Err(AppError::unsupported("lru_crawler metadump returned nothing"));
        }
        if reply.starts_with("CLIENT_ERROR") || reply.starts_with("ERROR") {
            return Err(AppError::unsupported(reply.trim().to_string()));
        }

        let mut keys = Vec::new();
        for line in reply.lines() {
            if !line.starts_with("key=") {
                continue;
            }
            let fields: BTreeMap<String, String> = line
                .split_whitespace()
                .filter_map(|token| token.split_once('='))
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            let Some(name) = fields.get("key") else {
                continue;
            };
            keys.push(DiscoveredKey {
                name: percent_decode(name),
                ttl: fields
                    .get("exp")
                    .and_then(|v| v.parse::<i64>().ok())
                    .map(|exp| if exp < 0 { -1 } else { exp }),
                size: fields.get("size").and_then(|v| v.parse::<usize>().ok()),
            });
        }
        Ok(keys)
    }

    /// Fallback enumeration through `stats items` + `stats cachedump`.
    ///
    /// `cachedump` output is capped at 1 MiB by the server, so very large
    /// instances return a partial list. Callers surface this in the UI.
    pub async fn cachedump(&self) -> AppResult<Vec<DiscoveredKey>> {
        let items = self.stats("items").await?;
        let mut slabs: Vec<(String, i64)> = Vec::new();
        for (key, value) in &items {
            if let Some(slab) = key
                .strip_prefix("items:")
                .and_then(|rest| rest.strip_suffix(":number"))
            {
                if let Ok(count) = value.parse::<i64>() {
                    if count > 0 {
                        slabs.push((slab.to_string(), count));
                    }
                }
            }
        }
        if slabs.is_empty() {
            return Err(AppError::unsupported(
                "the server exposes no slab statistics, so keys cannot be enumerated",
            ));
        }

        let mut keys = Vec::new();
        for (slab, count) in slabs {
            // Ask for what the slab holds, capped to keep the reply manageable.
            let limit = count.min(10_000);
            let reply = match self
                .roundtrip(&format!("stats cachedump {slab} {limit}"))
                .await
            {
                Ok(reply) => reply,
                Err(_) => continue,
            };
            for line in reply.lines() {
                let Some(rest) = line.strip_prefix("ITEM ") else {
                    continue;
                };
                // ITEM foo [12 b; 1699999999 s]
                let mut parts = rest.split_whitespace();
                let Some(name) = parts.next() else { continue };
                let mut size = None;
                let mut ttl = None;
                let tail: Vec<&str> = parts.collect();
                if tail.len() >= 3 {
                    size = tail[0].trim_start_matches('[').parse::<usize>().ok();
                    ttl = tail[2].parse::<i64>().ok();
                }
                keys.push(DiscoveredKey {
                    name: percent_decode(name),
                    ttl,
                    size,
                });
            }
        }
        keys.sort_by(|a, b| a.name.cmp(&b.name));
        keys.dedup_by(|a, b| a.name == b.name);
        Ok(keys)
    }

    /// Run an arbitrary command for the console panel.
    pub async fn raw(&self, command: &str) -> AppResult<String> {
        self.roundtrip(command).await
    }
}

fn map_status_error(status: &str) -> String {
    if let Some(rest) = status.strip_prefix("SERVER_ERROR ") {
        return rest.to_string();
    }
    if let Some(rest) = status.strip_prefix("CLIENT_ERROR ") {
        return format!("client error: {rest}");
    }
    match status {
        "NOT_STORED" => "the item was not stored (add/replace condition failed)".into(),
        "EXISTS" => "the item already exists".into(),
        "NOT_FOUND" => "the item does not exist".into(),
        "ERROR" => "unknown command".into(),
        other if other.is_empty() => "the server closed the connection".into(),
        other => other.to_string(),
    }
}

/// Parse a `VALUE <key> <flags> <bytes>\r\n<data>\r\nEND` block.
fn parse_value_block(reply: &str) -> Option<Item> {
    let mut lines = reply.lines();
    let header = lines.next()?;
    let mut parts = header.split_whitespace();
    if parts.next()? != "VALUE" {
        return None;
    }
    let _key = parts.next()?;
    let flags = parts.next().and_then(|f| f.parse::<u32>().ok()).unwrap_or(0);
    let bytes = parts.next().and_then(|b| b.parse::<usize>().ok()).unwrap_or(0);
    // The data may itself contain newlines, so re-join the remaining lines and
    // slice by the declared length.
    let rest: Vec<&str> = lines.collect();
    let joined = rest.join("\n");
    let data = joined.as_bytes();
    let end = bytes.min(data.len());
    Some(Item {
        bytes: data[..end].to_vec(),
        flags,
    })
}

fn parse_stat_lines(reply: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in reply.lines() {
        let Some(rest) = line.strip_prefix("STAT ") else {
            continue;
        };
        let (key, value) = match rest.split_once(' ') {
            Some(pair) => pair,
            None => (rest, ""),
        };
        out.insert(key.to_string(), value.to_string());
    }
    out
}

/// `lru_crawler metadump` percent-encodes keys.
fn percent_decode(raw: &str) -> String {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' && index + 2 < bytes.len() {
            if let Ok(hex) = std::str::from_utf8(&bytes[index + 1..index + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex, 16) {
                    out.push(byte);
                    index += 3;
                    continue;
                }
            }
        }
        out.push(bytes[index]);
        index += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_value_block() {
        let item = parse_value_block("VALUE user:1 0 5\r\nhello\r\nEND\n").unwrap();
        assert_eq!(item.bytes, b"hello");
        assert_eq!(item.flags, 0);
    }

    #[test]
    fn parses_value_block_with_newlines() {
        let item = parse_value_block("VALUE blob 4 11\r\nline1\nline2\r\nEND\n").unwrap();
        assert_eq!(item.bytes, b"line1\nline2");
        assert_eq!(item.flags, 4);
    }

    #[test]
    fn returns_none_for_miss() {
        assert!(parse_value_block("END\n").is_none());
    }

    #[test]
    fn parses_stat_lines() {
        let stats = parse_stat_lines("STAT pid 1234\nSTAT version 1.6.21\nSTAT slab_without_value\n");
        assert_eq!(stats.get("pid").map(String::as_str), Some("1234"));
        assert_eq!(stats.get("version").map(String::as_str), Some("1.6.21"));
        assert_eq!(stats.get("slab_without_value").map(String::as_str), Some(""));
    }

    #[test]
    fn decodes_percent_keys() {
        assert_eq!(percent_decode("user%3A1"), "user:1");
        assert_eq!(percent_decode("plain"), "plain");
        assert_eq!(percent_decode("bad%zz"), "bad%zz");
    }

    #[test]
    fn maps_server_errors() {
        assert_eq!(map_status_error("SERVER_ERROR out of memory"), "out of memory");
        assert!(map_status_error("NOT_FOUND").contains("does not exist"));
    }
}
