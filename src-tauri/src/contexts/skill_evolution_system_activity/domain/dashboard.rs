use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActivityDashboardKind {
    CurrentRuns,
    Candidates,
    Generation,
    Curator,
    Applications,
    Probation,
    Breakers,
}

impl ActivityDashboardKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CurrentRuns => "current_runs",
            Self::Candidates => "candidates",
            Self::Generation => "generation",
            Self::Curator => "curator",
            Self::Applications => "applications",
            Self::Probation => "probation",
            Self::Breakers => "breakers",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DashboardMaterializationV1 {
    pub(crate) schema_version: u16,
    pub(crate) kind: ActivityDashboardKind,
    pub(crate) event_id: String,
    pub(crate) event_code: ActivityEventCode,
    pub(crate) status: ActivityStatus,
    pub(crate) attention: ActivityAttentionKind,
    pub(crate) safe_identities: Vec<SafeIdentity>,
    pub(crate) metrics: BTreeMap<ActivityMetricCode, i64>,
    pub(crate) committed_at_ms: i64,
    pub(crate) source_sequence: u64,
}

impl DashboardMaterializationV1 {
    pub(crate) fn from_envelope(envelope: &EvolutionActivityEnvelopeV1) -> Option<Self> {
        let kind = dashboard_kind_for_event(envelope.event_code)?;
        Some(Self {
            schema_version: ACTIVITY_SCHEMA_VERSION_V1,
            kind,
            event_id: envelope.event_id.clone(),
            event_code: envelope.event_code,
            status: envelope.status,
            attention: envelope.attention_kind,
            safe_identities: envelope.safe_identities.clone(),
            metrics: envelope.metrics.clone(),
            committed_at_ms: envelope.committed_at_ms,
            source_sequence: envelope.source_sequence,
        })
    }
}

pub(crate) const fn dashboard_kind_for_event(
    event: ActivityEventCode,
) -> Option<ActivityDashboardKind> {
    use ActivityDashboardKind as Kind;
    use ActivityEventCode as Event;
    match event {
        Event::RunStarted
        | Event::RunCompleted
        | Event::RunFailed
        | Event::StageStarted
        | Event::StageCompleted
        | Event::StageFailed => Some(Kind::CurrentRuns),
        Event::EvidenceReady
        | Event::SeedReady
        | Event::AssessmentCompleted
        | Event::AssessmentNeedsReview => Some(Kind::Candidates),
        Event::GenerationStarted
        | Event::GenerationCompleted
        | Event::GenerationFailed
        | Event::DossierCompleted
        | Event::SkillCreated => Some(Kind::Generation),
        Event::CuratorQueued
        | Event::CuratorApproved
        | Event::CuratorRejected
        | Event::CuratorDeferred => Some(Kind::Curator),
        Event::OverlayPreviewed
        | Event::OverlayApplied
        | Event::OverlayReverted
        | Event::AutomaticEligible
        | Event::AutomaticApplied
        | Event::AutomaticBlocked => Some(Kind::Applications),
        Event::ProbationStarted | Event::ProbationPassed | Event::ProbationRegressed => {
            Some(Kind::Probation)
        }
        Event::BreakerOpened | Event::BreakerClosed => Some(Kind::Breakers),
        Event::RecoveryCompleted
        | Event::ReconciliationFailed
        | Event::RetentionApplied
        | Event::SourcePurged => None,
    }
}
