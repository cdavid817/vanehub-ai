use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_REPORT_BYTES: usize = 1024 * 1024;
const MAX_SUMMARY_BYTES: usize = 64 * 1024;
const MAX_ITEMS: usize = 128;
const MAX_ITEM_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DelegationReportOutcome {
    Completed,
    Blocked,
    NeedsInput,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub(crate) struct DelegationAgentReportV1 {
    pub(crate) schema_version: u16,
    pub(crate) outcome: DelegationReportOutcome,
    pub(crate) summary: String,
    pub(crate) findings: Vec<String>,
    pub(crate) actions_taken: Vec<String>,
    pub(crate) verification_claims: Vec<String>,
    pub(crate) risks: Vec<String>,
    pub(crate) follow_ups: Vec<String>,
    pub(crate) limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DelegationEvidenceRole {
    ProviderReported,
    HostObserved,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DelegationVerificationClaim {
    pub(crate) role: DelegationEvidenceRole,
    pub(crate) claim: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) struct DelegationHostEvidence {
    pub(crate) base_commit: String,
    pub(crate) changed_files: Vec<String>,
    pub(crate) diff_hash: Option<String>,
    pub(crate) exit_code: i32,
    pub(crate) observed_actions: Vec<String>,
    pub(crate) policy_violations: Vec<String>,
    pub(crate) cleanup_succeeded: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationReportError {
    InvalidSchema,
    InvalidVersion,
    LimitExceeded,
}

pub(crate) struct DelegationReportNormalizer;

impl DelegationReportNormalizer {
    pub(crate) fn normalize(
        value: Value,
    ) -> Result<DelegationAgentReportV1, DelegationReportError> {
        let bytes = serde_json::to_vec(&value).map_err(|_| DelegationReportError::InvalidSchema)?;
        if bytes.len() > MAX_REPORT_BYTES {
            return Err(DelegationReportError::LimitExceeded);
        }
        let report: DelegationAgentReportV1 =
            serde_json::from_value(value).map_err(|_| DelegationReportError::InvalidSchema)?;
        if report.schema_version != 1 {
            return Err(DelegationReportError::InvalidVersion);
        }
        if report.summary.trim().is_empty() || report.summary.len() > MAX_SUMMARY_BYTES {
            return Err(DelegationReportError::LimitExceeded);
        }
        for items in report.collections() {
            if items.len() > MAX_ITEMS
                || items
                    .iter()
                    .any(|item| item.trim().is_empty() || item.len() > MAX_ITEM_BYTES)
            {
                return Err(DelegationReportError::LimitExceeded);
            }
        }
        Ok(report)
    }

    pub(crate) fn provider_claims(
        report: &DelegationAgentReportV1,
    ) -> Vec<DelegationVerificationClaim> {
        report
            .verification_claims
            .iter()
            .cloned()
            .map(|claim| DelegationVerificationClaim {
                role: DelegationEvidenceRole::ProviderReported,
                claim,
            })
            .collect()
    }
}

impl DelegationAgentReportV1 {
    fn collections(&self) -> [&[String]; 6] {
        [
            &self.findings,
            &self.actions_taken,
            &self.verification_claims,
            &self.risks,
            &self.follow_ups,
            &self.limitations,
        ]
    }
}

#[cfg(test)]
#[path = "report_tests.rs"]
mod tests;
