use super::LoopTerminalReason;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum LoopVerifierRecommendation {
    Pass,
    Revise,
    Blocked,
}

impl LoopVerifierRecommendation {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Revise => "revise",
            Self::Blocked => "blocked",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopDecisionInput {
    pub(crate) required_checks_passed: bool,
    pub(crate) verifier_recommendation: LoopVerifierRecommendation,
    pub(crate) user_feedback: Option<String>,
    pub(crate) hard_terminal_reason: Option<LoopTerminalReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopDecisionOutcome {
    NextIteration,
    AwaitingAcceptance,
    Failed(LoopTerminalReason),
    Cancelled(LoopTerminalReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopDecision {
    pub(crate) outcome: LoopDecisionOutcome,
    pub(crate) reason: String,
}

pub(crate) fn decide_loop_iteration(input: &LoopDecisionInput) -> LoopDecision {
    if let Some(reason) = input.hard_terminal_reason {
        let outcome = match reason {
            LoopTerminalReason::GoalMet => LoopDecisionOutcome::AwaitingAcceptance,
            LoopTerminalReason::UserRejected | LoopTerminalReason::UserStopped => {
                LoopDecisionOutcome::Cancelled(reason)
            }
            _ => LoopDecisionOutcome::Failed(reason),
        };
        let detail = if reason == LoopTerminalReason::GoalMet {
            "Goal completion still requires human acceptance.".to_string()
        } else {
            format!("Hard terminal policy selected {}.", reason.as_str())
        };
        return LoopDecision {
            outcome,
            reason: detail,
        };
    }

    if input.verifier_recommendation == LoopVerifierRecommendation::Blocked {
        return LoopDecision {
            outcome: LoopDecisionOutcome::Failed(LoopTerminalReason::VerifierBlocked),
            reason: "Verifier reported a blocking condition.".to_string(),
        };
    }

    if !input.required_checks_passed {
        return revision(
            "One or more required deterministic checks did not pass.",
            input.user_feedback.as_deref(),
        );
    }

    if input.verifier_recommendation == LoopVerifierRecommendation::Revise {
        return revision(
            "Verifier requested another implementation iteration.",
            input.user_feedback.as_deref(),
        );
    }

    LoopDecision {
        outcome: LoopDecisionOutcome::AwaitingAcceptance,
        reason: "Required checks passed and Verifier recommendation is pass; human acceptance is required."
            .to_string(),
    }
}

fn revision(reason: &str, user_feedback: Option<&str>) -> LoopDecision {
    let feedback = user_feedback
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|_| " User continuation feedback will be included in the next Worker context.")
        .unwrap_or_default();
    LoopDecision {
        outcome: LoopDecisionOutcome::NextIteration,
        reason: format!("{reason}{feedback}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(checks: bool, recommendation: LoopVerifierRecommendation) -> LoopDecisionInput {
        LoopDecisionInput {
            required_checks_passed: checks,
            verifier_recommendation: recommendation,
            user_feedback: None,
            hard_terminal_reason: None,
        }
    }

    #[test]
    fn only_passing_checks_and_verifier_advice_reach_human_acceptance() {
        assert_eq!(
            decide_loop_iteration(&input(true, LoopVerifierRecommendation::Pass)).outcome,
            LoopDecisionOutcome::AwaitingAcceptance
        );
        assert_eq!(
            decide_loop_iteration(&input(false, LoopVerifierRecommendation::Pass)).outcome,
            LoopDecisionOutcome::NextIteration
        );
        assert_eq!(
            decide_loop_iteration(&input(true, LoopVerifierRecommendation::Revise)).outcome,
            LoopDecisionOutcome::NextIteration
        );
    }

    #[test]
    fn blocked_and_hard_terminal_outcomes_take_precedence() {
        assert_eq!(
            decide_loop_iteration(&input(true, LoopVerifierRecommendation::Blocked)).outcome,
            LoopDecisionOutcome::Failed(LoopTerminalReason::VerifierBlocked)
        );
        let mut limited = input(true, LoopVerifierRecommendation::Pass);
        limited.hard_terminal_reason = Some(LoopTerminalReason::MaxIterations);
        assert_eq!(
            decide_loop_iteration(&limited).outcome,
            LoopDecisionOutcome::Failed(LoopTerminalReason::MaxIterations)
        );
        limited.hard_terminal_reason = Some(LoopTerminalReason::UserStopped);
        assert_eq!(
            decide_loop_iteration(&limited).outcome,
            LoopDecisionOutcome::Cancelled(LoopTerminalReason::UserStopped)
        );
        limited.hard_terminal_reason = Some(LoopTerminalReason::GoalMet);
        assert_eq!(
            decide_loop_iteration(&limited).outcome,
            LoopDecisionOutcome::AwaitingAcceptance
        );
    }

    #[test]
    fn user_feedback_cannot_override_required_evidence() {
        let mut failed = input(false, LoopVerifierRecommendation::Pass);
        failed.user_feedback = Some("Accept this anyway".to_string());
        let decision = decide_loop_iteration(&failed);
        assert_eq!(decision.outcome, LoopDecisionOutcome::NextIteration);
        assert!(decision.reason.contains("next Worker context"));
    }
}
