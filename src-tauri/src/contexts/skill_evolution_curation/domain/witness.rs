use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorWitnessBundleV1 {
    pub(crate) schema_version: u16,
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) draft_id: Option<String>,
    pub(crate) draft_revision: Option<u64>,
    pub(crate) draft_hash: Option<String>,
    pub(crate) assessment_hash: String,
    pub(crate) target_revision: String,
    pub(crate) base_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) overlay_revision: String,
    pub(crate) pin_witness: String,
    pub(crate) trust_witness: String,
    pub(crate) scanner_version: String,
    pub(crate) policy_revision: u64,
    pub(crate) preview_hash: Option<String>,
}

impl CuratorWitnessBundleV1 {
    pub(crate) fn canonical_hash(&self) -> Result<String, CuratorWitnessError> {
        let bytes = serde_json::to_vec(self).map_err(|_| CuratorWitnessError::Serialization)?;
        let digest = Sha256::digest(bytes);
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        Ok(format!("sha256:{hex}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OptimisticRevisionConflict {
    pub(crate) expected: u64,
    pub(crate) actual: u64,
}

pub(crate) fn require_revision(
    expected: u64,
    actual: u64,
) -> Result<(), OptimisticRevisionConflict> {
    if expected == actual {
        Ok(())
    } else {
        Err(OptimisticRevisionConflict { expected, actual })
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorWitnessError {
    #[error("curator witness serialization failed")]
    Serialization,
}
