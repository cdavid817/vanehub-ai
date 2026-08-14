use serde_json::{json, Value};

use super::context_projection::{fingerprint, project_request, ContextWireShape};
use super::context_reduction::{
    apply_low_cost_reductions, build_structured_summary_turns, reconstruct_candidate,
};
use crate::contexts::agent_runtime::application::{
    ContextReinjectionEvidence, ContextReinjectionKind, ReinjectedContextValue,
};
use crate::contexts::agent_runtime::domain::{
    ContextOptimizationAction, ContextOptimizationBudget, ContextOptimizationPlan,
    OptimizationActionKind, OptimizationOutcome, OptimizationTarget, SafeFingerprint,
    CONTEXT_OPTIMIZER_VERSION,
};

fn plan(fingerprint_value: &str, sequence: usize) -> ContextOptimizationPlan {
    ContextOptimizationPlan {
        version: CONTEXT_OPTIMIZER_VERSION,
        budget: ContextOptimizationBudget {
            original_characters: 10_000,
            original_tokens: Some(2_500),
            target_characters: 5_000,
            target_tokens: Some(1_250),
        },
        actions: vec![ContextOptimizationAction {
            kind: OptimizationActionKind::MicrocompactToolResult,
            target: OptimizationTarget::Component(sequence),
            source_fingerprints: vec![SafeFingerprint::parse(fingerprint_value).expect("safe")],
            reclaimed_characters: 5_000,
            reclaimed_tokens: Some(1_250),
        }],
        summary_boundary: None,
        projected_characters: 5_000,
        projected_tokens: Some(1_250),
        outcome: OptimizationOutcome::Planned,
    }
}

