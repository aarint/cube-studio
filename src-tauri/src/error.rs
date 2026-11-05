//! Shared error type for the whole backend.
//!
//! Every driver maps its native error into [`AppError`] so the frontend only
//! ever has to understand one shape. The `kind` tag lets the UI decide whether
//! a failure is retryable, a user mistake, or a bug.

use serde::Serialize;

/// Coarse classification of a failure, used by the frontend to pick an
/// appropriate affordance (retry button, inline validation, fatal dialog...).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    /// Bad / missing configuration supplied by the user.
    InvalidInput,
    /// Host unreachable, DNS failure, connection refused, TLS failure.
    Network,
    /// Connected, but the server rejected us (auth, ACL, permission).
    Unauthorized,
    /// Server understood the request but returned an error reply.
    ServerError,
    /// Feature is not supported by this backend kind or by this server.
    Unsupported,
    /// The session was closed or never existed.
    NoSession,
    /// Request exceeded its deadline.
    Timeout,
    /// Temporarily unavailable — rate limited or backing off. Worth retrying.
    Unavailable,
    /// Anything we did not anticipate.
    Internal,
}

/// The single error type returned across the Tauri IPC boundary.
#[derive(Debug, Clone, Serialize)]
pub struct AppError {
    pub kind: ErrorKind,
    pub message: String,
    /// Optional machine readable details (server code, offending field, ...).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn invalid(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    pub fn network(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Network, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unauthorized, message)
    }

    pub fn server(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::ServerError, message)
    }

    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, message)
    }

    pub fn no_session(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NoSession, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }

    /// Classify an arbitrary driver error by inspecting its message.
    ///
    /// Drivers in this project expose wildly different error enums, and
    /// matching each of them exhaustively would couple this module to every
    /// driver version. Instead drivers call `map_driver` with a hint and we
    /// fall back to textual classification, which is good enough for choosing
    /// an icon and a retry affordance.
    pub fn driver(message: impl Into<String>) -> Self {
        let message = message.into();
        let lower = message.to_lowercase();
        let kind = if lower.contains("authentication")
            || lower.contains("unauthorized")
            || lower.contains("noauth")
            || lower.contains("invalid password")
            || lower.contains("access denied")
            || lower.contains("access refused")
            || lower.contains("forbidden")
        {
            ErrorKind::Unauthorized
        } else if lower.contains("timed out")
            || lower.contains("timeout")
            || lower.contains("deadline")
        {
            ErrorKind::Timeout
        } else if lower.contains("connection refused")
            || lower.contains("failed to lookup")
            || lower.contains("dns")
            || lower.contains("unreachable")
            || lower.contains("tls")
            || lower.contains("ssl")
            || lower.contains("certificate")
            || lower.contains("broken pipe")
            || lower.contains("connection reset")
        {
            ErrorKind::Network
        } else if lower.contains("not supported")
            || lower.contains("unsupported")
            || lower.contains("unknown command")
        {
            ErrorKind::Unsupported
        } else {
            ErrorKind::ServerError
        };
        Self::new(kind, message)
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        let kind = match err.kind() {
            std::io::ErrorKind::ConnectionRefused
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::HostUnreachable
            | std::io::ErrorKind::NetworkUnreachable => ErrorKind::Network,
            std::io::ErrorKind::TimedOut => ErrorKind::Timeout,
            std::io::ErrorKind::PermissionDenied => ErrorKind::Unauthorized,
            std::io::ErrorKind::InvalidInput | std::io::ErrorKind::InvalidData => {
                ErrorKind::InvalidInput
            }
            _ => ErrorKind::Internal,
        };
        Self::new(kind, err.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        Self::internal(format!("serialization error: {err}"))
    }
}

impl From<tokio::time::error::Elapsed> for AppError {
    fn from(_: tokio::time::error::Elapsed) -> Self {
        Self::new(ErrorKind::Timeout, "operation timed out")
    }
}

impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        serde_json::to_string(&err).unwrap_or_else(|_| err.message.clone())
    }
}

pub type AppResult<T> = Result<T, AppError>;
