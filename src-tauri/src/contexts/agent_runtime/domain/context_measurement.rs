pub(crate) const CONTEXT_SNAPSHOT_VERSION: &str = "onepiece-context-snapshot-v1";
pub(crate) const CONTEXT_ESTIMATOR_VERSION: &str = "onepiece-local-estimator-v1";
pub(crate) const CONTEXT_POLICY_VERSION: &str = "onepiece-context-production-v1";
const LARGE_TOOL_RESULT_CHARACTERS: u64 = 4_096;

// The complete vocabulary is intentionally present before every class is emitted by phase one.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum SemanticClass {
    SystemInstruction,
    ToolSchema,
    UserIntent,
    AssistantResponse,
    ToolRequest,
    ToolResult,
    Attachment,
    Memory,
    Unknown,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum RetentionClass {
    Protected,
    Verbatim,
    Summarizable,
    Microcompactable,
    Reinjectable,
    Discardable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MeasurementQuality {
    Reported,
    ReportedPlusEstimatedDelta,
    Estimated,
    CharactersOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProtocolState {
    Complete,
    Incomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextComponent {
    pub(crate) sequence: usize,
    pub(crate) semantic_class: SemanticClass,
    pub(crate) retention_class: RetentionClass,
    pub(crate) round: Option<usize>,
    pub(crate) characters: u64,
    pub(crate) estimated_tokens: Option<u64>,
    pub(crate) content_fingerprint: String,
    pub(crate) tool_reference: Option<String>,
    pub(crate) current_user_intent: bool,
    pub(crate) correction: bool,
    pub(crate) reinjectable: bool,
    pub(crate) repeated_tool_result: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextRound {
    pub(crate) index: usize,
    pub(crate) protocol_state: ProtocolState,
    pub(crate) component_sequences: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextCapacity {
    pub(crate) context_window_tokens: u64,
    pub(crate) maximum_output_tokens: Option<u64>,
    pub(crate) metadata_revision: String,
    pub(crate) source_identity: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextCompactionDecisionReason {
    BelowThreshold,
    AtOrAboveThreshold,
    InsufficientCapacityMetadata,
    CharactersOnlyMeasurement,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextCompactionDecision {
    pub(crate) should_compact: Option<bool>,
    pub(crate) threshold_tokens: Option<u64>,
    pub(crate) reason: ContextCompactionDecisionReason,
}

impl ContextCompactionDecision {
    pub(crate) fn evaluate(tokens: Option<u64>, capacity: Option<&ContextCapacity>) -> Self {
        let Some(capacity) = capacity else {
            return Self::unknown(ContextCompactionDecisionReason::InsufficientCapacityMetadata);
        };
        let Some(tokens) = tokens else {
            return Self::unknown(ContextCompactionDecisionReason::CharactersOnlyMeasurement);
        };
        let reserve = capacity.maximum_output_tokens.unwrap_or(0).min(20_000);
        let buffer = (capacity.context_window_tokens / 10).min(13_000);
        let threshold = capacity
            .context_window_tokens
            .saturating_sub(reserve)
            .saturating_sub(buffer);
        let should_compact = tokens >= threshold;
        Self {
            should_compact: Some(should_compact),
            threshold_tokens: Some(threshold),
            reason: if should_compact {
                ContextCompactionDecisionReason::AtOrAboveThreshold
            } else {
                ContextCompactionDecisionReason::BelowThreshold
            },
        }
    }

    fn unknown(reason: ContextCompactionDecisionReason) -> Self {
        Self {
            should_compact: None,
            threshold_tokens: None,
            reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextSnapshot {
    pub(crate) version: &'static str,
    pub(crate) estimator_version: &'static str,
    pub(crate) policy_version: &'static str,
    pub(crate) request_fingerprint: String,
    pub(crate) quality: MeasurementQuality,
    pub(crate) characters: u64,
    pub(crate) tokens: Option<u64>,
    pub(crate) components: Vec<ContextComponent>,
    pub(crate) rounds: Vec<ContextRound>,
    pub(crate) capacity: Option<ContextCapacity>,
    pub(crate) reserved_tokens: Option<u64>,
    pub(crate) remaining_tokens: Option<u64>,
    pub(crate) utilization_basis_points: Option<u32>,
    pub(crate) active_character_compaction: bool,
    pub(crate) compaction_decision: ContextCompactionDecision,
    pub(crate) overflow_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UsageAnchor {
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: String,
    pub(crate) request_fingerprint: String,
    pub(crate) input_tokens: u64,
    pub(crate) invocation_sequence: u32,
    pub(crate) component_fingerprints: Vec<String>,
    pub(crate) component_estimates: Vec<Option<u64>>,
}

pub(crate) fn classify_components(components: &mut [ContextComponent], rounds: &[ContextRound]) {
    let last_round = rounds.last().map(|round| round.index);
    for component in components {
        let incomplete = component.round.is_some_and(|index| {
            rounds.iter().any(|round| {
                round.index == index && round.protocol_state == ProtocolState::Incomplete
            })
        });
        component.retention_class = if incomplete
            || matches!(
                component.semantic_class,
                SemanticClass::SystemInstruction
                    | SemanticClass::ToolSchema
                    | SemanticClass::Unknown
            ) {
            RetentionClass::Protected
        } else if component.current_user_intent
            || component.correction
            || component.round == last_round
        {
            RetentionClass::Verbatim
        } else if component.reinjectable {
            RetentionClass::Reinjectable
        } else if component.semantic_class == SemanticClass::ToolResult
            && (component.repeated_tool_result
                || component.characters >= LARGE_TOOL_RESULT_CHARACTERS)
        {
            RetentionClass::Microcompactable
        } else {
            RetentionClass::Summarizable
        };
    }
}
