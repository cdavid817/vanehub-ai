use super::context_compaction_control::{
    AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS, AUTOMATIC_COMPACTION_FAILURE_LIMIT,
};
use super::context_measurement::ContextCompactionDecisionReason;
use super::*;

fn decision(value: Option<bool>) -> ContextCompactionDecision {
    ContextCompactionDecision {
        should_compact: value,
        threshold_tokens: value.map(|_| 70_000),
        reason: match value {
            Some(true) => ContextCompactionDecisionReason::AtOrAboveThreshold,
            Some(false) => ContextCompactionDecisionReason::BelowThreshold,
            None => ContextCompactionDecisionReason::InsufficientCapacityMetadata,
        },
    }
}

#[test]
fn token_decision_is_authoritative_for_true_and_false() {
    let active = select_authoritative_compaction(&decision(Some(true)), false);
    assert!(active.should_compact);
    assert_eq!(active.source, CompactionTriggerSource::TokenAware);
    assert!(!active.character_decision);

    let inactive = select_authoritative_compaction(&decision(Some(false)), true);
    assert!(!inactive.should_compact);
    assert_eq!(inactive.source, CompactionTriggerSource::TokenAware);
    assert!(inactive.character_decision);
}

#[test]
fn unavailable_token_decision_uses_character_fallback() {
    let active = select_authoritative_compaction(&decision(None), true);
    assert!(active.should_compact);
    assert_eq!(active.source, CompactionTriggerSource::CharacterFallback);
}

#[test]
fn suppression_cooldown_and_boundary_are_deterministic() {
    let mut state = AutomaticCompactionState::default();
    assert_eq!(
        state.bypass_reason(AutomaticCompactionMode::Suppressed, 100_000),
        Some(CompactionBypassReason::RequestSuppressed)
    );
    assert_eq!(
        state.bypass_reason(AutomaticCompactionMode::Automatic, 100_000),
        None
    );

    state.record_success(10_000);
    assert_eq!(
        state.bypass_reason(
            AutomaticCompactionMode::Automatic,
            10_000 + AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS - 1,
        ),
        Some(CompactionBypassReason::Cooldown)
    );
    assert_eq!(
        state.bypass_reason(
            AutomaticCompactionMode::Automatic,
            10_000 + AUTOMATIC_COMPACTION_COOLDOWN_CHARACTERS,
        ),
        None
    );
}

#[test]
fn disabled_user_preference_is_captured_by_generation_state() {
    let state = AutomaticCompactionState::with_user_preference(false);

    assert_eq!(
        state.bypass_reason(AutomaticCompactionMode::Automatic, 100_000),
        Some(CompactionBypassReason::UserPreferenceSuppressed)
    );
    assert!(AutomaticCompactionState::default()
        .bypass_reason(AutomaticCompactionMode::Automatic, 100_000)
        .is_none());
}

#[test]
fn failures_open_circuit_and_success_resets_it() {
    let mut state = AutomaticCompactionState::default();
    state.record_failure();
    assert_eq!(state.consecutive_failures(), 1);
    assert!(!state.circuit_open());
    state.record_failure();
    assert!(state.circuit_open());
    assert_eq!(
        state.consecutive_failures(),
        AUTOMATIC_COMPACTION_FAILURE_LIMIT
    );
    assert_eq!(
        state.bypass_reason(AutomaticCompactionMode::Automatic, u64::MAX),
        Some(CompactionBypassReason::CircuitOpen)
    );

    state.record_success(20_000);
    assert_eq!(state.consecutive_failures(), 0);
    assert!(!state.circuit_open());
    assert_eq!(
        AutomaticCompactionState::default().consecutive_failures(),
        0
    );
}
