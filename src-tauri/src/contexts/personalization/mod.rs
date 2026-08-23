//! Governed personalization: policy scopes and precedence, custom-instruction records, and the
//! identity, scope, lifecycle, review, and persistence of long-term memory.
//!
//! This context owns *authorization* — which instructions and which memories a concrete generation
//! is allowed to see. It deliberately does not own model-shaped work: extraction prompts, when
//! OnePiece compaction fires, when a CLI turn completes, and relevance selection stay in
//! `agent_runtime`, which asks this context for an already-filtered set.
//!
//! It also does not own any CLI's internal context compaction or native memory/instruction files.
//! VaneHub governs what it injects; a wrapped CLI keeps its own context machinery.

pub(crate) mod application;
pub(crate) mod domain;
pub(crate) mod infrastructure;
