//! Cube Studio backend library.
//!
//! Layering, from the outside in:
//!
//! * [`commands`] — the Tauri IPC surface. Thin: validate, dispatch, map errors.
//! * [`session`] — owns live connections; one `Session` per connected profile.
//! * [`backend`] — the `Backend` trait and one implementation per middleware.
//! * [`model`] — the transport-neutral data model shared with the frontend.
//! * [`store`] / [`config`] — persistence and connection profiles.

pub mod backend;
pub mod cloud;
pub mod commands;
pub mod config;
pub mod error;
pub mod locales;
pub mod model;
pub mod session;
pub mod store;

pub use error::{AppError, ErrorKind};
