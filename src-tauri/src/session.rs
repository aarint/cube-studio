//! Live session registry.
//!
//! The frontend identifies a connection by a session id (not a profile id), so
//! you can open the same profile twice, or connect without saving at all.
//! Sessions own their `Backend` and any active subscription pumps; closing a
//! session awaits a clean shutdown so sockets and consumer tasks go away.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::{Mutex, RwLock};

use crate::backend::{Backend, ConnectionInfo};
use crate::config::{BackendKind, ConnectionConfig};
use crate::error::{AppError, AppResult};

/// Metadata the UI needs to render a tab for a session.
#[derive(Debug, Clone, serde::Serialize)]
pub struct SessionDescriptor {
    pub session_id: String,
    pub profile_id: Option<String>,
    pub name: String,
    pub kind: BackendKind,
    pub endpoints: Vec<String>,
    pub info: ConnectionInfo,
}

/// Everything the registry keeps for one open connection.
struct Session {
    backend: Arc<dyn Backend>,
    config: ConnectionConfig,
    /// Monotonic connection generation; used for ordering in the UI.
    generation: u64,
}

/// Thread-safe registry of open connections.
#[derive(Default)]
pub struct SessionRegistry {
    sessions: RwLock<HashMap<String, Session>>,
    counter: AtomicU64,
}

impl SessionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Connect and register a new session.
    ///
    /// `profile_id` is stored so the UI can highlight which saved profile a tab
    /// came from; it is not used for lookups.
    pub async fn open(
        &self,
        config: ConnectionConfig,
        profile_id: Option<String>,
    ) -> AppResult<SessionDescriptor> {
        let config = config.normalize()?;
        let backend = crate::backend::connect(&config).await?;
        let info = backend.info().await?;

        let session_id = uuid::Uuid::new_v4().to_string();
        let generation = self.counter.fetch_add(1, Ordering::Relaxed);
        let descriptor = SessionDescriptor {
            session_id: session_id.clone(),
            profile_id: profile_id.clone(),
            name: config.display_name(),
            kind: config.kind,
            endpoints: config
                .endpoints()
                .map(|eps| eps.iter().map(|e| e.authority()).collect())
                .unwrap_or_default(),
            info,
        };

        self.sessions.write().await.insert(
            session_id,
            Session {
                backend: backend.into(),
                config,
                generation,
            },
        );
        tracing::info!(
            kind = %descriptor.kind,
            session = %descriptor.session_id,
            generation,
            "session opened"
        );
        Ok(descriptor)
    }

    /// Get a handle to the session's backend.
    ///
    /// Returning the `Arc` rather than running a closure lets callers hold it
    /// across awaits without threading lifetimes through every command.
    pub async fn backend(&self, session_id: &str) -> AppResult<Arc<dyn Backend>> {
        let guard = self.sessions.read().await;
        guard
            .get(session_id)
            .map(|session| session.backend.clone())
            .ok_or_else(|| AppError::no_session(format!("session {session_id} is not open")))
    }

    pub async fn descriptor(&self, session_id: &str) -> AppResult<SessionDescriptor> {
        let guard = self.sessions.read().await;
        let session = guard
            .get(session_id)
            .ok_or_else(|| AppError::no_session(format!("session {session_id} is not open")))?;
        Ok(SessionDescriptor {
            session_id: session_id.to_string(),
            profile_id: None,
            name: session.config.display_name(),
            kind: session.config.kind,
            endpoints: session
                .config
                .endpoints()
                .map(|eps| eps.iter().map(|e| e.authority()).collect())
                .unwrap_or_default(),
            info: session.backend.info().await?,
        })
    }

    /// List all open sessions in creation order.
    pub async fn list(&self) -> Vec<SessionDescriptor> {
        let guard = self.sessions.read().await;
        let mut ordered: Vec<_> = guard.iter().collect();
        ordered.sort_by_key(|(_, session)| session.generation);
        let mut out = Vec::with_capacity(ordered.len());
        for (id, session) in ordered {
            out.push(SessionDescriptor {
                session_id: id.clone(),
                profile_id: None,
                name: session.config.display_name(),
                kind: session.config.kind,
                endpoints: session
                    .config
                    .endpoints()
                    .map(|eps| eps.iter().map(|e| e.authority()).collect())
                    .unwrap_or_default(),
                info: match session.backend.info().await {
                    Ok(info) => info,
                    Err(_) => ConnectionInfo {
                        kind: session.config.kind,
                        server_version: None,
                        capabilities: crate::model::Capabilities::for_kind(session.config.kind),
                        default_container: None,
                    },
                },
            });
        }
        out
    }

    /// Close a session, shutting its backend down first.
    pub async fn close(&self, session_id: &str) -> AppResult<()> {
        let removed = self.sessions.write().await.remove(session_id);
        match removed {
            Some(session) => {
                session.backend.shutdown().await;
                tracing::info!(session = %session_id, "session closed");
                Ok(())
            }
            None => Ok(()),
        }
    }

    /// Close every session (used on app shutdown).
    pub async fn close_all(&self) {
        let sessions = {
            let mut guard = self.sessions.write().await;
            std::mem::take(&mut *guard)
        };
        for (_, session) in sessions {
            session.backend.shutdown().await;
        }
    }

    pub async fn is_open(&self, session_id: &str) -> bool {
        self.sessions.read().await.contains_key(session_id)
    }
}

/// Serialization helper so concurrent open calls for the same profile do not
/// race: only one connect attempt per profile id runs at a time.
#[derive(Default)]
pub struct ConnectLocks {
    locks: Mutex<HashMap<String, std::sync::Arc<Mutex<()>>>>,
}

impl ConnectLocks {
    pub async fn acquire(&self, key: &str) -> std::sync::Arc<Mutex<()>> {
        let mut guard = self.locks.lock().await;
        guard
            .entry(key.to_string())
            .or_insert_with(|| std::sync::Arc::new(Mutex::new(())))
            .clone()
    }
}
