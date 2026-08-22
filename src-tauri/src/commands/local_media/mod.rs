//! Thin local-media command adapters.
//!
//! Grouped by concern rather than one file per command: `profile.rs` is synchronous state, and
//! `operations.rs` is the accept-then-execute family that all share the same handle mapping.

mod dto;
mod mapper;
pub(crate) mod operations;
pub(crate) mod profile;
