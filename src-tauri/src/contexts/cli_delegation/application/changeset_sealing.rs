use super::{
    DelegationAgentReportV1, DelegationChangeFile, DelegationChangeSetCapture,
    DelegationEvidenceWarning, DelegationHostEvidence, DelegationReportComparator,
    DelegationTarget,
};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationChangeSetArtifact {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) diff_hash: String,
}

pub(crate) trait DelegationChangeSetArtifactPort: Send + Sync {
    fn seal_json(
        &self,
        operation_id: &str,
        attempt_id: &str,
        created_at: &str,
        value: &serde_json::Value,
    ) -> Result<(String, String), ()>;
}

pub(crate) struct DelegationChangeSetSealRequest {
    pub(crate) artifact_identity: String,
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) repository_identity: String,
    pub(crate) provider: DelegationTarget,
    pub(crate) cli_fingerprint: String,
    pub(crate) adapter_fingerprint: String,
    pub(crate) prompt_schema_fingerprint: String,
    pub(crate) capture: DelegationChangeSetCapture,
    pub(crate) provider_report: DelegationAgentReportV1,
    pub(crate) host_evidence: DelegationHostEvidence,
    pub(crate) risk_classification: String,
    pub(crate) limitations: Vec<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationChangeSetSealError {
    InvalidRequest,
    SerializationFailure,
    ArtifactFailure,
    IntegrityFailure,
}

pub(crate) struct DelegationChangeSetSealer {
    artifacts: Arc<dyn DelegationChangeSetArtifactPort>,
}

impl DelegationChangeSetSealer {
    pub(crate) fn new(artifacts: Arc<dyn DelegationChangeSetArtifactPort>) -> Self {
        Self { artifacts }
    }

    pub(crate) fn seal(
        &self,
        request: DelegationChangeSetSealRequest,
    ) -> Result<DelegationChangeSetArtifact, DelegationChangeSetSealError> {
        validate(&request)?;
        let patch_hash = sha256(&request.capture.canonical_patch);
        if patch_hash != request.capture.diff_hash {
            return Err(DelegationChangeSetSealError::IntegrityFailure);
        }
        let evidence_warnings =
            DelegationReportComparator::compare(&request.provider_report, &request.host_evidence);
        let manifest = DelegationChangeSetV1 {
            schema_version: 1,
            artifact_identity: &request.artifact_identity,
            delegation_id: &request.delegation_id,
            attempt_id: &request.attempt_id,
            repository_identity: &request.repository_identity,
            base_commit: &request.capture.base_commit,
            provider: request.provider.as_str(),
            cli_fingerprint: &request.cli_fingerprint,
            adapter_fingerprint: &request.adapter_fingerprint,
            prompt_schema_fingerprint: &request.prompt_schema_fingerprint,
            files: &request.capture.files,
            patch_base64: STANDARD.encode(&request.capture.canonical_patch),
            diff_hash: &request.capture.diff_hash,
            provider_report: &request.provider_report,
            host_evidence: &request.host_evidence,
            evidence_warnings: &evidence_warnings,
            risk_classification: &request.risk_classification,
            limitations: &request.limitations,
            applyable: true,
        };
        let value = serde_json::to_value(manifest)
            .map_err(|_| DelegationChangeSetSealError::SerializationFailure)?;
        let (artifact_id, content_hash) = self
            .artifacts
            .seal_json(
                &request.delegation_id,
                &request.attempt_id,
                &request.created_at,
                &value,
            )
            .map_err(|_| DelegationChangeSetSealError::ArtifactFailure)?;
        if artifact_id.is_empty() || !content_hash.starts_with("sha256:") {
            return Err(DelegationChangeSetSealError::IntegrityFailure);
        }
        Ok(DelegationChangeSetArtifact {
            artifact_id,
            content_hash,
            diff_hash: request.capture.diff_hash,
        })
    }
}

#[derive(Serialize)]
struct DelegationChangeSetV1<'a> {
    schema_version: u16,
    artifact_identity: &'a str,
    delegation_id: &'a str,
    attempt_id: &'a str,
    repository_identity: &'a str,
    base_commit: &'a str,
    provider: &'a str,
    cli_fingerprint: &'a str,
    adapter_fingerprint: &'a str,
    prompt_schema_fingerprint: &'a str,
    files: &'a [DelegationChangeFile],
    patch_base64: String,
    diff_hash: &'a str,
    provider_report: &'a DelegationAgentReportV1,
    host_evidence: &'a DelegationHostEvidence,
    evidence_warnings: &'a [DelegationEvidenceWarning],
    risk_classification: &'a str,
    limitations: &'a [String],
    applyable: bool,
}

fn validate(request: &DelegationChangeSetSealRequest) -> Result<(), DelegationChangeSetSealError> {
    if [
        request.artifact_identity.as_str(),
        request.delegation_id.as_str(),
        request.attempt_id.as_str(),
        request.repository_identity.as_str(),
        request.cli_fingerprint.as_str(),
        request.adapter_fingerprint.as_str(),
        request.prompt_schema_fingerprint.as_str(),
        request.risk_classification.as_str(),
        request.created_at.as_str(),
    ]
    .iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(DelegationChangeSetSealError::InvalidRequest);
    }
    if request.provider_report.schema_version != 1
        || request.host_evidence.base_commit != request.capture.base_commit
        || request.host_evidence.diff_hash.as_deref() != Some(&request.capture.diff_hash)
        || request.host_evidence.exit_code != 0
        || !request.host_evidence.policy_violations.is_empty()
        || !request.host_evidence.cleanup_succeeded
    {
        return Err(DelegationChangeSetSealError::InvalidRequest);
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let hex = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

#[cfg(test)]
#[path = "changeset_sealing_tests.rs"]
mod tests;
