use super::{
    DelegationAgentReportV1, DelegationChangeFile, DelegationEvidenceWarning,
    DelegationHostEvidence,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

const MAX_CHANGESET_BYTES: usize = 48 * 1024 * 1024;
const MAX_FILE_PAGE: usize = 100;
const MAX_DIFF_PAGE: usize = 256 * 1024;

pub(crate) struct DelegationChangeSetPayload {
    pub(crate) content_hash: String,
    pub(crate) bytes: Vec<u8>,
}

pub(crate) trait DelegationChangeSetReviewPort: Send + Sync {
    fn load(&self, artifact_id: &str, max_bytes: usize) -> Result<DelegationChangeSetPayload, ()>;
}

pub(crate) struct DelegationChangeSetReviewRequest {
    pub(crate) artifact_id: String,
    pub(crate) file_offset: usize,
    pub(crate) file_limit: usize,
    pub(crate) diff_offset: usize,
    pub(crate) diff_limit: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationChangeSetReviewError {
    InvalidPage,
    ArtifactFailure,
    InvalidSchema,
    IntegrityFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationDiffEncoding {
    Utf8,
    Base64,
}

pub(crate) struct DelegationChangeSetReview {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) repository_identity: String,
    pub(crate) base_commit: String,
    pub(crate) provider: String,
    pub(crate) provenance_fingerprints: Vec<String>,
    pub(crate) file_count: usize,
    pub(crate) binary_file_count: usize,
    pub(crate) files: Vec<DelegationChangeFile>,
    pub(crate) next_file_offset: Option<usize>,
    pub(crate) diff_hash: String,
    pub(crate) diff_encoding: DelegationDiffEncoding,
    pub(crate) diff_data: String,
    pub(crate) next_diff_offset: Option<usize>,
    pub(crate) provider_report: DelegationAgentReportV1,
    pub(crate) host_evidence: DelegationHostEvidence,
    pub(crate) evidence_warnings: Vec<DelegationEvidenceWarning>,
    pub(crate) risk_classification: String,
    pub(crate) limitations: Vec<String>,
    pub(crate) applyable: bool,
    pub(crate) integrity_verified: bool,
    pub(crate) complete_page: bool,
}

pub(crate) struct DelegationChangeSetReviewer {
    artifacts: Arc<dyn DelegationChangeSetReviewPort>,
}

impl DelegationChangeSetReviewer {
    pub(crate) fn new(artifacts: Arc<dyn DelegationChangeSetReviewPort>) -> Self {
        Self { artifacts }
    }

    pub(crate) fn review(
        &self,
        request: DelegationChangeSetReviewRequest,
    ) -> Result<DelegationChangeSetReview, DelegationChangeSetReviewError> {
        validate_page(&request)?;
        let payload = self
            .artifacts
            .load(&request.artifact_id, MAX_CHANGESET_BYTES)
            .map_err(|_| DelegationChangeSetReviewError::ArtifactFailure)?;
        if sha256(&payload.bytes) != payload.content_hash {
            return Err(DelegationChangeSetReviewError::IntegrityFailure);
        }
        let manifest: DelegationChangeSetV1 = serde_json::from_slice(&payload.bytes)
            .map_err(|_| DelegationChangeSetReviewError::InvalidSchema)?;
        if manifest.schema_version != 1 || manifest.artifact_identity.trim().is_empty() {
            return Err(DelegationChangeSetReviewError::InvalidSchema);
        }
        let patch = STANDARD
            .decode(&manifest.patch_base64)
            .map_err(|_| DelegationChangeSetReviewError::InvalidSchema)?;
        if sha256(&patch) != manifest.diff_hash {
            return Err(DelegationChangeSetReviewError::IntegrityFailure);
        }
        let (files, next_file_offset) = file_page(&manifest.files, &request)?;
        let (diff_encoding, diff_data, next_diff_offset) = diff_page(&patch, &request)?;
        let file_count = manifest.files.len();
        let binary_file_count = manifest.files.iter().filter(|file| file.binary).count();
        Ok(DelegationChangeSetReview {
            artifact_id: request.artifact_id,
            content_hash: payload.content_hash,
            delegation_id: manifest.delegation_id,
            attempt_id: manifest.attempt_id,
            repository_identity: manifest.repository_identity,
            base_commit: manifest.base_commit,
            provider: manifest.provider,
            provenance_fingerprints: vec![
                manifest.cli_fingerprint,
                manifest.adapter_fingerprint,
                manifest.prompt_schema_fingerprint,
            ],
            file_count,
            binary_file_count,
            files,
            next_file_offset,
            diff_hash: manifest.diff_hash,
            diff_encoding,
            diff_data,
            next_diff_offset,
            provider_report: manifest.provider_report,
            host_evidence: manifest.host_evidence,
            evidence_warnings: manifest.evidence_warnings,
            risk_classification: manifest.risk_classification,
            limitations: manifest.limitations,
            applyable: manifest.applyable,
            integrity_verified: true,
            complete_page: next_file_offset.is_none() && next_diff_offset.is_none(),
        })
    }
}

#[derive(Deserialize)]
struct DelegationChangeSetV1 {
    schema_version: u16,
    artifact_identity: String,
    delegation_id: String,
    attempt_id: String,
    repository_identity: String,
    base_commit: String,
    provider: String,
    cli_fingerprint: String,
    adapter_fingerprint: String,
    prompt_schema_fingerprint: String,
    files: Vec<DelegationChangeFile>,
    patch_base64: String,
    diff_hash: String,
    provider_report: DelegationAgentReportV1,
    host_evidence: DelegationHostEvidence,
    evidence_warnings: Vec<DelegationEvidenceWarning>,
    risk_classification: String,
    limitations: Vec<String>,
    applyable: bool,
}

fn validate_page(
    request: &DelegationChangeSetReviewRequest,
) -> Result<(), DelegationChangeSetReviewError> {
    if request.artifact_id.trim().is_empty()
        || !(1..=MAX_FILE_PAGE).contains(&request.file_limit)
        || !(1..=MAX_DIFF_PAGE).contains(&request.diff_limit)
    {
        return Err(DelegationChangeSetReviewError::InvalidPage);
    }
    Ok(())
}

fn file_page(
    files: &[DelegationChangeFile],
    request: &DelegationChangeSetReviewRequest,
) -> Result<(Vec<DelegationChangeFile>, Option<usize>), DelegationChangeSetReviewError> {
    if request.file_offset > files.len() {
        return Err(DelegationChangeSetReviewError::InvalidPage);
    }
    let end = request
        .file_offset
        .saturating_add(request.file_limit)
        .min(files.len());
    Ok((
        files[request.file_offset..end].to_vec(),
        (end < files.len()).then_some(end),
    ))
}

fn diff_page(
    patch: &[u8],
    request: &DelegationChangeSetReviewRequest,
) -> Result<(DelegationDiffEncoding, String, Option<usize>), DelegationChangeSetReviewError> {
    if request.diff_offset > patch.len() {
        return Err(DelegationChangeSetReviewError::InvalidPage);
    }
    if let Ok(text) = std::str::from_utf8(patch) {
        if !text.is_char_boundary(request.diff_offset) {
            return Err(DelegationChangeSetReviewError::InvalidPage);
        }
        let mut end = request
            .diff_offset
            .saturating_add(request.diff_limit)
            .min(text.len());
        while end > request.diff_offset && !text.is_char_boundary(end) {
            end -= 1;
        }
        return Ok((
            DelegationDiffEncoding::Utf8,
            text[request.diff_offset..end].to_owned(),
            (end < text.len()).then_some(end),
        ));
    }
    let end = request
        .diff_offset
        .saturating_add(request.diff_limit)
        .min(patch.len());
    Ok((
        DelegationDiffEncoding::Base64,
        STANDARD.encode(&patch[request.diff_offset..end]),
        (end < patch.len()).then_some(end),
    ))
}

fn sha256(bytes: &[u8]) -> String {
    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

#[cfg(test)]
#[path = "changeset_review_tests.rs"]
mod tests;
