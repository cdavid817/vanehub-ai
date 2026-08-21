//! Published in-process facade for the `artifacts` context.
//!
//! `openspec/project.md` recorded that this context had no `api.rs` and owed one "before a second
//! consumer depends on it". It has had several for a while — `agent_runtime`, `cli_delegation`,
//! `tooling::extensions`, and the command layer all reach in — so the debt is settled here rather
//! than exempted.
//!
//! This is a visibility boundary, not a redesign: the same types, re-exported through one
//! deliberate surface, so a consumer cannot quietly acquire a dependency on an internal module
//! that was never meant to be part of the contract.

/// Consumed only by cross-context test doubles today: the service's own ports and error type,
/// which a consumer needs to stand up an `ArtifactService` without real storage. Published here so
/// a test reaches for the same surface production does, rather than learning a second path into
/// this context.
#[cfg(test)]
pub(crate) use super::application::{
    ArtifactBlobMetadata, ArtifactBlobPort, ArtifactBlobStoreError, ArtifactCatalogPort,
    ArtifactServiceError,
};
pub(crate) use super::application::{
    ArtifactBlobStorePolicy, ArtifactCreateRequest, ArtifactCreator, ArtifactDescriptor,
    ArtifactEvidenceKind, ArtifactPublicationReference, ArtifactService, ArtifactVisibility,
};
