use serde::{Deserialize, Serialize};

use super::{DelegationAgentReportV1, DelegationHostEvidence, DelegationReportOutcome};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", rename_all = "snake_case")]
pub(crate) enum DelegationEvidenceWarning {
    ProviderActionNotHostObserved { action_index: usize },
    ProviderVerificationNotHostObserved { claim_index: usize },
    HostChangedFileNotProviderReported { path: String },
    ProviderOutcomeConflictsWithHost { host_reason: String },
}

pub(crate) struct DelegationReportComparator;

impl DelegationReportComparator {
    pub(crate) fn compare(
        report: &DelegationAgentReportV1,
        host: &DelegationHostEvidence,
    ) -> Vec<DelegationEvidenceWarning> {
        let mut warnings = Vec::new();
        for (action_index, action) in report.actions_taken.iter().enumerate() {
            if !host
                .observed_actions
                .iter()
                .any(|observed| statements_match(action, observed))
            {
                warnings.push(DelegationEvidenceWarning::ProviderActionNotHostObserved {
                    action_index,
                });
            }
        }
        for (claim_index, claim) in report.verification_claims.iter().enumerate() {
            if !host
                .observed_actions
                .iter()
                .any(|observed| statements_match(claim, observed))
            {
                warnings.push(
                    DelegationEvidenceWarning::ProviderVerificationNotHostObserved { claim_index },
                );
            }
        }
        for path in &host.changed_files {
            if !provider_report_mentions_path(report, path) {
                warnings.push(
                    DelegationEvidenceWarning::HostChangedFileNotProviderReported {
                        path: path.clone(),
                    },
                );
            }
        }
        if report.outcome == DelegationReportOutcome::Completed {
            if host.exit_code != 0 {
                warnings.push(
                    DelegationEvidenceWarning::ProviderOutcomeConflictsWithHost {
                        host_reason: "non_zero_exit".into(),
                    },
                );
            }
            if !host.policy_violations.is_empty() {
                warnings.push(
                    DelegationEvidenceWarning::ProviderOutcomeConflictsWithHost {
                        host_reason: "policy_violation".into(),
                    },
                );
            }
            if !host.cleanup_succeeded {
                warnings.push(
                    DelegationEvidenceWarning::ProviderOutcomeConflictsWithHost {
                        host_reason: "cleanup_failed".into(),
                    },
                );
            }
        }
        warnings
    }
}

fn statements_match(provider: &str, host: &str) -> bool {
    provider.trim().eq_ignore_ascii_case(host.trim())
}

fn provider_report_mentions_path(report: &DelegationAgentReportV1, path: &str) -> bool {
    let path = path.replace('\\', "/").to_lowercase();
    std::iter::once(report.summary.as_str())
        .chain(report.findings.iter().map(String::as_str))
        .chain(report.actions_taken.iter().map(String::as_str))
        .any(|statement| statement.replace('\\', "/").to_lowercase().contains(&path))
}

#[cfg(test)]
#[path = "report_comparison_tests.rs"]
mod tests;
