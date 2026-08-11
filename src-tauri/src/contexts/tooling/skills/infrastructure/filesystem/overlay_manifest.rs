#![cfg_attr(not(test), allow(dead_code))]

use crate::contexts::tooling::skills::application::{
    OverlayApplicationError, OverlayIntegrityCode, OverlayKey, OverlayManifestRepository,
    OverlayManifestSnapshot, SkillApplicationError,
};
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayConflict, OverlayConflictState, OverlayDocument, OverlayFile,
    OverlayLearnBlock, OverlayMutationState, OverlayOrigin, OverlayPatch, OverlayScope,
    OverlayTrust, OverlayTrustState, SkillId, OVERLAY_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;
use std::fs;
use std::path::PathBuf;

use super::overlay_layout::OverlayStorageLayout;

const MAXIMUM_MANIFEST_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct FilesystemOverlayManifestRepository {
    home_root: PathBuf,
}

impl FilesystemOverlayManifestRepository {
    pub(crate) fn new() -> Self {
        Self {
            home_root: super::default_home_root(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_home_root(home_root: PathBuf) -> Self {
        Self { home_root }
    }

    fn read_snapshot(
        &self,
        key: &OverlayKey,
    ) -> Result<Option<OverlayManifestSnapshot>, SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(&self.home_root, key)
            .map_err(|error| SkillApplicationError::Validation(error.to_string()))?;
        let metadata = match fs::symlink_metadata(&layout.manifest_path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(SkillApplicationError::Filesystem(error.to_string())),
        };
        if metadata.file_type().is_symlink()
            || !metadata.is_file()
            || metadata.len() > MAXIMUM_MANIFEST_BYTES
        {
            return Err(integrity(OverlayIntegrityCode::DocumentHashMismatch));
        }
        let bytes = fs::read(&layout.manifest_path)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        let document = parse_overlay_manifest(&bytes).map_err(manifest_application_error)?;
        if document.canonical_skill_id != key.canonical_skill_id
            || document.scope() != key.scope
            || document.workspace_identity() != key.workspace_identity.as_deref()
        {
            return Err(integrity(OverlayIntegrityCode::DocumentHashMismatch));
        }
        Ok(Some(OverlayManifestSnapshot {
            document,
            document_hash: sha256(&bytes),
        }))
    }
}

impl OverlayManifestRepository for FilesystemOverlayManifestRepository {
    fn load(
        &self,
        key: &OverlayKey,
    ) -> Result<Option<OverlayManifestSnapshot>, SkillApplicationError> {
        self.read_snapshot(key)
    }

    fn applicable(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<Vec<OverlayManifestSnapshot>, SkillApplicationError> {
        let mut snapshots = Vec::new();
        for (scope, workspace_identity) in [
            (OverlayScope::System, None),
            (OverlayScope::User, None),
            (OverlayScope::Project, workspace_identity),
        ] {
            if scope == OverlayScope::Project && workspace_identity.is_none() {
                continue;
            }
            let key = OverlayKey {
                canonical_skill_id: canonical_skill_id.clone(),
                scope,
                workspace_identity: workspace_identity.map(str::to_string),
            };
            if let Some(snapshot) = self.read_snapshot(&key)? {
                snapshots.push(snapshot);
            }
        }
        Ok(snapshots)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OverlayManifestError {
    InvalidJson(String),
    UnsupportedSchemaVersion { found: u32, supported: u32 },
    UnsupportedFutureVersion { found: u32, supported: u32 },
    InvalidDomain(String),
}

impl fmt::Display for OverlayManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(message) => {
                write!(formatter, "Invalid Overlay manifest JSON: {message}")
            }
            Self::UnsupportedSchemaVersion { found, supported } => write!(
                formatter,
                "Unsupported Overlay schema version {found}; supported version is {supported}"
            ),
            Self::UnsupportedFutureVersion { found, supported } => write!(
                formatter,
                "Overlay schema version {found} is newer than supported version {supported}"
            ),
            Self::InvalidDomain(message) => {
                write!(formatter, "Invalid Overlay manifest domain data: {message}")
            }
        }
    }
}

pub(super) fn parse_overlay_manifest(
    bytes: &[u8],
) -> Result<OverlayDocument, OverlayManifestError> {
    let probe: VersionProbe = serde_json::from_slice(bytes).map_err(json_error)?;
    if probe.schema_version > OVERLAY_SCHEMA_VERSION {
        return Err(OverlayManifestError::UnsupportedFutureVersion {
            found: probe.schema_version,
            supported: OVERLAY_SCHEMA_VERSION,
        });
    }
    if probe.schema_version != OVERLAY_SCHEMA_VERSION {
        return Err(OverlayManifestError::UnsupportedSchemaVersion {
            found: probe.schema_version,
            supported: OVERLAY_SCHEMA_VERSION,
        });
    }
    let wire: OverlayDocumentWire = serde_json::from_slice(bytes).map_err(json_error)?;
    wire.try_into()
}

pub(super) fn serialize_overlay_manifest(
    document: &OverlayDocument,
) -> Result<Vec<u8>, OverlayManifestError> {
    if document.schema_version > OVERLAY_SCHEMA_VERSION {
        return Err(OverlayManifestError::UnsupportedFutureVersion {
            found: document.schema_version,
            supported: OVERLAY_SCHEMA_VERSION,
        });
    }
    if document.schema_version != OVERLAY_SCHEMA_VERSION {
        return Err(OverlayManifestError::UnsupportedSchemaVersion {
            found: document.schema_version,
            supported: OVERLAY_SCHEMA_VERSION,
        });
    }
    serde_json::to_vec_pretty(&OverlayDocumentWire::from(document)).map_err(json_error)
}

#[derive(Deserialize)]
struct VersionProbe {
    schema_version: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayDocumentWire {
    schema_version: u32,
    canonical_skill_id: String,
    scope: String,
    workspace_identity: Option<String>,
    revision: u64,
    base_identity: String,
    base_instruction_hash: String,
    base_package_hash: String,
    trust: OverlayTrustWire,
    patches: Vec<OverlayPatchWire>,
    learn_blocks: Vec<OverlayLearnBlockWire>,
    files: Vec<OverlayFileWire>,
    conflicts: Vec<OverlayConflictWire>,
    created_at: String,
    updated_at: String,
    prior_revision_hash: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayTrustWire {
    state: String,
    origin: String,
    source_summary: Option<String>,
    reviewed_revision: Option<u64>,
    reviewed_content_hash: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayPatchWire {
    id: String,
    old_string: String,
    new_string: String,
    replace_all: bool,
    state: String,
    creation_base_hash: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayLearnBlockWire {
    id: String,
    guidance: String,
    state: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayFileWire {
    id: String,
    logical_path: String,
    media_type: String,
    size: u64,
    content_hash: String,
    payload_ref: String,
    state: String,
    created_at: String,
    updated_at: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OverlayConflictWire {
    id: String,
    mutation_id: String,
    reason: String,
    witnessed_base_hash: String,
    state: String,
    resolution_revision: Option<u64>,
}

impl From<&OverlayDocument> for OverlayDocumentWire {
    fn from(document: &OverlayDocument) -> Self {
        Self {
            schema_version: document.schema_version,
            canonical_skill_id: document.canonical_skill_id.as_str().to_string(),
            scope: document.scope().as_str().to_string(),
            workspace_identity: document.workspace_identity().map(str::to_string),
            revision: document.revision(),
            base_identity: document.base_witness.base_identity.clone(),
            base_instruction_hash: document.base_witness.instruction_hash.clone(),
            base_package_hash: document.base_witness.package_hash.clone(),
            trust: OverlayTrustWire::from(document.trust()),
            patches: document
                .patches
                .iter()
                .map(OverlayPatchWire::from)
                .collect(),
            learn_blocks: document
                .learn_blocks
                .iter()
                .map(OverlayLearnBlockWire::from)
                .collect(),
            files: document.files.iter().map(OverlayFileWire::from).collect(),
            conflicts: document
                .conflicts
                .iter()
                .map(OverlayConflictWire::from)
                .collect(),
            created_at: document.created_at.clone(),
            updated_at: document.updated_at.clone(),
            prior_revision_hash: document.prior_revision_hash().map(str::to_string),
        }
    }
}

impl From<&OverlayTrust> for OverlayTrustWire {
    fn from(trust: &OverlayTrust) -> Self {
        Self {
            state: trust_state_name(trust.state()).to_string(),
            origin: trust_origin_name(trust.origin()).to_string(),
            source_summary: trust.source_summary().map(str::to_string),
            reviewed_revision: trust.reviewed_revision(),
            reviewed_content_hash: trust.reviewed_content_hash().map(str::to_string),
        }
    }
}

impl From<&OverlayPatch> for OverlayPatchWire {
    fn from(patch: &OverlayPatch) -> Self {
        Self {
            id: patch.id.clone(),
            old_string: patch.old_string.clone(),
            new_string: patch.new_string.clone(),
            replace_all: patch.replace_all,
            state: mutation_state_name(patch.state()).to_string(),
            creation_base_hash: patch.creation_base_hash.clone(),
            created_at: patch.created_at.clone(),
            updated_at: patch.updated_at.clone(),
        }
    }
}

impl From<&OverlayLearnBlock> for OverlayLearnBlockWire {
    fn from(block: &OverlayLearnBlock) -> Self {
        Self {
            id: block.id.clone(),
            guidance: block.guidance.clone(),
            state: mutation_state_name(block.state()).to_string(),
            created_at: block.created_at.clone(),
            updated_at: block.updated_at.clone(),
        }
    }
}

impl From<&OverlayFile> for OverlayFileWire {
    fn from(file: &OverlayFile) -> Self {
        Self {
            id: file.id.clone(),
            logical_path: file.logical_path.clone(),
            media_type: file.media_type.clone(),
            size: file.size,
            content_hash: file.content_hash.clone(),
            payload_ref: file.payload_ref.clone(),
            state: mutation_state_name(file.state()).to_string(),
            created_at: file.created_at.clone(),
            updated_at: file.updated_at.clone(),
        }
    }
}

impl From<&OverlayConflict> for OverlayConflictWire {
    fn from(conflict: &OverlayConflict) -> Self {
        Self {
            id: conflict.id().to_string(),
            mutation_id: conflict.mutation_id().to_string(),
            reason: conflict.reason.clone(),
            witnessed_base_hash: conflict.witnessed_base_hash.clone(),
            state: conflict_state_name(conflict.state()).to_string(),
            resolution_revision: conflict.resolution_revision(),
        }
    }
}

impl TryFrom<OverlayDocumentWire> for OverlayDocument {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayDocumentWire) -> Result<Self, Self::Error> {
        let skill_id = SkillId::parse(wire.canonical_skill_id).map_err(domain_error)?;
        let scope = parse_scope(&wire.scope)?;
        let base_witness = OverlayBaseWitness::new(
            &wire.base_identity,
            &wire.base_instruction_hash,
            &wire.base_package_hash,
        )
        .map_err(domain_error)?;
        let trust = wire.trust.try_into()?;
        let patches = wire
            .patches
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        let learn_blocks = wire
            .learn_blocks
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        let files = wire
            .files
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        let conflicts = wire
            .conflicts
            .into_iter()
            .map(TryInto::try_into)
            .collect::<Result<Vec<_>, _>>()?;
        OverlayDocument::rehydrate(
            skill_id,
            scope,
            wire.workspace_identity.as_deref(),
            wire.revision,
            base_witness,
            trust,
            patches,
            learn_blocks,
            files,
            conflicts,
            &wire.created_at,
            &wire.updated_at,
            wire.prior_revision_hash.as_deref(),
        )
        .map_err(domain_error)
    }
}

impl TryFrom<OverlayTrustWire> for OverlayTrust {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayTrustWire) -> Result<Self, Self::Error> {
        OverlayTrust::rehydrate(
            parse_trust_state(&wire.state)?,
            parse_origin(&wire.origin)?,
            wire.source_summary,
            wire.reviewed_revision,
            wire.reviewed_content_hash,
        )
        .map_err(domain_error)
    }
}

impl TryFrom<OverlayPatchWire> for OverlayPatch {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayPatchWire) -> Result<Self, Self::Error> {
        OverlayPatch::rehydrate(
            &wire.id,
            &wire.old_string,
            &wire.new_string,
            wire.replace_all,
            parse_mutation_state(&wire.state)?,
            &wire.creation_base_hash,
            &wire.created_at,
            &wire.updated_at,
        )
        .map_err(domain_error)
    }
}

impl TryFrom<OverlayLearnBlockWire> for OverlayLearnBlock {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayLearnBlockWire) -> Result<Self, Self::Error> {
        OverlayLearnBlock::rehydrate(
            &wire.id,
            &wire.guidance,
            parse_mutation_state(&wire.state)?,
            &wire.created_at,
            &wire.updated_at,
        )
        .map_err(domain_error)
    }
}

impl TryFrom<OverlayFileWire> for OverlayFile {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayFileWire) -> Result<Self, Self::Error> {
        OverlayFile::rehydrate(
            &wire.id,
            &wire.logical_path,
            &wire.media_type,
            wire.size,
            &wire.content_hash,
            &wire.payload_ref,
            parse_mutation_state(&wire.state)?,
            &wire.created_at,
            &wire.updated_at,
        )
        .map_err(domain_error)
    }
}

impl TryFrom<OverlayConflictWire> for OverlayConflict {
    type Error = OverlayManifestError;

    fn try_from(wire: OverlayConflictWire) -> Result<Self, Self::Error> {
        OverlayConflict::rehydrate(
            &wire.id,
            &wire.mutation_id,
            &wire.reason,
            &wire.witnessed_base_hash,
            parse_conflict_state(&wire.state)?,
            wire.resolution_revision,
        )
        .map_err(domain_error)
    }
}

fn parse_scope(value: &str) -> Result<OverlayScope, OverlayManifestError> {
    OverlayScope::parse(value).ok_or_else(|| invalid_enum("scope", value))
}

fn parse_mutation_state(value: &str) -> Result<OverlayMutationState, OverlayManifestError> {
    match value {
        "active" => Ok(OverlayMutationState::Active),
        "disabled" => Ok(OverlayMutationState::Disabled),
        "reverted" => Ok(OverlayMutationState::Reverted),
        _ => Err(invalid_enum("mutation state", value)),
    }
}

fn parse_conflict_state(value: &str) -> Result<OverlayConflictState, OverlayManifestError> {
    match value {
        "active" => Ok(OverlayConflictState::Active),
        "resolved" => Ok(OverlayConflictState::Resolved),
        "ignored" => Ok(OverlayConflictState::Ignored),
        _ => Err(invalid_enum("conflict state", value)),
    }
}

fn parse_trust_state(value: &str) -> Result<OverlayTrustState, OverlayManifestError> {
    match value {
        "trusted" => Ok(OverlayTrustState::Trusted),
        "untrusted" => Ok(OverlayTrustState::Untrusted),
        _ => Err(invalid_enum("trust state", value)),
    }
}

fn parse_origin(value: &str) -> Result<OverlayOrigin, OverlayManifestError> {
    match value {
        "local" => Ok(OverlayOrigin::Local),
        "imported" => Ok(OverlayOrigin::Imported),
        _ => Err(invalid_enum("trust origin", value)),
    }
}

fn mutation_state_name(state: OverlayMutationState) -> &'static str {
    match state {
        OverlayMutationState::Active => "active",
        OverlayMutationState::Disabled => "disabled",
        OverlayMutationState::Reverted => "reverted",
    }
}

fn conflict_state_name(state: OverlayConflictState) -> &'static str {
    match state {
        OverlayConflictState::Active => "active",
        OverlayConflictState::Resolved => "resolved",
        OverlayConflictState::Ignored => "ignored",
    }
}

fn trust_state_name(state: OverlayTrustState) -> &'static str {
    match state {
        OverlayTrustState::Trusted => "trusted",
        OverlayTrustState::Untrusted => "untrusted",
    }
}

