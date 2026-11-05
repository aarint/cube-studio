//! Tauri command surface.
//!
//! Every command returns `Result<T, String>` where the error string is a
//! serialized [`AppError`], so the frontend can branch on `kind` without
//! parsing prose. Commands are intentionally thin: validation and dispatch
//! live in [`crate::backend`] and [`crate::session`].

use std::sync::Arc;

use tauri::{AppHandle, Emitter, State};
use tokio::sync::Mutex;

use crate::backend::{ConnectionInfo, TailHandle};
use crate::config::{BackendKind, ConnectionConfig};
use crate::error::{AppError, AppResult};
use crate::model::{
    Capabilities, ConsoleResult, Container, Entry, EntryPage, MutationResult, StatItem,
    StreamMessage, WriteOptions,
};
use crate::session::{SessionDescriptor, SessionRegistry};
use crate::store::ProfileStore;

/// Event name used to push streamed messages to the frontend.
pub const STREAM_EVENT: &str = "cube://stream-message";
/// Event emitted when a subscription ends.
pub const STREAM_CLOSED_EVENT: &str = "cube://stream-closed";

/// Application-wide state handed to every command.
pub struct AppState {
    pub sessions: Arc<SessionRegistry>,
    pub store: Arc<ProfileStore>,
    /// Active subscriptions, keyed by subscription id.
    pub tails: Mutex<std::collections::HashMap<String, TailSubscription>>,
}

/// Bookkeeping for a live subscription so it can be cancelled by id.
pub struct TailSubscription {
    pub session_id: String,
    pub container: String,
    /// Dropping this aborts the forwarding task.
    pub abort: tokio::task::AbortHandle,
}

impl AppState {
    pub fn new(store: ProfileStore) -> Self {
        Self {
            sessions: Arc::new(SessionRegistry::new()),
            store: Arc::new(store),
            tails: Mutex::new(std::collections::HashMap::new()),
        }
    }
}

/// Turn an `AppResult` into the string error Tauri expects.
fn to_ipc<T>(result: AppResult<T>) -> Result<T, String> {
    result.map_err(|err| serde_json::to_string(&err).unwrap_or_else(|_| err.message.clone()))
}

// ---- metadata ---------------------------------------------------------

/// Everything the UI needs to build its connection form and sidebar.
#[derive(serde::Serialize)]
pub struct AppMetadata {
    pub backends: Vec<BackendMetadata>,
    /// Cloud providers whose managed instances can be discovered.
    pub cloud_providers: Vec<CloudProviderMetadata>,
    /// Managed services, flattened, with the credential fields each needs.
    pub cloud_services: Vec<CloudServiceMetadata>,
    pub version: String,
    pub platform: String,
    /// Languages the UI ships with; the frontend also carries the strings.
    pub locales: Vec<&'static str>,
}

#[derive(serde::Serialize)]
pub struct CloudProviderMetadata {
    pub id: crate::cloud::CloudProvider,
    pub label: &'static str,
    pub credentials_url: &'static str,
}

#[derive(serde::Serialize)]
pub struct CloudServiceMetadata {
    pub id: &'static str,
    pub provider: crate::cloud::CloudProvider,
    pub backend: BackendKind,
    pub auth: crate::cloud::CloudAuthKind,
    pub credential_fields: Vec<&'static str>,
}

#[derive(serde::Serialize)]
pub struct BackendMetadata {
    pub kind: BackendKind,
    pub label: &'static str,
    pub default_port: u16,
    pub capabilities: Capabilities,
    /// Fields the form should show for this backend.
    pub fields: Vec<&'static str>,
}

