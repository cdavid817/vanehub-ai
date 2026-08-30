#![allow(dead_code)]

use super::{
    OverlayApplicationError, OverlayHistoryEntry, OverlayHistoryPage, OverlayHistoryQuery,
    OverlayImportRequest, SkillApplicationError, SkillResourceDocument,
};
use crate::contexts::tooling::skills::domain::{
    BaseSkillResource, OverlayContentKind, OverlayDocument, OverlayScope, OverlayTextScan, SkillId,
    SkillLayer,
};

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct OverlayKey {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) workspace_identity: Option<String>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayManifestSnapshot {
    pub(crate) document: OverlayDocument,
    pub(crate) document_hash: String,
}

pub(crate) trait OverlayManifestRepository: Send + Sync {
    fn load(
        &self,
        key: &OverlayKey,
    ) -> Result<Option<OverlayManifestSnapshot>, SkillApplicationError>;

    fn applicable(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<Vec<OverlayManifestSnapshot>, SkillApplicationError>;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayPayloadWrite {
    pub(crate) content_hash: String,
    pub(crate) content: Vec<u8>,
}

pub(crate) trait OverlayPayloadRepository: Send + Sync {
    fn read_verified(
        &self,
        key: &OverlayKey,
        content_hash: &str,
    ) -> Result<Vec<u8>, SkillApplicationError>;

    fn referenced_content_hashes(
        &self,
        key: &OverlayKey,
    ) -> Result<Vec<String>, SkillApplicationError>;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayEffectivePackageSnapshot {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) base_identity: String,
    pub(crate) base_layer: SkillLayer,
    pub(crate) instructions: String,
    pub(crate) resources: Vec<BaseSkillResource>,
    pub(crate) instruction_hash: String,
    pub(crate) package_hash: String,
}

pub(crate) trait OverlayEffectiveSnapshotPort: Send + Sync {
    fn read_effective_package(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayEffectivePackageSnapshot, SkillApplicationError>;

    fn read_effective_resource(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        logical_path: &str,
    ) -> Result<SkillResourceDocument, SkillApplicationError> {
        let _ = (workspace_identity, logical_path);
        Err(SkillApplicationError::NotFound(
            canonical_skill_id.as_str().to_string(),
        ))
    }

    fn read_effective_resource_bytes(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
        logical_path: &str,
    ) -> Result<Vec<u8>, SkillApplicationError> {
        self.read_effective_resource(canonical_skill_id, workspace_identity, logical_path)
            .map(|resource| resource.content.into_bytes())
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayValidatedFile {
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) content_kind: OverlayContentKind,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
}

pub(crate) trait OverlayContentScannerPort: Send + Sync {
    fn scan_text(&self, content: &str) -> OverlayTextScan;

    fn validate_file(
        &self,
        logical_path: &str,
        media_type: &str,
        content: &[u8],
    ) -> Result<OverlayValidatedFile, OverlayApplicationError>;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayPreparedImport {
    pub(crate) document: OverlayDocument,
    pub(crate) payloads: Vec<OverlayPayloadWrite>,
    pub(crate) scanner_version: String,
}

pub(crate) trait OverlayImportParserPort: Send + Sync {
    fn parse(
        &self,
        request: &OverlayImportRequest,
    ) -> Result<OverlayPreparedImport, SkillApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayPinSnapshot {
    pub(crate) pinned: bool,
    pub(crate) revision_witness: String,
}

pub(crate) trait OverlayPinStatePort: Send + Sync {
    fn pin_snapshot(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayPinSnapshot, SkillApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayUsageSnapshot {
    pub(crate) patch_count: u64,
    pub(crate) overlay_mutation_count: u64,
    pub(crate) last_patched_at: Option<String>,
    pub(crate) last_overlay_mutation_at: Option<String>,
    pub(crate) revision_witness: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayUsageDelta {
    pub(crate) patch_count_delta: u64,
    pub(crate) overlay_mutation_count_delta: u64,
    pub(crate) timestamp: String,
    pub(crate) expected_revision_witness: String,
}

pub(crate) trait OverlayUsageStatePort: Send + Sync {
    fn usage_snapshot(
        &self,
        key: &OverlayKey,
    ) -> Result<OverlayUsageSnapshot, SkillApplicationError>;
}

pub(crate) trait OverlayHistoryRepository: Send + Sync {
    fn read_verified_page(
        &self,
        key: &OverlayKey,
        query: &OverlayHistoryQuery,
    ) -> Result<OverlayHistoryPage, SkillApplicationError>;

    fn verified_tail_hash(&self, key: &OverlayKey)
        -> Result<Option<String>, SkillApplicationError>;

    fn find_curator_application(
        &self,
        key: &OverlayKey,
        application_id: &str,
    ) -> Result<Option<OverlayHistoryEntry>, SkillApplicationError>;
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayTransactionPlan {
    pub(crate) key: OverlayKey,
    pub(crate) expected_revision: Option<u64>,
    pub(crate) expected_document_hash: Option<String>,
    pub(crate) next_manifest: OverlayManifestSnapshot,
    pub(crate) payload_additions: Vec<OverlayPayloadWrite>,
    pub(crate) history_event: OverlayHistoryEntry,
    pub(crate) usage_delta: OverlayUsageDelta,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayTransactionOutcome {
    pub(crate) committed_revision: u64,
    pub(crate) document_hash: String,
    pub(crate) history_event_hash: String,
    pub(crate) usage_revision_witness: String,
}

pub(crate) trait OverlayTransactionExecutor: Send + Sync {
    fn manifest_snapshot(
        &self,
        document: OverlayDocument,
    ) -> Result<OverlayManifestSnapshot, SkillApplicationError>;

    fn execute(
        &self,
        plan: OverlayTransactionPlan,
    ) -> Result<OverlayTransactionOutcome, SkillApplicationError>;

    fn recover(&self, key: &OverlayKey) -> Result<(), SkillApplicationError>;
}

pub(crate) trait OverlayRuntimeCacheInvalidationPort: Send + Sync {
    fn invalidate(&self, key: &OverlayKey);
}
