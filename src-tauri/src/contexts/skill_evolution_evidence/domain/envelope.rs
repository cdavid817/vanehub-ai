use chrono::DateTime;
use serde::{Deserialize, Serialize};

use super::{
    CliMountSnapshot, EnvelopeCommon, EvidenceSanitizer, EvidenceValidationError, FailureClass,
    FeedbackState, OperationClass, SafeCounts, SanitizationError, SanitizationResult,
    SkillLifecycleAnomaly, SkillLoadOutcome, SourceFamily, TerminalOutcome, UtilityOutcome,
    VerificationClass, VerificationOutcome, MAX_CLI_BINDINGS, MAX_CORRECTION_NOTE_CHARS,
    MAX_OBSERVED_SKILL_REVISIONS,
};

pub(crate) const EVIDENCE_ENVELOPE_SCHEMA_V1: u16 = 1;
const MAX_IDENTIFIER_CHARS: usize = 160;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "sourceKind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum EvidenceSourceEnvelope {
    NativeExecution {
        schema_version: u16,
        common: EnvelopeCommon,
        operation_class: OperationClass,
        outcome: TerminalOutcome,
        failure_class: Option<FailureClass>,
        safe_counts: SafeCounts,
    },
    SkillLoading {
        schema_version: u16,
        common: EnvelopeCommon,
        skill_id: String,
        revision: String,
        outcome: SkillLoadOutcome,
        anomaly: Option<SkillLifecycleAnomaly>,
        observation_count: u32,
    },
    DelegatedUtility {
        schema_version: u16,
        common: EnvelopeCommon,
        utility_skill_id: String,
        revision: String,
        outcome: UtilityOutcome,
        #[serde(default, skip_serializing_if = "is_zero_u64")]
        duration_ms: u64,
        tool_count: u32,
        approval_count: u32,
    },
    #[serde(alias = "plan_verification")]
    RunVerification {
        schema_version: u16,
        common: EnvelopeCommon,
        #[serde(alias = "planRunId")]
        run_id: String,
        verifier: VerificationClass,
        outcome: VerificationOutcome,
        passed_count: u32,
        failed_count: u32,
        predecessor_attempt_id: Option<String>,
    },
    ManagedCli {
        schema_version: u16,
        common: EnvelopeCommon,
        outcome: TerminalOutcome,
        failure_class: Option<FailureClass>,
        mount_snapshot: Option<CliMountSnapshot>,
        configured_binding_ids: Vec<String>,
    },
    InteractiveCli {
        schema_version: u16,
        common: EnvelopeCommon,
        outcome: TerminalOutcome,
        mount_snapshot: Option<CliMountSnapshot>,
        configured_binding_ids: Vec<String>,
    },
    ExplicitFeedback {
        schema_version: u16,
        common: EnvelopeCommon,
        feedback: FeedbackState,
        feedback_revision: u64,
        correction_note: Option<String>,
    },
}

fn is_zero_u64(value: &u64) -> bool {
    *value == 0
}

impl EvidenceSourceEnvelope {
    pub(crate) fn schema_version(&self) -> u16 {
        match self {
            Self::NativeExecution { schema_version, .. }
            | Self::SkillLoading { schema_version, .. }
            | Self::DelegatedUtility { schema_version, .. }
            | Self::RunVerification { schema_version, .. }
            | Self::ManagedCli { schema_version, .. }
            | Self::InteractiveCli { schema_version, .. }
            | Self::ExplicitFeedback { schema_version, .. } => *schema_version,
        }
    }

    pub(crate) fn source_family(&self) -> SourceFamily {
        match self {
            Self::NativeExecution { .. } => SourceFamily::NativeExecution,
            Self::SkillLoading { .. } => SourceFamily::SkillLoading,
            Self::DelegatedUtility { .. } => SourceFamily::DelegatedUtility,
            Self::RunVerification { .. } => SourceFamily::RunVerification,
            Self::ManagedCli { .. } => SourceFamily::ManagedCli,
            Self::InteractiveCli { .. } => SourceFamily::InteractiveCli,
            Self::ExplicitFeedback { .. } => SourceFamily::ExplicitFeedback,
        }
    }

    pub(crate) fn sanitized_registered_text(
        &self,
        sanitizer: &EvidenceSanitizer,
    ) -> Result<Option<SanitizationResult>, SanitizationError> {
        match self {
            Self::ExplicitFeedback {
                correction_note: Some(note),
                ..
            } => sanitizer.sanitize(note).map(Some),
            _ => Ok(None),
        }
    }

