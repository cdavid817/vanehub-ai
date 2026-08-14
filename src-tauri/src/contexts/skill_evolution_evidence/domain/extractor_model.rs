use serde::{Deserialize, Serialize};

use super::{
    attribute_evidence, AttributionResult, EvidenceSourceEnvelope, SignalCategory, SourceFamily,
    SourceFidelity, EVIDENCE_SANITIZER_V1, EXTRACTOR_VERSION_V1,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ExtractorFamily {
    ExplicitFeedback,
    ExecutionFailure,
    VerificationOutcome,
    RetryRecovery,
    DelegationOutcome,
    SkillLifecycleAnomaly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SignalPolarity {
    Positive,
    Negative,
    Neutral,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SignalSeverity {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SignalSourceLinkKind {
    Session,
    Message,
    Run,
    Attempt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignalSourceLink {
    kind: SignalSourceLinkKind,
    source_id: String,
}

impl SignalSourceLink {
    pub(crate) fn kind(&self) -> SignalSourceLinkKind {
        self.kind
    }

    pub(crate) fn source_id(&self) -> &str {
        &self.source_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SignalDraft {
    extractor: ExtractorFamily,
    extractor_version: u16,
    category: SignalCategory,
    polarity: SignalPolarity,
    severity: SignalSeverity,
    attribution: AttributionResult,
    source_family: SourceFamily,
    source_fidelity: SourceFidelity,
    source_agent_id: Option<String>,
    source_event_id: String,
    occurred_at: String,
    workspace: Option<String>,
    safe_summary: String,
    discriminator: String,
    sanitizer_version: u16,
    source_links: Vec<SignalSourceLink>,
}

impl SignalDraft {
    pub(crate) fn extractor(&self) -> ExtractorFamily {
        self.extractor
    }

    pub(crate) fn category(&self) -> SignalCategory {
        self.category
    }

    pub(crate) fn polarity(&self) -> SignalPolarity {
        self.polarity
    }

    pub(crate) fn severity(&self) -> SignalSeverity {
        self.severity
    }

    pub(crate) fn safe_summary(&self) -> &str {
        &self.safe_summary
    }

    pub(crate) fn extractor_version(&self) -> u16 {
        self.extractor_version
    }

    pub(crate) fn attribution(&self) -> &AttributionResult {
        &self.attribution
    }

    pub(crate) fn source_family(&self) -> SourceFamily {
        self.source_family
    }

    pub(crate) fn source_fidelity(&self) -> SourceFidelity {
        self.source_fidelity
    }

    pub(crate) fn source_agent_id(&self) -> Option<&str> {
        self.source_agent_id.as_deref()
    }

    pub(crate) fn source_event_id(&self) -> &str {
        &self.source_event_id
    }

    pub(crate) fn occurred_at(&self) -> &str {
        &self.occurred_at
    }

    pub(crate) fn workspace(&self) -> Option<&str> {
        self.workspace.as_deref()
    }

    pub(crate) fn discriminator(&self) -> &str {
        &self.discriminator
    }

    pub(crate) fn sanitizer_version(&self) -> u16 {
        self.sanitizer_version
    }

    pub(crate) fn source_links(&self) -> &[SignalSourceLink] {
        &self.source_links
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draft(
    envelope: &EvidenceSourceEnvelope,
    extractor: ExtractorFamily,
    category: SignalCategory,
    polarity: SignalPolarity,
    severity: SignalSeverity,
    safe_summary: String,
    discriminator: String,
) -> SignalDraft {
    let common = envelope.common();
    let source_links = [
        (SignalSourceLinkKind::Session, common.session_id.as_ref()),
        (SignalSourceLinkKind::Message, common.message_id.as_ref()),
        (SignalSourceLinkKind::Run, common.run_id.as_ref()),
        (SignalSourceLinkKind::Attempt, common.attempt_id.as_ref()),
    ]
    .into_iter()
    .filter_map(|(kind, source_id)| {
        source_id.as_ref().map(|source_id| SignalSourceLink {
            kind,
            source_id: (*source_id).clone(),
        })
    })
    .collect();
    SignalDraft {
        extractor,
        extractor_version: EXTRACTOR_VERSION_V1,
        category,
        polarity,
        severity,
        attribution: attribute_evidence(envelope),
        source_family: envelope.source_family(),
        source_fidelity: common.fidelity,
        source_agent_id: common.stable_agent_id.clone(),
        source_event_id: common.source_event_id.clone(),
        occurred_at: common.occurred_at.clone(),
        workspace: common.workspace.clone(),
        safe_summary,
        discriminator,
        sanitizer_version: EVIDENCE_SANITIZER_V1,
        source_links,
    }
}