fn trust_origin_name(origin: OverlayOrigin) -> &'static str {
    match origin {
        OverlayOrigin::Local => "local",
        OverlayOrigin::Imported => "imported",
    }
}

fn invalid_enum(label: &str, value: &str) -> OverlayManifestError {
    OverlayManifestError::InvalidDomain(format!("Unsupported {label}: {value}"))
}

fn domain_error(error: impl fmt::Display) -> OverlayManifestError {
    OverlayManifestError::InvalidDomain(error.to_string())
}

fn json_error(error: serde_json::Error) -> OverlayManifestError {
    OverlayManifestError::InvalidJson(error.to_string())
}

fn manifest_application_error(error: OverlayManifestError) -> SkillApplicationError {
    let code = match error {
        OverlayManifestError::UnsupportedSchemaVersion { .. }
        | OverlayManifestError::UnsupportedFutureVersion { .. } => {
            OverlayIntegrityCode::UnsupportedSchemaVersion
        }
        OverlayManifestError::InvalidJson(_) | OverlayManifestError::InvalidDomain(_) => {
            OverlayIntegrityCode::DocumentHashMismatch
        }
    };
    integrity(code)
}

fn integrity(code: OverlayIntegrityCode) -> SkillApplicationError {
    OverlayApplicationError::Integrity { code }.into()
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
