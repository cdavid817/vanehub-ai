//! CLI parameter domain: the selection envelope, the capability registry and its invariants,
//! deterministic rendering, compatibility, dependency evaluation, and structured diagnostics.
//!
//! Nothing here touches SQLite, Tauri, the filesystem, or a child process.

pub(crate) mod catalog;
pub(crate) mod catalog_validation;
pub(crate) mod compatibility;
pub(crate) mod definition;
pub(crate) mod dependency;
pub(crate) mod diagnostic;
pub(crate) mod error;
pub(crate) mod profile;
pub(crate) mod rendering;
pub(crate) mod selection;
#[cfg(test)]
pub(crate) mod testing;
pub(crate) mod validation;
