//! Skill-contributed tool packaging, discovery, integrity, trust, and lifecycle state.
//!
//! Kept as a `tooling` subdomain rather than a peer context: it has its own domain language and
//! transaction boundary, but every consumer reaches it through the tool-execution surface
//! `tooling` already owns, and `openspec/project.md` treats the context table as a closed map.
//!
//! The execution halves of `add-sandboxed-skill-tool-runtime` (declarative dispatch, the
//! WebAssembly sandbox, catalog contribution, and the governance commands) land in later
//! sections. Sections 1 and 2 publish the whole contract surface those sections implement
//! against, so several ports, models, and error variants have no caller in either a production or
//! a test build yet — hence a blanket allow rather than the usual `cfg_attr(not(test), ...)`.
//! Remove it once section 9 closes and every declared port has a consumer.
#![allow(dead_code)]

pub(crate) mod api;
pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;

/// Whether this build carries the optional WebAssembly module runtime.
///
/// Read as a constant rather than as `#[cfg]` at each use site so the disabled configuration is a
/// value the ordinary code path handles — a `wasm` tool becomes visibly unavailable — instead of
/// a compilation variant where the handling code does not exist to be tested.
pub(crate) const MODULE_RUNTIME_ENABLED: bool = cfg!(feature = "skill-tool-module-runtime");
