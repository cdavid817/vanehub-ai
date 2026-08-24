//! CLI parameter use cases: list, draft preview, save, reset, and launch resolution. Every
//! outward dependency is a port; nothing here knows about SQLite, Tauri, or a provider process.

pub(crate) mod error;
#[cfg(test)]
mod fakes;
pub(crate) mod models;
pub(crate) mod ports;
pub(crate) mod resolution;
pub(crate) mod service;
pub(crate) mod support;
#[cfg(test)]
mod tests;
