//! Published contract for consumers of Skill tool state.
//!
//! Only the bounded, byte-free projections are exposed. Manifests, module content, integrity
//! verification, and governance operations stay behind the application layer, so a consumer can
//! render a Skill's tool inventory without gaining a way to read or run any of it.

#[allow(unused_imports)]
pub(crate) use super::application::{
    project_inventory_summary, DiscoveredSkillTool, SkillToolDiscoveryOutcome,
    SkillToolInventoryEntry, SkillToolInventorySummary, SkillToolRevisionState,
    MAX_INVENTORY_ENTRIES,
};