#[tauri::command]
pub fn app_metadata() -> AppMetadata {
    let backends = BackendKind::all()
        .iter()
        .map(|kind| BackendMetadata {
            kind: *kind,
            label: kind.display_name(),
            default_port: kind.default_port(),
            capabilities: Capabilities::for_kind(*kind),
            fields: match kind {
                BackendKind::Redis => vec!["username", "password", "redis_mode", "redis_db"],
                BackendKind::Memcached => vec![],
                BackendKind::Kafka => {
                    vec!["username", "password", "kafka_sasl_mechanism", "kafka_group_id"]
                }
                BackendKind::RabbitMq => vec!["username", "password", "vhost"],
                BackendKind::Nats => vec!["username", "password", "token"],
                BackendKind::Mqtt => vec!["username", "password", "client_id", "keep_alive"],
            },
        })
        .collect();

    let cloud_services: Vec<CloudServiceMetadata> = crate::cloud::catalog()
        .into_iter()
        .map(|service| CloudServiceMetadata {
            id: service.id,
            provider: service.provider,
            backend: service.backend,
            auth: service.auth,
            credential_fields: service.credential_fields,
        })
        .collect();

    let cloud_providers = crate::cloud::CloudProvider::all()
        .iter()
        .map(|provider| CloudProviderMetadata {
            id: *provider,
            label: crate::cloud::provider_label(*provider),
            credentials_url: provider.credentials_url(),
        })
        .collect();

    AppMetadata {
        backends,
        cloud_providers,
        cloud_services,
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        locales: crate::locales::SUPPORTED.to_vec(),
    }
}

// ---- cloud discovery --------------------------------------------------

/// List managed instances for one cloud service.
///
/// Read-only by construction: every provider call behind this is a `List` or
/// `Get`. Credentials are used and dropped, never persisted.
#[tauri::command]
pub async fn discover_cloud_resources(
    service: String,
    credentials: crate::cloud::CloudCredentials,
    timeout_secs: Option<u64>,
) -> Result<Vec<crate::cloud::DiscoveredResource>, String> {
    let timeout = std::time::Duration::from_secs(timeout_secs.unwrap_or(30).clamp(5, 120));
    to_ipc(crate::cloud::discover(&service, &credentials, timeout).await)
}

/// Build a ready-to-connect profile from a discovered instance.
///
/// `secret` is the data-plane password, which is separate from the control-plane
/// credentials the discovery used.
#[tauri::command]
pub fn cloud_resource_profile(
    resource: crate::cloud::DiscoveredResource,
    secret: Option<String>,
) -> Result<ConnectionConfig, String> {
    to_ipc(resource.to_connection_config(secret.as_deref()))
}

/// Connect to a discovered instance in one step.
#[tauri::command]
pub async fn connect_cloud_resource(
    state: State<'_, AppState>,
    resource: crate::cloud::DiscoveredResource,
    secret: Option<String>,
    save: Option<bool>,
) -> Result<SessionDescriptor, String> {
    let config = to_ipc(resource.to_connection_config(secret.as_deref()))?;

    let mut config = config;
    if save.unwrap_or(false) {
        let store = state.store.clone();
        let to_save = config.clone();
        match tokio::task::spawn_blocking(move || store.upsert(to_save)).await {
            Ok(Ok(saved)) => {
                config.password = saved.password;
                config.token = saved.token;
            }
            Ok(Err(err)) => return Err(err.into()),
            Err(err) => {
                return Err(AppError::internal(format!("profile task failed: {err}")).into())
            }
        }
    }

    to_ipc(state.sessions.open(config, None).await)
}

// ---- profiles ---------------------------------------------------------

#[tauri::command]
pub async fn list_profiles(state: State<'_, AppState>) -> Result<Vec<ConnectionConfig>, String> {
    let store = state.store.clone();
    to_ipc(tokio::task::spawn_blocking(move || store.list()).await.unwrap_or_else(|e| {
        Err(AppError::internal(format!("profile task failed: {e}")))
    }))
}

