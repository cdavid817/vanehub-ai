mod catalog;
mod context_compaction_control;
#[cfg(test)]
mod context_compaction_control_tests;
mod context_compaction_evidence;
mod context_engine;
#[cfg(test)]
mod context_engine_benchmark_tests;
#[cfg(test)]
mod context_engine_tests;
mod context_measurement;
#[cfg(test)]
mod context_measurement_tests;
mod context_optimizer;
#[cfg(test)]
mod context_optimizer_tests;
mod context_quality_assessment;
#[cfg(test)]
mod context_quality_assessment_tests;
mod context_summary;
#[cfg(test)]
mod context_summary_tests;
mod error;
mod expert_role;
mod generation;
mod loop_decision;
mod loop_engineering;
mod loop_progress;
mod memory_document;
#[cfg(test)]
mod memory_document_tests;
mod memory_extraction;
#[cfg(test)]
mod memory_extraction_tests;
mod memory_freshness;
mod memory_selection;
#[cfg(test)]
mod memory_selection_tests;
mod provider;
mod seat_roster;
mod seat_turn;
mod utility_delegation;
mod workflow;

pub(crate) use catalog::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AgentId, AgentOrigin,
    AvailabilityAssessment, AvailabilityProbe, ExecutableStatus, InteractionMode, LaunchMetadata,
    ManagedSdkStatus,
};
pub(crate) use context_compaction_control::{
    select_authoritative_compaction, AutomaticCompactionMode, AutomaticCompactionState,
    CompactionBypassReason, CompactionTriggerSource, AUTOMATIC_COMPACTION_POLICY_VERSION,
};
pub(crate) use context_compaction_evidence::{CompactionPath, ContextCompactionEvidence};
#[allow(unused_imports)]
pub(crate) use context_engine::{
    select_context, CandidateSignals, ContextBudget, ContextCandidate, ContextEvidence,
    ContextEvidenceManifest, ContextEvidenceManifestPage, ContextEvidenceSummary, ContextRange,
    ContextReasonCode, ContextRequest, ContextSelection, ContextSelectionError, ContextSourceKind,
    ContextSourceOutcome, EstimateQuality, CONTEXT_ENGINE_POLICY_VERSION,
};
pub(crate) use context_measurement::{
    classify_components, ContextCapacity, ContextCompactionDecision, ContextComponent,
    ContextRound, ContextSnapshot, MeasurementQuality, ProtocolState, RetentionClass,
    SemanticClass, UsageAnchor, CONTEXT_ESTIMATOR_VERSION, CONTEXT_POLICY_VERSION,
    CONTEXT_SNAPSHOT_VERSION,
};
#[allow(unused_imports)]
pub(crate) use context_optimizer::{
    build_optimization_plan, verify_optimization_candidate, CandidateEvidence,
    ContextOptimizationAction, ContextOptimizationBudget, ContextOptimizationPlan,
    ContextOptimizationVerification, FallbackReason, OptimizationActionKind, OptimizationOutcome,
    OptimizationPlanError, OptimizationTarget, ReductionBasis, SafeFingerprint, SummaryBoundary,
    ToolResultOutcome, ToolResultReplacement, VerificationFailure, CONTEXT_OPTIMIZER_VERSION,
    CONTEXT_VERIFIER_VERSION,
};
#[allow(unused_imports)]
pub(crate) use context_quality_assessment::{
    ContextAssessmentInvariants, ContextAssessmentMeasurementQuality, ContextAssessmentOutcome,
    ContextAssessmentPath, ContextAssessmentReason, ContextAssessmentTriggerSource,
    ContextQualityAssessment, ContextQualityAssessmentInput, ContextQualityAssessmentPage,
    ContextQualityAssessmentRecord, ContextQualitySummary, CONTEXT_QUALITY_ASSESSMENT_VERSION,
    CONTEXT_QUALITY_HISTORY_HARD_LIMIT,
};
#[allow(unused_imports)]
pub(crate) use context_summary::{
    parse_structured_summary, StructuredSummaryEvidence, StructuredSummaryFailure,
    StructuredSummarySectionEvidence, STRUCTURED_SUMMARY_MAX_CHARACTERS, STRUCTURED_SUMMARY_PROMPT,
    STRUCTURED_SUMMARY_VERSION,
};
pub(crate) use error::AgentRuntimeDomainError;
pub(crate) use expert_role::{
    ExpertRole, ExpertRoleInput, ExpertRoleOrigin, ExpertRoleReviewPolicy,
};
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
pub(crate) use memory_document::{
    compose_memory_document, parse_memory_document, validate_name, MemoryDocument, MemoryMetadata,
    MemoryType,
};
#[allow(unused_imports)]
pub(crate) use memory_extraction::{
    parse_memory_actions, MemoryAction, MemoryActionKind, MemoryActionRejection,
    ParsedMemoryActions, MEMORY_ACTIONS_INSTRUCTION,
};
#[allow(unused_imports)]
pub(crate) use memory_freshness::{
    memory_staleness_caveat, render_memory_age, MEMORY_STALENESS_CAVEAT,
};
#[allow(unused_imports)]
pub(crate) use memory_selection::{
    parse_memory_selection, MAX_SELECTED_MEMORIES, MEMORY_SELECTION_INSTRUCTION,
};
pub(crate) use provider::{
    AgentProviderId, ProviderCapabilities, ProviderCapabilityInput, ProviderFamily,
    ProviderMetadata, ProviderReadinessPrerequisites, ProviderSessionRef, ProviderUsageCapability,
};
#[allow(unused_imports)]
pub(crate) use seat_roster::{
    build_seat_briefing, build_seat_context, derive_mentions, normalize_model_family, ModelFamily,
    SeatBriefingEntry, SeatContext, SeatContextMode, SeatTurn,
};
pub(crate) use seat_turn::{
    apply_human_handoff, next_turn_targets, parse_human_handoff, ChainEndReason,
};
pub(crate) use utility_delegation::{
    UtilityDelegationAttempt, UtilityDelegationCounts, UtilityDelegationLimits,
    UtilityDelegationRequest, UtilityDelegationResult, UtilityDelegationSnapshot,
    UtilityDelegationTerminal,
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
        assert_eq!(
            InteractionMode::parse("api").expect("api mode"),
            InteractionMode::Api
        );
        assert_eq!(InteractionMode::Api.as_str(), "api");

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
    fn missing_managed_sdk_does_not_block_cli_session_selection() {
        let agent = agent(
            AvailabilityAssessment::new(
                AgentAvailability::Unavailable,
                Some("Managed SDK dependency 'codex-sdk' is not installed.".to_string()),
            ),
            vec![InteractionMode::Cli, InteractionMode::NativeDesktop],
        );

        agent
            .ensure_session_selectable(InteractionMode::Cli)
            .expect("CLI remains selectable");
        assert!(matches!(
            agent.ensure_session_selectable(InteractionMode::NativeDesktop),
            Err(AgentRuntimeDomainError::AgentUnavailable(_))
        ));
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
