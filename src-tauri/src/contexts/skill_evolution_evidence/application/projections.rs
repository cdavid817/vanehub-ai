use std::sync::Arc;

use zeroize::Zeroizing;

use crate::contexts::skill_evolution_evidence::domain::{
    canonical_workspace_scope, CliMountSnapshot, EnvelopeCommon, EvidenceSourceEnvelope,
    FailureClass, OperationClass, SafeCounts, SkillLifecycleAnomaly, SkillLoadOutcome,
    TerminalOutcome, UtilityOutcome, VerificationClass, VerificationOutcome,
    EVIDENCE_ENVELOPE_SCHEMA_V1,
};

use super::{EnqueueOutcome, EvidencePipeline};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectionDisposition {
    Disabled,
    Accepted,
    Dropped,
}

pub(crate) trait EvidenceProjectionSink: Send + Sync {
    fn submit(&self, envelope: EvidenceSourceEnvelope) -> ProjectionDisposition;
}

impl EvidenceProjectionSink for EvidencePipeline {
    fn submit(&self, envelope: EvidenceSourceEnvelope) -> ProjectionDisposition {
        match self.enqueue(envelope) {
            EnqueueOutcome::Accepted { .. } => ProjectionDisposition::Accepted,
            EnqueueOutcome::Dropped { .. } | EnqueueOutcome::Closed { .. } => {
                ProjectionDisposition::Dropped
            }
        }
    }
}

#[derive(Clone)]
pub(crate) struct RuntimeEvidenceProjector {
    sink: Option<Arc<dyn EvidenceProjectionSink>>,
    workspace_key: Option<Arc<Zeroizing<Vec<u8>>>>,
}

impl RuntimeEvidenceProjector {
    pub(crate) fn disabled() -> Self {
        Self {
            sink: None,
            workspace_key: None,
        }
    }

    pub(crate) fn enabled(sink: Arc<dyn EvidenceProjectionSink>, workspace_key: &[u8]) -> Self {
        Self {
            sink: Some(sink),
            workspace_key: Some(Arc::new(Zeroizing::new(workspace_key.to_vec()))),
        }
    }

    pub(crate) fn workspace_scope(&self, workspace: Option<&str>) -> Option<String> {
        workspace.and_then(|value| {
            self.workspace_key
                .as_deref()
                .and_then(|key| canonical_workspace_scope(key, value).ok())
        })
    }

    pub(crate) fn native(&self, fact: NativeExecutionFact) -> ProjectionDisposition {
        self.submit(EvidenceSourceEnvelope::NativeExecution {
            schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
            common: fact.common,
            operation_class: fact.operation_class,
            outcome: fact.outcome,
            failure_class: fact.failure_class,
            safe_counts: fact.safe_counts,
        })
    }

    pub(crate) fn skill_lifecycle(&self, fact: SkillLifecycleFact) -> ProjectionDisposition {
        self.submit(EvidenceSourceEnvelope::SkillLoading {
            schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
            common: fact.common,
            skill_id: fact.skill_id,
            revision: fact.revision,
            outcome: fact.outcome,
            anomaly: fact.anomaly,
            observation_count: fact.observation_count,
        })
    }

    pub(crate) fn delegation(&self, fact: DelegatedUtilityFact) -> ProjectionDisposition {
        self.submit(EvidenceSourceEnvelope::DelegatedUtility {
            schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
            common: fact.common,
            utility_skill_id: fact.utility_skill_id,
            revision: fact.revision,
            outcome: fact.outcome,
            duration_ms: fact.duration_ms,
            tool_count: fact.tool_count,
            approval_count: fact.approval_count,
        })
    }

    pub(crate) fn verification(&self, fact: RunVerificationFact) -> ProjectionDisposition {
        self.submit(EvidenceSourceEnvelope::RunVerification {
            schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
            common: fact.common,
            run_id: fact.run_id,
            verifier: fact.verifier,
            outcome: fact.outcome,
            passed_count: fact.passed_count,
            failed_count: fact.failed_count,
            predecessor_attempt_id: fact.predecessor_attempt_id,
        })
    }

    pub(crate) fn cli(&self, fact: CliLifecycleFact) -> ProjectionDisposition {
        let CliLifecycleFact {
            kind,
            common,
            outcome,
            failure_class,
            mount_snapshot,
            configured_binding_ids,
        } = fact;
        let envelope = match kind {
            CliRuntimeKind::Managed => EvidenceSourceEnvelope::ManagedCli {
                schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
                common,
                outcome,
                failure_class,
                mount_snapshot,
                configured_binding_ids,
            },
            CliRuntimeKind::Interactive => EvidenceSourceEnvelope::InteractiveCli {
                schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
                common,
                outcome,
                mount_snapshot,
                configured_binding_ids,
            },
        };
        self.submit(envelope)
    }

    fn submit(&self, envelope: EvidenceSourceEnvelope) -> ProjectionDisposition {
        self.sink
            .as_ref()
            .map_or(ProjectionDisposition::Disabled, |sink| {
                sink.submit(envelope)
            })
    }
}

pub(crate) struct NativeExecutionFact {
    pub(crate) common: EnvelopeCommon,
    pub(crate) operation_class: OperationClass,
    pub(crate) outcome: TerminalOutcome,
    pub(crate) failure_class: Option<FailureClass>,
    pub(crate) safe_counts: SafeCounts,
}

pub(crate) struct SkillLifecycleFact {
    pub(crate) common: EnvelopeCommon,
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) outcome: SkillLoadOutcome,
    pub(crate) anomaly: Option<SkillLifecycleAnomaly>,
    pub(crate) observation_count: u32,
}

pub(crate) struct DelegatedUtilityFact {
    pub(crate) common: EnvelopeCommon,
    pub(crate) utility_skill_id: String,
    pub(crate) revision: String,
    pub(crate) outcome: UtilityOutcome,
    pub(crate) duration_ms: u64,
    pub(crate) tool_count: u32,
    pub(crate) approval_count: u32,
}

pub(crate) struct RunVerificationFact {
    pub(crate) common: EnvelopeCommon,
    pub(crate) run_id: String,
    pub(crate) verifier: VerificationClass,
    pub(crate) outcome: VerificationOutcome,
    pub(crate) passed_count: u32,
    pub(crate) failed_count: u32,
    pub(crate) predecessor_attempt_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliRuntimeKind {
    Managed,
    Interactive,
}

pub(crate) struct CliLifecycleFact {
    pub(crate) kind: CliRuntimeKind,
    pub(crate) common: EnvelopeCommon,
    pub(crate) outcome: TerminalOutcome,
    pub(crate) failure_class: Option<FailureClass>,
    pub(crate) mount_snapshot: Option<CliMountSnapshot>,
    pub(crate) configured_binding_ids: Vec<String>,
}

#[cfg(test)]
mod tests;
