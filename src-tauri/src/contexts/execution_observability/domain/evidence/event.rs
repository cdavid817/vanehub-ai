use super::correlation::EvidenceCorrelation;
use super::error::EvidenceDomainError;
use super::identity::{EvidenceEventId, EvidenceSourceContext, SafeReasonCode, SourceEventId};
use super::payload::{
    EvidenceKind, SafeEvidencePayload, EVIDENCE_SCHEMA_VERSION, MAX_SAFE_PAYLOAD_BYTES,
};
use crate::contexts::execution_observability::domain::{ExecutionFidelity, ExecutionStatus};
use sha2::{Digest, Sha256};

pub(crate) const MAX_REDACTION_RULE_IDS: usize = 16;
const MAX_TIMESTAMP_LENGTH: usize = 40;

/// What the producer says it removed before handing the event over.
///
/// Rule ids only, never the values they matched. Sorted and de-duplicated so the same removal
/// reported twice cannot change an event's fingerprint and turn an idempotent retry into a
/// conflict.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct RedactionReceipt {
    applied: bool,
    rule_ids: Vec<SafeReasonCode>,
}

impl RedactionReceipt {
    pub(crate) fn none() -> Self {
        Self::default()
    }

    pub(crate) fn applied(
        rule_ids: impl IntoIterator<Item = SafeReasonCode>,
    ) -> Result<Self, EvidenceDomainError> {
        let mut rule_ids = rule_ids.into_iter().collect::<Vec<_>>();
        rule_ids.sort();
        rule_ids.dedup();
        if rule_ids.len() > MAX_REDACTION_RULE_IDS {
            return Err(EvidenceDomainError::TooManyRedactionRules {
                max: MAX_REDACTION_RULE_IDS,
            });
        }
        Ok(Self {
            applied: !rule_ids.is_empty(),
            rule_ids,
        })
    }

    pub(crate) fn is_applied(&self) -> bool {
        self.applied
    }

    pub(crate) fn rule_ids(&self) -> &[SafeReasonCode] {
        &self.rule_ids
    }
}

/// One append-only journal entry.
///
/// Construction is the only way to obtain one, and construction validates: a value that reaches
/// the repository has already satisfied every invariant, so persistence has no policy of its own
/// to get wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutionEvidenceEvent {
    event_id: EvidenceEventId,
    source_context: EvidenceSourceContext,
    source_event_id: SourceEventId,
    schema_version: u16,
    occurred_at: String,
    correlation: EvidenceCorrelation,
    status: Option<ExecutionStatus>,
    fidelity: ExecutionFidelity,
    payload: SafeEvidencePayload,
    redaction: RedactionReceipt,
}

pub(crate) struct ExecutionEvidenceEventInput {
    pub(crate) event_id: EvidenceEventId,
    pub(crate) source_context: EvidenceSourceContext,
    pub(crate) source_event_id: SourceEventId,
    pub(crate) schema_version: u16,
    pub(crate) occurred_at: String,
    pub(crate) correlation: EvidenceCorrelation,
    pub(crate) status: Option<ExecutionStatus>,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) payload: SafeEvidencePayload,
    pub(crate) redaction: RedactionReceipt,
}

impl ExecutionEvidenceEvent {
    pub(crate) fn new(input: ExecutionEvidenceEventInput) -> Result<Self, EvidenceDomainError> {
        if input.schema_version == 0 || input.schema_version > EVIDENCE_SCHEMA_VERSION {
            return Err(EvidenceDomainError::UnsupportedSchemaVersion {
                version: input.schema_version,
            });
        }
        if input.occurred_at.is_empty() || input.occurred_at.len() > MAX_TIMESTAMP_LENGTH {
            return Err(EvidenceDomainError::InvalidTimestamp);
        }
        input.correlation.validate()?;
        input.payload.validate()?;
        require_lifecycle_correlation(&input.payload, &input.correlation)?;
        require_status_matches_payload(&input.payload, input.status)?;

        let event = Self {
            event_id: input.event_id,
            source_context: input.source_context,
            source_event_id: input.source_event_id,
            schema_version: input.schema_version,
            occurred_at: input.occurred_at,
            correlation: input.correlation,
            status: input.status,
            fidelity: input.fidelity,
            payload: input.payload,
            redaction: input.redaction,
        };
        // Checked after construction so no combination of individually bounded fields can add up
        // to an unbounded row.
        if event.canonical_payload_encoding().len() > MAX_SAFE_PAYLOAD_BYTES {
            return Err(EvidenceDomainError::PayloadTooLarge {
                max: MAX_SAFE_PAYLOAD_BYTES,
            });
        }
        Ok(event)
    }