#[tauri::command]
pub async fn save_profile(
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<ConnectionConfig, String> {
    let store = state.store.clone();
    to_ipc(
        tokio::task::spawn_blocking(move || store.upsert(config))
            .await
            .unwrap_or_else(|e| Err(AppError::internal(format!("profile task failed: {e}")))),
    )
}

#[tauri::command]
pub async fn delete_profile(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let store = state.store.clone();
    to_ipc(
        tokio::task::spawn_blocking(move || store.delete(&id))
            .await
            .unwrap_or_else(|e| Err(AppError::internal(format!("profile task failed: {e}")))),
    )
}

// ---- connections ------------------------------------------------------

#[tauri::command]
pub async fn connect(
    state: State<'_, AppState>,
    config: ConnectionConfig,
) -> Result<SessionDescriptor, String> {
    // Remember the password on the profile when the user asked to save it, then
    // open the session with a config that definitely has the secret.
    let mut config = config;
    let profile_id = if config.meta.contains_key("save") {
        let store = state.store.clone();
        let to_save = config.clone();
        match tokio::task::spawn_blocking(move || store.upsert(to_save)).await {
            Ok(Ok(saved)) => {
                config.password = saved.password;
                config.token = saved.token;
                Some(saved.id)
            }
            Ok(Err(err)) => return Err(err.into()),
            Err(err) => {
                return Err(AppError::internal(format!("profile task failed: {err}")).into())
            }
        }
    } else if !config.id.is_empty() && state.store.get(&config.id).is_ok() {
        Some(config.id.clone())
    } else {
        None
    };

    to_ipc(state.sessions.open(config, profile_id).await)
}

#[tauri::command]
pub async fn disconnect(state: State<'_, AppState>, session_id: String) -> Result<(), String> {
    // Cancel every tail belonging to this session first.
    let mut tails = state.tails.lock().await;
    let stale: Vec<String> = tails
        .iter()
        .filter(|(_, tail)| tail.session_id == session_id)
        .map(|(id, _)| id.clone())
        .collect();
    for id in stale {
        if let Some(tail) = tails.remove(&id) {
            tail.abort.abort();
        }
    }
    drop(tails);

    to_ipc(state.sessions.close(&session_id).await)
}

#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionDescriptor>, String> {
    Ok(state.sessions.list().await)
}

#[tauri::command]
pub async fn session_info(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<ConnectionInfo, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.info().await
    }
    .await)
}

#[tauri::command]
pub async fn test_connection(config: ConnectionConfig) -> Result<ConnectionInfo, String> {
    // Connect, describe, and immediately drop: a throwaway session.
    to_ipc(async {
        let backend = crate::backend::connect(&config.normalize()?).await?;
        let info = backend.info().await;
        backend.shutdown().await;
        info
    }
    .await)
}

// ---- browsing ---------------------------------------------------------

#[tauri::command]
pub async fn list_containers(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<Container>, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.containers().await
    }
    .await)
}

#[tauri::command]
pub async fn list_entries(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    cursor: Option<String>,
    limit: Option<usize>,
    search: Option<String>,
) -> Result<EntryPage, String> {
    let limit = limit.unwrap_or(200);
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend
            .entries(&container, cursor.as_deref(), limit, search.as_deref())
            .await
    }
    .await)
}

#[tauri::command]
pub async fn read_entry(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    entry: String,
) -> Result<Entry, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.read_entry(&container, &entry).await
    }
    .await)
}

#[tauri::command]
pub async fn write_entry(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    entry: String,
    payload_base64: Option<String>,
    text: Option<String>,
    options: Option<WriteOptions>,
) -> Result<MutationResult, String> {
    to_ipc(async {
        let bytes = decode_payload(payload_base64, text)?;
        let options = options.unwrap_or_default();
        let backend = state.sessions.backend(&session_id).await?;
        backend
            .write_entry(&container, &entry, &bytes, &options)
            .await
    }
    .await)
}

#[tauri::command]
pub async fn delete_entries(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    entries: Vec<String>,
) -> Result<MutationResult, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.delete_entry(&container, &entries).await
    }
    .await)
}

#[tauri::command]
pub async fn set_entry_ttl(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    entry: String,
    ttl_secs: i64,
) -> Result<(), String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.set_ttl(&container, &entry, ttl_secs).await
    }
    .await)
}

