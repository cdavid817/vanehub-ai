//! The execution evidence journal's domain.
//!
//! Submodules are `pub(crate)` so a consumer can reach a bound or a helper it needs without this
//! file re-exporting the entire surface speculatively; the re-exports below are the names the rest
//! of the crate actually uses today, and later task groups extend the list as they consume more.
pub(crate) mod correlation;
pub(crate) mod coverage;
pub(crate) mod encoding;
pub(crate) mod error;
pub(crate) mod event;
pub(crate) mod identity;
pub(crate) mod payload;
pub(crate) mod safety;

pub(crate) use correlation::EvidenceCorrelation;
pub(crate) use coverage::{reason_codes, EvidenceCoverageState, QueryCoverage};
pub(crate) use error::EvidenceDomainError;
pub(crate) use event::{fidelity_token, parse_fidelity_token, parse_status_token, status_token};
pub(crate) use event::{ExecutionEvidenceEvent, ExecutionEvidenceEventInput, RedactionReceipt};
pub(crate) use identity::{
    EvidenceAgentId, EvidenceCommandId, EvidenceEventId, EvidenceFileMutationId,
    EvidenceOperationId, EvidenceSeatId, EvidenceSessionId, EvidenceSourceContext,
    EvidenceToolCallId, SafeReasonCode, SourceEventId,
};
pub(crate) use payload::{
    CommandRuntimeKind, EvidenceKind, OutputAvailability, SafeEvidencePayload, VerificationOutcome,
    EVIDENCE_SCHEMA_VERSION,
};

#[cfg(test)]
pub(crate) mod builders;

#[cfg(test)]
mod tests;
