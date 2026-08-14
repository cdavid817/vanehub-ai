use crate::contexts::agent_runtime::domain::{
    ContextCapacity, ContextCompactionDecision, ContextComponent, ContextRound, ContextSnapshot,
    MeasurementQuality, UsageAnchor, CONTEXT_ESTIMATOR_VERSION, CONTEXT_POLICY_VERSION,
    CONTEXT_SNAPSHOT_VERSION,
};

pub(crate) struct ContextAnalysisInput {
    pub(crate) provider_id: Option<String>,
    pub(crate) model_id: String,
    pub(crate) request_fingerprint: String,
    pub(crate) characters: u64,
    pub(crate) components: Vec<ContextComponent>,
    pub(crate) rounds: Vec<ContextRound>,
    pub(crate) token_estimate_complete: bool,
    pub(crate) capacity: Option<ContextCapacity>,
    pub(crate) active_character_compaction: bool,
    pub(crate) invocation_sequence: u32,
    pub(crate) overflow_count: u32,
}

pub(crate) struct ContextAnalysisService;

impl ContextAnalysisService {
    pub(crate) fn analyze(
        input: ContextAnalysisInput,
        anchor: Option<&UsageAnchor>,
    ) -> ContextSnapshot {
        let estimates: Vec<_> = input
            .components
            .iter()
            .map(|component| component.estimated_tokens)
            .collect();
        let estimated_tokens = if input.token_estimate_complete {
            estimates
                .iter()
                .try_fold(0_u64, |total, value| Some(total.saturating_add((*value)?)))
        } else {
            None
        };
        let (tokens, quality) = reconcile(&input, anchor, estimated_tokens);
        let compaction_decision =
            ContextCompactionDecision::evaluate(tokens, input.capacity.as_ref());
        let (reserved_tokens, remaining_tokens, utilization_basis_points) =
            capacity_evidence(tokens, input.capacity.as_ref());
        ContextSnapshot {
            version: CONTEXT_SNAPSHOT_VERSION,
            estimator_version: CONTEXT_ESTIMATOR_VERSION,
            policy_version: CONTEXT_POLICY_VERSION,
            request_fingerprint: input.request_fingerprint,
            quality,
            characters: input.characters,
            tokens,
            components: input.components,
            rounds: input.rounds,
            capacity: input.capacity,
            reserved_tokens,
            remaining_tokens,
            utilization_basis_points,
            active_character_compaction: input.active_character_compaction,
            compaction_decision,
            overflow_count: input.overflow_count,
        }
    }

    pub(crate) fn finalize_anchor(
        snapshot: &ContextSnapshot,
        provider_id: Option<&str>,
        model_id: &str,
        invocation_sequence: u32,
        reported_input_tokens: i64,
    ) -> Option<UsageAnchor> {
        let input_tokens = u64::try_from(reported_input_tokens)
            .ok()
            .filter(|value| *value > 0)?;
        Some(UsageAnchor {
            provider_id: provider_id.map(str::to_string),
            model_id: model_id.to_string(),
            request_fingerprint: snapshot.request_fingerprint.clone(),
            input_tokens,
            invocation_sequence,
            component_fingerprints: snapshot
                .components
                .iter()
                .map(|component| component.content_fingerprint.clone())
                .collect(),
            component_estimates: snapshot
                .components
                .iter()
                .map(|component| component.estimated_tokens)
                .collect(),
        })
    }

    pub(crate) fn finalize_reported_snapshot(
        snapshot: &mut ContextSnapshot,
        reported_input_tokens: i64,
    ) -> bool {
        let Some(input_tokens) = u64::try_from(reported_input_tokens)
            .ok()
            .filter(|value| *value > 0)
        else {
            return false;
        };
        snapshot.tokens = Some(input_tokens);
        snapshot.quality = MeasurementQuality::Reported;
        snapshot.compaction_decision =
            ContextCompactionDecision::evaluate(Some(input_tokens), snapshot.capacity.as_ref());
        let evidence = capacity_evidence(Some(input_tokens), snapshot.capacity.as_ref());
        snapshot.reserved_tokens = evidence.0;
        snapshot.remaining_tokens = evidence.1;
        snapshot.utilization_basis_points = evidence.2;
        true
    }
}

fn capacity_evidence(
    tokens: Option<u64>,
    capacity: Option<&ContextCapacity>,
) -> (Option<u64>, Option<u64>, Option<u32>) {
    let (Some(tokens), Some(capacity)) = (tokens, capacity) else {
        return (None, None, None);
    };
    let summary_reserve = capacity.maximum_output_tokens.unwrap_or(0).min(20_000);
    let safety_buffer = (capacity.context_window_tokens / 10).min(13_000);
    let reserved = summary_reserve.saturating_add(safety_buffer);
    let remaining = capacity
        .context_window_tokens
        .saturating_sub(reserved)
        .saturating_sub(tokens);
    let utilization = if capacity.context_window_tokens == 0 {
        0
    } else {
        tokens
            .min(capacity.context_window_tokens)
            .saturating_mul(10_000)
            .checked_div(capacity.context_window_tokens)
            .unwrap_or(0) as u32
    };
    (Some(reserved), Some(remaining), Some(utilization))
}

fn reconcile(
    input: &ContextAnalysisInput,
    anchor: Option<&UsageAnchor>,
    estimated_tokens: Option<u64>,
) -> (Option<u64>, MeasurementQuality) {
    let Some(anchor) = anchor.filter(|anchor| {
        anchor.provider_id == input.provider_id
            && anchor.model_id == input.model_id
            && anchor.invocation_sequence.checked_add(1) == Some(input.invocation_sequence)
    }) else {
        return estimated(estimated_tokens);
    };
    let fingerprints: Vec<_> = input
        .components
        .iter()
        .map(|component| &component.content_fingerprint)
        .collect();
    if anchor.request_fingerprint == input.request_fingerprint {
        return (Some(anchor.input_tokens), MeasurementQuality::Reported);
    }
    if anchor.component_fingerprints.len() > fingerprints.len()
        || anchor.component_estimates.len() != anchor.component_fingerprints.len()
        || !anchor
            .component_fingerprints
            .iter()
            .zip(&fingerprints)
            .all(|(previous, current)| previous == *current)
    {
        return estimated(estimated_tokens);
    }
    let delta = input.components[anchor.component_fingerprints.len()..]
        .iter()
        .try_fold(0_u64, |total, component| {
            Some(total.saturating_add(component.estimated_tokens?))
        });
    match delta {
        Some(delta) => (
            Some(anchor.input_tokens.saturating_add(delta)),
            MeasurementQuality::ReportedPlusEstimatedDelta,
        ),
        None => estimated(estimated_tokens),
    }
}

fn estimated(tokens: Option<u64>) -> (Option<u64>, MeasurementQuality) {
    (
        tokens,
        if tokens.is_some() {
            MeasurementQuality::Estimated
        } else {
            MeasurementQuality::CharactersOnly
        },
    )
}
