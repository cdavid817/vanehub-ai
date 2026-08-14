use super::context_measurement::ContextCompactionDecisionReason;
use super::*;

fn component(class: SemanticClass, round: Option<usize>) -> ContextComponent {
    ContextComponent {
        sequence: 0,
        semantic_class: class,
        retention_class: RetentionClass::Discardable,
        round,
        characters: 1,
        estimated_tokens: Some(1),
        content_fingerprint: "hash".to_string(),
        tool_reference: None,
        current_user_intent: false,
        correction: false,
        reinjectable: false,
        repeated_tool_result: false,
    }
}

#[test]
fn ordered_classification_protects_control_and_incomplete_protocol() {
    let rounds = vec![
        ContextRound {
            index: 0,
            protocol_state: ProtocolState::Incomplete,
            component_sequences: vec![0],
        },
        ContextRound {
            index: 1,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![1],
        },
    ];
    let mut components = vec![
        component(SemanticClass::ToolResult, Some(0)),
        component(SemanticClass::Unknown, Some(1)),
        component(SemanticClass::SystemInstruction, None),
    ];
    classify_components(&mut components, &rounds);
    assert!(components
        .iter()
        .all(|component| component.retention_class == RetentionClass::Protected));
}

#[test]
fn classification_keeps_current_and_recent_content_verbatim() {
    let rounds = vec![
        ContextRound {
            index: 0,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![0],
        },
        ContextRound {
            index: 1,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![1],
        },
    ];
    let mut older = component(SemanticClass::AssistantResponse, Some(0));
    let mut current = component(SemanticClass::UserIntent, Some(1));
    current.current_user_intent = true;
    let mut correction = component(SemanticClass::UserIntent, Some(0));
    correction.correction = true;
    let mut components = vec![older.clone(), current, correction];
    classify_components(&mut components, &rounds);
    assert_eq!(components[0].retention_class, RetentionClass::Summarizable);
    assert_eq!(components[1].retention_class, RetentionClass::Verbatim);
    assert_eq!(components[2].retention_class, RetentionClass::Verbatim);
    older.reinjectable = true;
    classify_components(std::slice::from_mut(&mut older), &rounds);
    assert_eq!(older.retention_class, RetentionClass::Reinjectable);
}

#[test]
fn older_repeated_tool_results_are_microcompactable() {
    let rounds = vec![
        ContextRound {
            index: 0,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![0],
        },
        ContextRound {
            index: 1,
            protocol_state: ProtocolState::Complete,
            component_sequences: vec![],
        },
    ];
    let mut value = component(SemanticClass::ToolResult, Some(0));
    value.repeated_tool_result = true;
    classify_components(std::slice::from_mut(&mut value), &rounds);
    assert_eq!(value.retention_class, RetentionClass::Microcompactable);

    let mut large = component(SemanticClass::ToolResult, Some(0));
    large.characters = 4_096;
    classify_components(std::slice::from_mut(&mut large), &rounds);
    assert_eq!(large.retention_class, RetentionClass::Microcompactable);
}

#[test]
fn production_policy_handles_boundaries_unknowns_and_saturating_reserves() {
    let capacity = ContextCapacity {
        context_window_tokens: 100_000,
        maximum_output_tokens: Some(30_000),
        metadata_revision: "r1".to_string(),
        source_identity: "official".to_string(),
    };
    assert_eq!(
        ContextCompactionDecision::evaluate(Some(69_999), Some(&capacity)).reason,
        ContextCompactionDecisionReason::BelowThreshold
    );
    assert_eq!(
        ContextCompactionDecision::evaluate(Some(70_000), Some(&capacity)).reason,
        ContextCompactionDecisionReason::AtOrAboveThreshold
    );
    assert_eq!(
        ContextCompactionDecision::evaluate(None, Some(&capacity)).reason,
        ContextCompactionDecisionReason::CharactersOnlyMeasurement
    );
    assert_eq!(
        ContextCompactionDecision::evaluate(Some(1), None).reason,
        ContextCompactionDecisionReason::InsufficientCapacityMetadata
    );
    let tiny = ContextCapacity {
        context_window_tokens: 1,
        maximum_output_tokens: Some(u64::MAX),
        ..capacity
    };
    assert_eq!(
        ContextCompactionDecision::evaluate(Some(0), Some(&tiny)).threshold_tokens,
        Some(0)
    );
}
