//! The managed-install subdomain's published surface.
//!
//! A symbol appears here when a consumer needs it. The retrieval internals — the redirect loop,
//! the chunk size, the digest accumulation — stay private, because a consumer that could reach
//! them could also work around them.

pub(crate) use super::domain::error::ManagedInstallError;
pub(crate) use super::domain::policy::{ArtifactIntegrity, ManagedPlatform, RetrievalPolicy};
pub(crate) use super::infrastructure::retriever::{
    ArtifactRequest, HttpsArtifactRetriever, ManagedArtifactRetriever, RetrievedArtifact,
};
