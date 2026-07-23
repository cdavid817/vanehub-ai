mod catalog;
mod error;
mod generation;
mod loop_decision;
mod loop_engineering;
mod loop_progress;
mod workflow;

pub(crate) use catalog::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentId, AvailabilityAssessment,
    AvailabilityProbe, ExecutableStatus, InteractionMode, LaunchMetadata, ManagedSdkStatus,
};
pub(crate) use error::AgentRuntimeDomainError;
pub(crate) use generation::GenerationAttempt;
pub(crate) use loop_decision::{
    decide_loop_iteration, LoopDecision, LoopDecisionInput, LoopDecisionOutcome,
    LoopVerifierRecommendation,
};
pub(crate) use loop_engineering::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun, LoopRunPhase, LoopRunSnapshot,
    LoopRunStatus, LoopTerminalReason, LoopVerificationCommand,
};
pub(crate) use loop_progress::{
    assess_revision_progress, fingerprint_objective_state, LoopCheckOutcome,
    LoopObjectiveFingerprints, LoopRequiredCheckObservation, LoopRevisionProgress,
};
pub(crate) use workflow::{AgentLifecycle, AgentReadiness, AgentWorkflow};

#[cfg(test)]
mod tests {
    use super::*;

    fn agent(availability: AvailabilityAssessment, modes: Vec<InteractionMode>) -> AgentDefinition {
        AgentDefinition::new(AgentDefinitionInput {
            id: "codex-cli".to_string(),
            display_name: "Codex CLI".to_string(),
            provider: "OpenAI".to_string(),
            managed_sdk_dependency_id: Some("codex-sdk".to_string()),
            launch: LaunchMetadata::new(
                "cli".to_string(),
                Some("codex".to_string()),
                None,
                Some("codex".to_string()),
            )
            .expect("launch metadata"),
            supported_interaction_modes: modes,
            availability,
            capability_tags: vec!["coding".to_string(), "coding".to_string()],
        })
        .expect("agent")
    }

    #[test]
    fn registry_values_validate_identity_launch_modes_and_capabilities() {
        assert_eq!(
            InteractionMode::parse("native-desktop").expect("mode"),
            InteractionMode::NativeDesktop
        );
        assert!(InteractionMode::parse("terminal").is_err());
        assert!(AgentId::parse(" \n ").is_err());

        let agent = agent(
            AvailabilityAssessment::new(AgentAvailability::Available, None),
            vec![InteractionMode::Cli, InteractionMode::Cli],
        );
        assert_eq!(agent.id().as_str(), "codex-cli");
        assert_eq!(agent.launch().kind(), &catalog::LaunchKind::Cli);
        assert_eq!(agent.launch().command(), Some("codex"));
        assert_eq!(agent.supported_interaction_modes(), &[InteractionMode::Cli]);
        assert_eq!(agent.capability_tags(), &["coding".to_string()]);
        assert!(agent.has_capability("coding"));
    }

    #[test]
    fn availability_assessment_preserves_dependency_and_executable_reasons() {
        let missing_sdk = AvailabilityAssessment::assess(AvailabilityProbe {
            managed_sdk: ManagedSdkStatus::Missing("codex-sdk".to_string()),
            executable: ExecutableStatus::Available,
        });
        assert_eq!(missing_sdk.state(), AgentAvailability::Unavailable);
        assert_eq!(
            missing_sdk.reason(),
            Some("Managed SDK dependency 'codex-sdk' is not installed.")
        );

        let missing_command = AvailabilityAssessment::assess(AvailabilityProbe {
            managed_sdk: ManagedSdkStatus::NotRequired,
            executable: ExecutableStatus::Missing("opencode".to_string()),
        });
        assert_eq!(
            missing_command.reason(),
            Some("Command 'opencode' was not found on PATH.")
        );
        assert_eq!(
            AvailabilityAssessment::assess(AvailabilityProbe {
                managed_sdk: ManagedSdkStatus::NotRequired,
                executable: ExecutableStatus::NotDeclared,
            })
            .state(),
            AgentAvailability::Unknown
        );

        let unknown_sdk = AvailabilityAssessment::assess(AvailabilityProbe {
            managed_sdk: ManagedSdkStatus::Unrecognized("other-sdk".to_string()),
            executable: ExecutableStatus::Available,
        });
        assert_eq!(unknown_sdk.state(), AgentAvailability::Unavailable);
        assert_eq!(
            unknown_sdk.reason(),
            Some("Managed SDK dependency 'other-sdk' is not recognized.")
        );
    }

    #[test]
    fn workflow_selection_readiness_and_lifecycle_are_domain_controlled() {
        let available = agent(
            AvailabilityAssessment::new(AgentAvailability::Available, None),
            vec![InteractionMode::Cli, InteractionMode::Browser],
        );
        let mut workflow = AgentWorkflow::new("build");
        workflow
            .select(&available, InteractionMode::Cli)
            .expect("select");
        assert_eq!(
            workflow.active_agent_id().map(AgentId::as_str),
            Some("codex-cli")
        );
        assert_eq!(workflow.intent(), "build");
        workflow.begin_launch().expect("starting");
        workflow.mark_running().expect("running");
        workflow.mark_failed().expect("failed");
        assert!(workflow.mark_running().is_err());
        workflow.begin_launch().expect("restart");
        workflow.mark_stopped().expect("stopped");

        let readiness = AgentReadiness::for_browser(&available);
        assert!(readiness.is_ready());
        assert!(readiness.requires_authentication());
        assert_eq!(readiness.reason(), None);

        let unavailable = agent(
            AvailabilityAssessment::new(
                AgentAvailability::Unavailable,
                Some("missing".to_string()),
            ),
            vec![InteractionMode::Cli],
        );
        assert!(AgentWorkflow::new("build")
            .select(&unavailable, InteractionMode::Cli)
            .is_err());
        assert!(AgentWorkflow::new("build")
            .select(&available, InteractionMode::NativeDesktop)
            .is_err());

        let needs_authentication = agent(
            AvailabilityAssessment::new(AgentAvailability::NeedsAuthentication, None),
            vec![InteractionMode::Cli],
        );
        assert!(matches!(
            AgentWorkflow::new("build").select(&needs_authentication, InteractionMode::Cli),
            Err(AgentRuntimeDomainError::AgentUnavailable(_))
        ));

        assert!(matches!(
            AgentWorkflow::rehydrate(
                Some("codex-cli".to_string()),
                None,
                AgentLifecycle::Idle,
                "build".to_string(),
            ),
            Err(AgentRuntimeDomainError::IncompleteWorkflowSelection)
        ));
    }

    #[test]
    fn generation_transitions_require_attachment_and_are_terminal_once() {
        let mut generation = GenerationAttempt::reserve("session-1").expect("reserve");
        assert_eq!(generation.state(), generation::GenerationState::Reserved);
        assert!(generation.complete().is_err());
        generation
            .attach("message-1", true)
            .expect("attach process");
        assert_eq!(generation.message_id(), Some("message-1"));
        generation.complete().expect("complete");
        assert_eq!(generation.state(), generation::GenerationState::Completed);
        assert!(generation.fail().is_err());
        assert!(generation.cancel().is_err());

        let mut cancelled = GenerationAttempt::reserve("session-2").expect("reserve");
        let outcome = cancelled.cancel().expect("cancel reserved");
        assert_eq!(outcome.message_id, None);
        assert!(!outcome.process_attached);
        assert_eq!(cancelled.state(), generation::GenerationState::Cancelled);
    }
}