    pub(crate) fn event_id(&self) -> &EvidenceEventId {
        &self.event_id
    }

    pub(crate) fn source_context(&self) -> EvidenceSourceContext {
        self.source_context
    }

    pub(crate) fn source_event_id(&self) -> &SourceEventId {
        &self.source_event_id
    }

    pub(crate) fn schema_version(&self) -> u16 {
        self.schema_version
    }

    pub(crate) fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    pub(crate) fn correlation(&self) -> &EvidenceCorrelation {
        &self.correlation
    }

    pub(crate) fn kind(&self) -> EvidenceKind {
        self.payload.kind()
    }

    pub(crate) fn status(&self) -> Option<ExecutionStatus> {
        self.status
    }

    pub(crate) fn fidelity(&self) -> ExecutionFidelity {
        self.fidelity
    }

    pub(crate) fn payload(&self) -> &SafeEvidencePayload {
        &self.payload
    }

    pub(crate) fn redaction(&self) -> &RedactionReceipt {
        &self.redaction
    }

    /// Identity by content, used to tell an idempotent retry from a conflicting reuse of a source
    /// id in one lookup.
    ///
    /// Excludes the generated event id, the assigned sequence, and the insertion timestamp: those
    /// differ on every attempt by construction, and including them would make every retry look
    /// like a conflict. Includes everything the producer actually asserted.
    pub(crate) fn canonical_fingerprint(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.canonical_encoding().as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn canonical_encoding(&self) -> String {
        let mut parts = vec![
            (
                "source_context".to_string(),
                self.source_context.as_str().to_string(),
            ),
            (
                "source_event_id".to_string(),
                self.source_event_id.as_str().to_string(),
            ),
            (
                "schema_version".to_string(),
                self.schema_version.to_string(),
            ),
            ("occurred_at".to_string(), self.occurred_at.clone()),
            ("kind".to_string(), self.kind().as_str().to_string()),
            (
                "status".to_string(),
                self.status
                    .map(status_token)
                    .unwrap_or_default()
                    .to_string(),
            ),
            (
                "fidelity".to_string(),
                fidelity_token(self.fidelity).to_string(),
            ),
            (
                "redaction_applied".to_string(),
                self.redaction.is_applied().to_string(),
            ),
            (
                "redaction_rules".to_string(),
                self.redaction
                    .rule_ids()
                    .iter()
                    .map(SafeReasonCode::as_str)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
        ];
        for (key, value) in self.correlation.canonical_parts() {
            parts.push((format!("correlation.{key}"), value));
        }
        parts.push(("payload".to_string(), self.canonical_payload_encoding()));
        parts
            .into_iter()
            .map(|(key, value)| format!("{key}={}", escape(&value)))
            .collect::<Vec<_>>()
            .join("\u{1f}")
    }

    fn canonical_payload_encoding(&self) -> String {
        super::encoding::canonical_payload_encoding(&self.payload)
    }
}

/// Separator-safe so two different field pairs can never encode to the same byte sequence.
fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\u{1f}', "\\u001f")
}

pub(crate) fn status_token(status: ExecutionStatus) -> &'static str {
    match status {
        ExecutionStatus::Accepted => "queued",
        ExecutionStatus::Running => "running",
        ExecutionStatus::Succeeded => "succeeded",
        ExecutionStatus::Failed => "failed",
        ExecutionStatus::Cancelled => "cancelled",
        ExecutionStatus::Incomplete => "incomplete",
    }
}

pub(crate) fn parse_status_token(value: &str) -> Option<ExecutionStatus> {
    Some(match value {
        "queued" => ExecutionStatus::Accepted,
        "running" => ExecutionStatus::Running,
        "succeeded" => ExecutionStatus::Succeeded,
        "failed" => ExecutionStatus::Failed,
        "cancelled" => ExecutionStatus::Cancelled,
        "incomplete" => ExecutionStatus::Incomplete,
        _ => return None,
    })
}

