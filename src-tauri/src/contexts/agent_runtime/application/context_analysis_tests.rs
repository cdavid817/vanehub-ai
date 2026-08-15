use super::{ContextAnalysisInput, ContextAnalysisService};
use crate::contexts::agent_runtime::domain::{
    ContextCapacity, ContextComponent, MeasurementQuality, RetentionClass, SemanticClass,
};

fn component(hash: &str, tokens: Option<u64>) -> ContextComponent {
    ContextComponent {
        sequence: 0,
        semantic_class: SemanticClass::UserIntent,
        retention_class: RetentionClass::Verbatim,
        round: Some(0),
        characters: 10,
        estimated_tokens: tokens,
        content_fingerprint: hash.to_string(),
        tool_reference: None,
        current_user_intent: true,
        correction: false,
        reinjectable: false,
        repeated_tool_result: false,
    }
}

fn input(
    sequence: u32,
    fingerprint: &str,
    components: Vec<ContextComponent>,
) -> ContextAnalysisInput {
    ContextAnalysisInput {
        provider_id: Some("openai".to_string()),
        model_id: "gpt-5.4".to_string(),
        request_fingerprint: fingerprint.to_string(),
        characters: 10,
        components,
        rounds: vec![],
        token_estimate_complete: true,
        capacity: None,
        active_character_compaction: false,
        invocation_sequence: sequence,
        overflow_count: 0,
    }
}

#[test]
fn reported_anchor_reconciles_identical_retry_and_append_only_successor() {
    let initial =
        ContextAnalysisService::analyze(input(0, "request-a", vec![component("a", Some(4))]), None);
    let anchor =
        ContextAnalysisService::finalize_anchor(&initial, Some("openai"), "gpt-5.4", 0, 100)
            .expect("valid usage");
    let identical = ContextAnalysisService::analyze(
        input(1, "request-a", vec![component("a", Some(4))]),
        Some(&anchor),
    );
    assert_eq!(identical.quality, MeasurementQuality::Reported);
    assert_eq!(identical.tokens, Some(100));

    let appended = ContextAnalysisService::analyze(
        input(
            1,
            "request-b",
            vec![component("a", Some(4)), component("tool", Some(7))],
        ),
        Some(&anchor),
    );
    assert_eq!(
        appended.quality,
        MeasurementQuality::ReportedPlusEstimatedDelta
    );
    assert_eq!(appended.tokens, Some(107));
}

#[test]
fn anchors_fail_closed_for_changed_identity_order_usage_and_sequence() {
    let initial =
        ContextAnalysisService::analyze(input(2, "a", vec![component("one", Some(2))]), None);
    assert!(
        ContextAnalysisService::finalize_anchor(&initial, Some("openai"), "gpt-5.4", 2, 0)
            .is_none()
    );
    assert!(
        ContextAnalysisService::finalize_anchor(&initial, Some("openai"), "gpt-5.4", 2, -1)
            .is_none()
    );
    let anchor =
        ContextAnalysisService::finalize_anchor(&initial, Some("openai"), "gpt-5.4", 2, 50)
            .expect("anchor");
    for mut changed in [
        input(4, "b", vec![component("one", Some(2))]),
        input(3, "b", vec![component("different", Some(2))]),
        input(3, "b", vec![]),
    ] {
        changed.model_id = if changed.request_fingerprint == "b" && changed.components.len() == 1 {
            "other".to_string()
        } else {
            changed.model_id
        };
        assert_eq!(
            ContextAnalysisService::analyze(changed, Some(&anchor)).quality,
            MeasurementQuality::Estimated
        );
    }
}

#[test]
fn unsupported_estimation_degrades_to_characters_only() {
    let mut value = input(0, "unknown", vec![component("x", None)]);
    value.token_estimate_complete = false;
    let snapshot = ContextAnalysisService::analyze(value, None);
    assert_eq!(snapshot.quality, MeasurementQuality::CharactersOnly);
    assert_eq!(snapshot.tokens, None);
    assert_eq!(snapshot.remaining_tokens, None);
    assert_eq!(snapshot.utilization_basis_points, None);
}

#[test]
fn known_capacity_exposes_reserve_remaining_and_utilization_evidence() {
    let mut value = input(0, "known", vec![component("x", Some(25_000))]);
    value.capacity = Some(ContextCapacity {
        context_window_tokens: 100_000,
        maximum_output_tokens: Some(20_000),
        metadata_revision: "r1".to_string(),
        source_identity: "official".to_string(),
    });
    let mut snapshot = ContextAnalysisService::analyze(value, None);
    assert_eq!(snapshot.reserved_tokens, Some(30_000));
    assert_eq!(snapshot.remaining_tokens, Some(45_000));
    assert_eq!(snapshot.utilization_basis_points, Some(2_500));

    assert!(ContextAnalysisService::finalize_reported_snapshot(
        &mut snapshot,
        50_000
    ));
    assert_eq!(snapshot.remaining_tokens, Some(20_000));
    assert_eq!(snapshot.utilization_basis_points, Some(5_000));
}
