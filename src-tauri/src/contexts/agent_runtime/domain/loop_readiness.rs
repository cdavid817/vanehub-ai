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