pub(crate) fn fidelity_token(fidelity: ExecutionFidelity) -> &'static str {
    match fidelity {
        ExecutionFidelity::Native => "native",
        ExecutionFidelity::Proxied => "proxied",
        ExecutionFidelity::Inferred => "inferred",
        ExecutionFidelity::Opaque => "opaque",
    }
}

pub(crate) fn parse_fidelity_token(value: &str) -> Option<ExecutionFidelity> {
    Some(match value {
        "native" => ExecutionFidelity::Native,
        "proxied" => ExecutionFidelity::Proxied,
        "inferred" => ExecutionFidelity::Inferred,
        "opaque" => ExecutionFidelity::Opaque,
        _ => return None,
    })
}

/// A lifecycle event without the id of the thing whose lifecycle it describes is unusable: it can
/// be counted but never joined to, which is the same as not having recorded it.
fn require_lifecycle_correlation(
    payload: &SafeEvidencePayload,
    correlation: &EvidenceCorrelation,
) -> Result<(), EvidenceDomainError> {
    let kind = payload.kind();
    let missing = |field: &'static str| EvidenceDomainError::MissingCorrelation {
        kind: kind.as_str(),
        field,
    };
    match payload {
        SafeEvidencePayload::RunStarted { .. } | SafeEvidencePayload::RunCompleted { .. } => {
            correlation
                .run_id
                .as_ref()
                .ok_or_else(|| missing("run id"))?;
        }
        SafeEvidencePayload::ToolStarted { .. } | SafeEvidencePayload::ToolCompleted { .. } => {
            correlation
                .tool_call_id
                .as_ref()
                .ok_or_else(|| missing("tool call id"))?;
        }
        SafeEvidencePayload::CommandStarted { .. }
        | SafeEvidencePayload::CommandCompleted { .. } => {
            correlation
                .command_id
                .as_ref()
                .ok_or_else(|| missing("command id"))?;
        }
        SafeEvidencePayload::FileMutationObserved { .. } => {
            correlation
                .file_mutation_id
                .as_ref()
                .ok_or_else(|| missing("file mutation id"))?;
        }
        SafeEvidencePayload::AgentDelegated { .. } | SafeEvidencePayload::AgentCompleted { .. } => {
            correlation
                .agent_id
                .as_ref()
                .ok_or_else(|| missing("agent id"))?;
        }
        SafeEvidencePayload::OperationFailed { .. } => {
            correlation
                .operation_id
                .as_ref()
                .ok_or_else(|| missing("operation id"))?;
        }
        _ => {}
    }
    Ok(())
}

/// A start cannot be terminal and a completion cannot be non-terminal. Allowing either would let
/// the projection derive a duration for work that had not finished.
fn require_status_matches_payload(
    payload: &SafeEvidencePayload,
    status: Option<ExecutionStatus>,
) -> Result<(), EvidenceDomainError> {
    let kind = payload.kind();
    let mismatch = || EvidenceDomainError::PayloadKindMismatch {
        kind: kind.as_str(),
    };
    let is_start = matches!(
        payload,
        SafeEvidencePayload::RunStarted { .. }
            | SafeEvidencePayload::ToolStarted { .. }
            | SafeEvidencePayload::CommandStarted { .. }
            | SafeEvidencePayload::ShellOpened { .. }
            | SafeEvidencePayload::AgentDelegated { .. }
    );
    let is_completion = matches!(
        payload,
        SafeEvidencePayload::RunCompleted { .. }
            | SafeEvidencePayload::ToolCompleted { .. }
            | SafeEvidencePayload::CommandCompleted { .. }
            | SafeEvidencePayload::AgentCompleted { .. }
            | SafeEvidencePayload::VerificationCompleted { .. }
    );
    match status {
        Some(status) if is_start && status.is_terminal() => Err(mismatch()),
        Some(status) if is_completion && !status.is_terminal() => Err(mismatch()),
        None if is_completion => Err(mismatch()),
        _ => Ok(()),
    }
}
