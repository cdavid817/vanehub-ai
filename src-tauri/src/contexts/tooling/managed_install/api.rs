//! The managed-install subdomain's published surface.
//!
//! A symbol appears here when a consumer needs it. The retrieval internals — the redirect loop,
//! the chunk size, the digest accumulation — stay private, because a consumer that could reach
//! them could also work around them.

pub(crate) use super::domain::error::ManagedInstallError;
// The platform-selection and archive halves have no production consumer yet -- they are
// specified and tested here so the change that needs them adds a caller rather than a capability.
#[allow(unused_imports)]
pub(crate) use super::domain::policy::{
    artifact_for_current_platform, ArtifactIntegrity, ManagedPlatform, PlatformArtifact,
    RetrievalPolicy,
};
#[allow(unused_imports)]
pub(crate) use super::infrastructure::extraction::{
    extract_tar_gz, extract_zip, ArchiveEntryKind, ExtractedArchive, ExtractionGuard,
    ExtractionLimits,
};
#[allow(unused_imports)]
pub(crate) use super::infrastructure::retriever::{
    ArtifactRequest, HttpsArtifactRetriever, ManagedArtifactRetriever, RetrievedArtifact,
};