#[test]
fn anthropic_microcompaction_preserves_protocol_error_and_order() {
    let original_result = json!({
        "type": "tool_result",
        "tool_use_id": "call-1",
        "is_error": true,
        "content": "sensitive raw output".repeat(500),
    });
    let body = json!({
        "messages": [
            {"role": "assistant", "content": [
                {"type": "tool_use", "id": "call-1", "name": "read", "input": {}}
            ]},
            {"role": "user", "content": [original_result.clone()]},
            {"role": "assistant", "content": [{"type": "text", "text": "after"}]}
        ]
    });
    let candidate = apply_low_cost_reductions(
        &body,
        ContextWireShape::Anthropic,
        &plan(&fingerprint(&original_result), 2),
    )
    .expect("candidate");
    let messages = candidate["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 3);
    let result = &messages[1]["content"][0];
    assert_eq!(result["type"], "tool_result");
    assert_eq!(result["tool_use_id"], "call-1");
    assert_eq!(result["is_error"], true);
    assert!(result["content"]
        .as_str()
        .expect("marker")
        .contains("OnePiece compacted"));
    assert!(!candidate.to_string().contains("sensitive raw output"));
}

#[test]
fn openai_microcompaction_preserves_tool_call_id_status_and_order() {
    let original_result = json!({
        "role": "tool",
        "tool_call_id": "call-2",
        "status": "error",
        "content": "private tool data".repeat(500),
    });
    let body = json!({
        "messages": [
            {"role": "assistant", "tool_calls": [
                {"id": "call-2", "type": "function", "function": {"name": "read", "arguments": "{}"}}
            ]},
            original_result.clone(),
            {"role": "assistant", "content": "after"}
        ]
    });
    let candidate = apply_low_cost_reductions(
        &body,
        ContextWireShape::OpenAiCompatible,
        &plan(&fingerprint(&original_result), 1),
    )
    .expect("candidate");
    let messages = candidate["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 3);
    assert_eq!(messages[1]["role"], "tool");
    assert_eq!(messages[1]["tool_call_id"], "call-2");
    assert_eq!(messages[1]["status"], "error");
    assert!(messages[1]["content"]
        .as_str()
        .expect("marker")
        .contains("outcome=failed"));
    assert!(!candidate.to_string().contains("private tool data"));
}

#[test]
fn transient_removal_requires_an_explicit_matching_action() {
    let transient = json!({"role": "assistant", "content": "transient"});
    let unknown = json!({"role": "custom", "content": "preserve unknown"});
    let body = json!({"messages": [transient.clone(), unknown.clone()]});
    let mut reduction_plan = plan(&fingerprint(&transient), 0);
    reduction_plan.actions[0].kind = OptimizationActionKind::DiscardTransient;
    let candidate =
        apply_low_cost_reductions(&body, ContextWireShape::OpenAiCompatible, &reduction_plan)
            .expect("candidate");
    assert_eq!(candidate["messages"], Value::Array(vec![unknown]));
}

#[test]
fn structured_summary_selects_only_boundary_rounds_and_strips_thinking() {
    let body = json!({
        "system": "do not inherit",
        "tools": [{"name": "read"}],
        "thinking": {"type": "adaptive"},
        "messages": [
            {"role": "user", "content": "first"},
            {"role": "assistant", "content": [
                {"type": "thinking", "thinking": "hidden"},
                {"type": "text", "text": "answer"},
                {"type": "tool_use", "id": "call-1", "name": "read", "input": {}}
            ]},
            {"role": "user", "content": [
                {"type": "tool_result", "tool_use_id": "call-1", "content": "result"}
            ]},
            {"role": "assistant", "content": [{"type": "text", "text": "done"}]},
            {"role": "user", "content": "recent intent"}
        ]
    });
    let turns = build_structured_summary_turns(
        &body,
        ContextWireShape::Anthropic,
        &crate::contexts::agent_runtime::domain::SummaryBoundary {
            first_round: 0,
            last_round: 0,
            round_count: 1,
        },
    )
    .expect("turns");
    assert_eq!(turns.len(), 3);
    assert!(turns
        .iter()
        .all(|turn| !turn.to_string().contains("hidden")));
    assert!(!turns
        .iter()
        .any(|turn| turn.to_string().contains("recent intent")));
}

#[test]
fn structured_summary_request_inherits_no_tools_system_or_generation_options() {
    let source = json!({
        "messages": [
            {"role": "user", "content": "selected"},
            {"role": "assistant", "content": "selected answer", "reasoning_content": "hidden"},
            {"role": "user", "content": "not selected"}
        ]
    });
    let turns = build_structured_summary_turns(
        &source,
        ContextWireShape::OpenAiCompatible,
        &crate::contexts::agent_runtime::domain::SummaryBoundary {
            first_round: 0,
            last_round: 0,
            round_count: 1,
        },
    )
    .expect("turns");
    let body = super::openai_compatible_provider::build_request_body(
        "model",
        &turns,
        &[],
        None,
        &super::api_process_adapter::GenerationOptions::disabled(),
    );
    assert!(body.get("tools").is_none());
    assert!(body.get("reasoning_effort").is_none());
    assert_eq!(body["messages"].as_array().expect("messages").len(), 2);
    assert!(!body.to_string().contains("hidden"));
    assert!(!body.to_string().contains("not selected"));
}

#[test]
fn structured_summary_rejects_non_prefix_or_unavailable_boundaries() {
    let body = json!({"messages": [{"role": "user", "content": "one"}]});
    for boundary in [
        crate::contexts::agent_runtime::domain::SummaryBoundary {
            first_round: 1,
            last_round: 1,
            round_count: 1,
        },
        crate::contexts::agent_runtime::domain::SummaryBoundary {
            first_round: 0,
            last_round: 1,
            round_count: 2,
        },
    ] {
        assert!(build_structured_summary_turns(
            &body,
            ContextWireShape::OpenAiCompatible,
            &boundary,
        )
        .is_err());
    }
}

fn valid_summary() -> String {
    [
        ("PRIMARY INTENT", "Continue safely."),
        ("TECHNICAL CONSTRAINTS", "Preserve protocol."),
        ("DECISIONS", "Use neutral actions."),
        ("FILES AND CODE AREAS", "context_reduction.rs"),
        ("ERRORS AND FIXES", "None."),
        ("COMPLETED WORK", "Old round complete."),
        ("PENDING WORK", "Continue runtime work."),
        ("IMMEDIATE NEXT ACTION", "Verify candidate."),
    ]
    .into_iter()
    .map(|(heading, content)| format!("## {heading}\n{content}"))
    .collect::<Vec<_>>()
    .join("\n")
}

fn summary_plan(source_fingerprints: Vec<SafeFingerprint>) -> ContextOptimizationPlan {
    ContextOptimizationPlan {
        version: CONTEXT_OPTIMIZER_VERSION,
        budget: ContextOptimizationBudget {
            original_characters: 2_000,
            original_tokens: Some(500),
            target_characters: 1_000,
            target_tokens: Some(250),
        },
        actions: vec![ContextOptimizationAction {
            kind: OptimizationActionKind::SummarizeRound,
            target: OptimizationTarget::Round(0),
            source_fingerprints,
            reclaimed_characters: 1_000,
            reclaimed_tokens: Some(250),
        }],
        summary_boundary: Some(crate::contexts::agent_runtime::domain::SummaryBoundary {
            first_round: 0,
            last_round: 0,
            round_count: 1,
        }),
        projected_characters: 1_000,
        projected_tokens: Some(250),
        outcome: OptimizationOutcome::Planned,
    }
}

fn reinjection() -> ReinjectedContextValue {
    ReinjectedContextValue {
        kind: ContextReinjectionKind::Memory,
        content: "current memory".to_string(),
        evidence: ContextReinjectionEvidence {
            kind: "memory",
            revision: "memory-r2".to_string(),
            source_fingerprint: "0123456789abcdef01234567".to_string(),
            characters: 14,
        },
    }
}

fn round_fingerprints(original: &Value, shape: ContextWireShape) -> Vec<SafeFingerprint> {
    let projection = project_request(original, shape);
    projection.rounds[0]
        .component_sequences
        .iter()
        .map(|sequence| {
            SafeFingerprint::parse(&projection.components[*sequence].content_fingerprint).unwrap()
        })
        .collect()
}

#[test]
fn reconstructs_anthropic_summary_reinjection_and_preserved_messages() {
    let original = json!({
        "model": "model",
        "system": "protected system",
        "messages": [
            {"role": "user", "content": "old request"},
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "current request"}
        ]
    });
    let original_copy = original.clone();
    let candidate = reconstruct_candidate(
        &original,
        ContextWireShape::Anthropic,
        &summary_plan(round_fingerprints(&original, ContextWireShape::Anthropic)),
        Some(&valid_summary()),
        &[reinjection()],
    )
    .expect("candidate");
    assert_eq!(original, original_copy);
    assert_eq!(candidate["system"], "protected system");
    assert!(candidate.to_string().contains("current memory"));
    assert!(candidate.to_string().contains("current request"));
    assert!(!candidate.to_string().contains("old request"));
    let projected = project_request(&candidate, ContextWireShape::Anthropic);
    assert!(projected.rounds.iter().all(|round| {
        round.protocol_state == crate::contexts::agent_runtime::domain::ProtocolState::Complete
    }));
    assert!(
        projected
            .components
            .iter()
            .map(|value| value.characters)
            .sum::<u64>()
            >= projected.characters
    );
}

#[test]
fn reconstructs_equivalent_openai_candidate_semantics() {
    let original = json!({
        "model": "model",
        "messages": [
            {"role": "system", "content": "protected system"},
            {"role": "user", "content": "old request"},
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "current request"}
        ]
    });
    let candidate = reconstruct_candidate(
        &original,
        ContextWireShape::OpenAiCompatible,
        &summary_plan(round_fingerprints(
            &original,
            ContextWireShape::OpenAiCompatible,
        )),
        Some(&valid_summary()),
        &[reinjection()],
    )
    .expect("candidate");
    assert_eq!(candidate["messages"][0]["role"], "system");
    assert_eq!(candidate["messages"][0]["content"], "protected system");
    assert!(candidate.to_string().contains("current memory"));
    assert!(candidate.to_string().contains("current request"));
    assert!(!candidate.to_string().contains("old request"));
    let projected = project_request(&candidate, ContextWireShape::OpenAiCompatible);
    assert!(projected.rounds.iter().all(|round| {
        round.protocol_state == crate::contexts::agent_runtime::domain::ProtocolState::Complete
    }));
    assert!(
        projected
            .components
            .iter()
            .map(|value| value.characters)
            .sum::<u64>()
            >= projected.characters
    );
}

