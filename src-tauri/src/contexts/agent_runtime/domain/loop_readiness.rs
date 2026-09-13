#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopReadinessCheckCode {
    DefinitionEnabled,
    ProjectAvailable,
    BranchAvailable,
    WorkerEligible,
    VerifierEligible,
    VerificationValid,
    PathScopeValid,
    NoActiveRun,
    /// The definition carries a supported scope version and requested mode.
    ScopeVersionSupported,
    /// Every reachable mutation surface satisfies the requested mode.
    ExecutionCoverage,
}

impl LoopReadinessCheckCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DefinitionEnabled => "definition-enabled",
            Self::ProjectAvailable => "project-available",
            Self::BranchAvailable => "branch-available",
            Self::WorkerEligible => "worker-eligible",
            Self::VerifierEligible => "verifier-eligible",
            Self::VerificationValid => "verification-valid",
            Self::PathScopeValid => "path-scope-valid",
            Self::NoActiveRun => "no-active-run",
            Self::ScopeVersionSupported => "scope-version-supported",
            Self::ExecutionCoverage => "execution-coverage",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopReadinessCategory {
    Definition,
    Workspace,
    Agent,
    Verification,
    Runtime,
}

impl LoopReadinessCategory {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Definition => "definition",
            Self::Workspace => "workspace",
            Self::Agent => "agent",
            Self::Verification => "verification",
            Self::Runtime => "runtime",
        }
    }
}
