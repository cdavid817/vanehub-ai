use serde::{Deserialize, Serialize};

pub(crate) const MAX_CORRECTION_NOTE_CHARS: usize = 1_000;
pub(crate) const MAX_OBSERVED_SKILL_REVISIONS: usize = 32;
pub(crate) const MAX_CLI_BINDINGS: usize = 32;

macro_rules! wire_enum {
    ($name:ident { $($variant:ident),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub(crate) enum $name {
            $($variant),+
        }
    };
}

wire_enum!(SourceFamily {
    NativeExecution,
    SkillLoading,
    DelegatedUtility,
    PlanVerification,
    ManagedCli,
    InteractiveCli,
    ExplicitFeedback,
});

wire_enum!(SignalCategory {
    ExplicitFeedback,
    ExecutionFailure,
    VerificationOutcome,
    RetryRecovery,
    DelegationOutcome,
    SkillLifecycleAnomaly,
});

wire_enum!(AttributionStrength {
    Verified,
    Correlated,
    Weak,
    Unattributed,
});

wire_enum!(FeedbackState {
    Helpful,
    Unhelpful,
    Corrected,
});

wire_enum!(SeedReadiness {
    Collecting,
    Ready,
    HumanReviewOnly,
    Ineligible,
});

wire_enum!(SourceFidelity {
    Native,
    Proxied,
    Inferred,
    Opaque,
});

wire_enum!(SkillAssociationKind {
    Injected,
    Loaded,
    Delegated,
    Mounted,
    Configured,
});

wire_enum!(OperationClass {
    Generation,
    Tool,
    Permission,
    Provider,
    Process,
});

wire_enum!(TerminalOutcome {
    Succeeded,
    Failed,
    Cancelled,
    Incomplete,
});

wire_enum!(FailureClass {
    Agent,
    Provider,
    Process,
    Tool,
    Permission,
    Timeout,
    Limit,
    Sandbox,
});

wire_enum!(SkillLoadOutcome {
    Loaded,
    Refused,
    Unavailable,
    Omitted,
});

wire_enum!(SkillLifecycleAnomaly {
    LoadRefusal,
    DependencyUnavailable,
    OverlayConflict,
    PromptBudgetOmission,
});

wire_enum!(UtilityOutcome {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    Limited,
    Refused,
    Incomplete,
});

wire_enum!(VerificationClass {
    Test,
    Build,
    Lint,
    Review,
    Type,
    Security,
    Specification,
    Acceptance,
    Plan,
});

wire_enum!(VerificationOutcome {
    Passed,
    Failed,
    Inconclusive,
    Skipped,
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ObservedSkillRevision {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) association_kind: SkillAssociationKind,
    pub(crate) observed_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MountedSkillRevision {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CliMountSnapshot {
    pub(crate) manifest_hash: String,
    pub(crate) skills: Vec<MountedSkillRevision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SafeCounts {
    pub(crate) attempts: u32,
    pub(crate) failures: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnvelopeCommon {
    pub(crate) source_event_id: String,
    pub(crate) occurred_at: String,
    pub(crate) stable_agent_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) attempt_id: Option<String>,
    pub(crate) workspace: Option<String>,
    pub(crate) fidelity: SourceFidelity,
    pub(crate) observed_skill_revisions: Vec<ObservedSkillRevision>,
}