#[test]
fn equivalent_wire_plans_have_matching_action_and_protocol_evidence() {
    let anthropic = json!({
        "system": "protected",
        "messages": [
            {"role": "user", "content": "old request"},
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "current request"}
        ]
    });
    let openai = json!({
        "messages": [
            {"role": "system", "content": "protected"},
            {"role": "user", "content": "old request"},
            {"role": "assistant", "content": "old answer"},
            {"role": "user", "content": "current request"}
        ]
    });
    let anthropic_plan = summary_plan(round_fingerprints(&anthropic, ContextWireShape::Anthropic));
    let openai_plan = summary_plan(round_fingerprints(
        &openai,
        ContextWireShape::OpenAiCompatible,
    ));
    assert_eq!(
        anthropic_plan
            .actions
            .iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>(),
        openai_plan
            .actions
            .iter()
            .map(|action| action.kind)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        anthropic_plan.summary_boundary,
        openai_plan.summary_boundary
    );
    assert_eq!(anthropic_plan.outcome, openai_plan.outcome);

    let anthropic_candidate = reconstruct_candidate(
        &anthropic,
        ContextWireShape::Anthropic,
        &anthropic_plan,
        Some(&valid_summary()),
        &[],
    )
    .unwrap();
    let openai_candidate = reconstruct_candidate(
        &openai,
        ContextWireShape::OpenAiCompatible,
        &openai_plan,
        Some(&valid_summary()),
        &[],
    )
    .unwrap();
    let anthropic_projection = project_request(&anthropic_candidate, ContextWireShape::Anthropic);
    let openai_projection = project_request(&openai_candidate, ContextWireShape::OpenAiCompatible);
    assert_eq!(
        anthropic_projection
            .rounds
            .iter()
            .map(|round| round.protocol_state)
            .collect::<Vec<_>>(),
        openai_projection
            .rounds
            .iter()
            .map(|round| round.protocol_state)
            .collect::<Vec<_>>()
    );
    assert_eq!(
        anthropic_projection
            .components
            .iter()
            .map(|component| component.semantic_class)
            .collect::<Vec<_>>(),
        openai_projection
            .components
            .iter()
            .map(|component| component.semantic_class)
            .collect::<Vec<_>>()
    );
}