    pub(crate) fn validate(&self) -> Result<(), EvidenceValidationError> {
        if self.schema_version() != EVIDENCE_ENVELOPE_SCHEMA_V1 {
            return Err(EvidenceValidationError::UnsupportedSchemaVersion(
                self.schema_version(),
            ));
        }
        validate_common(self.common())?;

        match self {
            Self::SkillLoading {
                skill_id, revision, ..
            } => {
                validate_identifier(skill_id, "skill_id")?;
                validate_identifier(revision, "revision")
            }
            Self::DelegatedUtility {
                utility_skill_id,
                revision,
                ..
            } => {
                validate_identifier(utility_skill_id, "utility_skill_id")?;
                validate_identifier(revision, "revision")
            }
            Self::RunVerification {
                run_id,
                predecessor_attempt_id,
                ..
            } => {
                validate_identifier(run_id, "run_id")?;
                validate_optional_identifier(predecessor_attempt_id, "predecessor_attempt_id")
            }
            Self::ManagedCli {
                mount_snapshot,
                configured_binding_ids,
                ..
            }
            | Self::InteractiveCli {
                mount_snapshot,
                configured_binding_ids,
                ..
            } => validate_cli_fields(mount_snapshot.as_ref(), configured_binding_ids),
            Self::ExplicitFeedback {
                correction_note, ..
            } => validate_correction_note(correction_note.as_deref()),
            Self::NativeExecution { .. } => Ok(()),
        }
    }

    pub(crate) fn common(&self) -> &EnvelopeCommon {
        match self {
            Self::NativeExecution { common, .. }
            | Self::SkillLoading { common, .. }
            | Self::DelegatedUtility { common, .. }
            | Self::RunVerification { common, .. }
            | Self::ManagedCli { common, .. }
            | Self::InteractiveCli { common, .. }
            | Self::ExplicitFeedback { common, .. } => common,
        }
    }
}

fn validate_common(common: &EnvelopeCommon) -> Result<(), EvidenceValidationError> {
    validate_identifier(&common.source_event_id, "source_event_id")?;
    validate_timestamp(&common.occurred_at, "occurred_at")?;
    validate_optional_identifier(&common.stable_agent_id, "stable_agent_id")?;
    validate_optional_identifier(&common.session_id, "session_id")?;
    validate_optional_identifier(&common.message_id, "message_id")?;
    validate_optional_identifier(&common.run_id, "run_id")?;
    validate_optional_identifier(&common.attempt_id, "attempt_id")?;
    validate_optional_identifier(&common.workspace, "workspace")?;

    if common.observed_skill_revisions.len() > MAX_OBSERVED_SKILL_REVISIONS {
        return Err(EvidenceValidationError::TooManyObservedSkillRevisions {
            max: MAX_OBSERVED_SKILL_REVISIONS,
        });
    }
    for observation in &common.observed_skill_revisions {
        validate_identifier(&observation.skill_id, "observed_skill_id")?;
        validate_identifier(&observation.revision, "observed_skill_revision")?;
        validate_timestamp(&observation.observed_at, "observed_at")?;
    }
    Ok(())
}

fn validate_cli_fields(
    snapshot: Option<&CliMountSnapshot>,
    configured_binding_ids: &[String],
) -> Result<(), EvidenceValidationError> {
    if configured_binding_ids.len() > MAX_CLI_BINDINGS {
        return Err(EvidenceValidationError::TooManyCliBindings {
            max: MAX_CLI_BINDINGS,
        });
    }
    for binding_id in configured_binding_ids {
        validate_identifier(binding_id, "configured_binding_id")?;
    }
    if let Some(snapshot) = snapshot {
        validate_identifier(&snapshot.manifest_hash, "mount_manifest_hash")?;
        if snapshot.skills.len() > MAX_OBSERVED_SKILL_REVISIONS {
            return Err(EvidenceValidationError::TooManyObservedSkillRevisions {
                max: MAX_OBSERVED_SKILL_REVISIONS,
            });
        }
        for skill in &snapshot.skills {
            validate_identifier(&skill.skill_id, "mounted_skill_id")?;
            validate_identifier(&skill.revision, "mounted_skill_revision")?;
        }
    }
    Ok(())
}

fn validate_correction_note(note: Option<&str>) -> Result<(), EvidenceValidationError> {
    if note.is_some_and(|value| value.chars().count() > MAX_CORRECTION_NOTE_CHARS) {
        return Err(EvidenceValidationError::CorrectionNoteTooLong {
            max: MAX_CORRECTION_NOTE_CHARS,
        });
    }
    Ok(())
}

fn validate_timestamp(value: &str, field: &'static str) -> Result<(), EvidenceValidationError> {
    DateTime::parse_from_rfc3339(value)
        .map(|_| ())
        .map_err(|_| EvidenceValidationError::InvalidTimestamp(field))
}

fn validate_optional_identifier(
    value: &Option<String>,
    field: &'static str,
) -> Result<(), EvidenceValidationError> {
    if let Some(value) = value {
        validate_identifier(value, field)?;
    }
    Ok(())
}

fn validate_identifier(value: &str, field: &'static str) -> Result<(), EvidenceValidationError> {
    let valid = !value.is_empty()
        && value.chars().count() <= MAX_IDENTIFIER_CHARS
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/' | b'@')
        });
    if valid {
        Ok(())
    } else {
        Err(EvidenceValidationError::MalformedIdentifier(field))
    }
}