#[tauri::command]
pub async fn publish_message(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
    payload_base64: Option<String>,
    text: Option<String>,
    options: Option<WriteOptions>,
) -> Result<MutationResult, String> {
    to_ipc(async {
        let bytes = decode_payload(payload_base64, text)?;
        let options = options.unwrap_or_default();
        let backend = state.sessions.backend(&session_id).await?;
        backend.publish(&container, &bytes, &options).await
    }
    .await)
}

#[tauri::command]
pub async fn flush_container(
    state: State<'_, AppState>,
    session_id: String,
    container: String,
) -> Result<MutationResult, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.flush(&container).await
    }
    .await)
}

#[tauri::command]
pub async fn server_stats(
    state: State<'_, AppState>,
    session_id: String,
) -> Result<Vec<StatItem>, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.stats().await
    }
    .await)
}

#[tauri::command]
pub async fn run_console(
    state: State<'_, AppState>,
    session_id: String,
    command: String,
) -> Result<ConsoleResult, String> {
    to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.console(&command).await
    }
    .await)
}

// ---- live streaming ---------------------------------------------------

/// Start a subscription and forward its messages as Tauri events.
#[tauri::command]
pub async fn start_tail(
    app: AppHandle,
    state: State<'_, AppState>,
    session_id: String,
    container: String,
) -> Result<String, String> {
    let handle: TailHandle = to_ipc(async {
        let backend = state.sessions.backend(&session_id).await?;
        backend.tail(&container).await
    }
    .await)?;

    let subscription_id = handle.subscription_id.clone();
    let TailHandle {
        subscription_id: id_for_task,
        mut receiver,
    } = handle;

    let app_for_task = app.clone();
    let task = tokio::spawn(async move {
        let mut delivered = 0u64;
        while let Some(message) = receiver.recv().await {
            delivered += 1;
            let payload: StreamMessage = message;
            if app_for_task.emit(STREAM_EVENT, &payload).is_err() {
                break;
            }
        }
        let _ = app_for_task.emit(
            STREAM_CLOSED_EVENT,
            serde_json::json!({
                "subscription_id": id_for_task,
                "messages": delivered,
            }),
        );
    });

    state.tails.lock().await.insert(
        subscription_id.clone(),
        TailSubscription {
            session_id,
            container,
            abort: task.abort_handle(),
        },
    );

    Ok(subscription_id)
}

#[tauri::command]
pub async fn stop_tail(state: State<'_, AppState>, subscription_id: String) -> Result<(), String> {
    let mut tails = state.tails.lock().await;
    let removed = tails.remove(&subscription_id);
    drop(tails);
    match removed {
        Some(tail) => {
            tail.abort.abort();
            Ok(())
        }
        // Stopping an already-stopped subscription is not an error.
        None => Ok(()),
    }
}

/// Which subscriptions are currently active, so the UI can restore state after
/// a reload.
#[tauri::command]
pub async fn list_tails(state: State<'_, AppState>) -> Result<Vec<TailDescriptor>, String> {
    let tails = state.tails.lock().await;
    Ok(tails
        .iter()
        .map(|(id, tail)| TailDescriptor {
            subscription_id: id.clone(),
            session_id: tail.session_id.clone(),
            container: tail.container.clone(),
        })
        .collect())
}

#[derive(serde::Serialize)]
pub struct TailDescriptor {
    pub subscription_id: String,
    pub session_id: String,
    pub container: String,
}

// ---- helpers ----------------------------------------------------------

/// Payload arrives either as base64 (binary safety) or as plain text.
fn decode_payload(base64_value: Option<String>, text: Option<String>) -> AppResult<Vec<u8>> {
    use base64::Engine as _;
    match (base64_value, text) {
        (Some(encoded), _) if !encoded.is_empty() => base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| AppError::invalid(format!("invalid base64 payload: {e}"))),
        (_, Some(text)) => Ok(text.into_bytes()),
        _ => Ok(Vec::new()),
    }
}

/// Initialise tracing once, honouring `RUST_LOG`.
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("cube_studio_lib=info,warn"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true)
        .try_init();
}
