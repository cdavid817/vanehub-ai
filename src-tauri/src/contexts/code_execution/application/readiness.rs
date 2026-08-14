#![allow(clippy::enum_variant_names)]

use super::{SandboxBackendCapabilities, SandboxProcessBackend};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeSandboxReadinessReason {
    RestrictedIdentityUnavailable,
    ResourceLimitsUnavailable,
    ProcessTreeContainmentUnavailable,
    AclConfinementUnavailable,
    NetworkIsolationUnavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeSandboxReadiness {
    pub(crate) ready: bool,
    pub(crate) reasons: Vec<CodeSandboxReadinessReason>,
}

impl CodeSandboxReadiness {
    pub(crate) fn probe(backend: &dyn SandboxProcessBackend) -> Self {
        Self::from_capabilities(backend.capabilities())
    }

    fn from_capabilities(capabilities: SandboxBackendCapabilities) -> Self {
        let mut reasons = Vec::new();
        if !capabilities.restricted_identity {
            reasons.push(CodeSandboxReadinessReason::RestrictedIdentityUnavailable);
        }
        if !capabilities.job_cpu_limit
            || !capabilities.job_memory_limit
            || !capabilities.job_process_limit
        {
            reasons.push(CodeSandboxReadinessReason::ResourceLimitsUnavailable);
        }
        if !capabilities.kill_process_tree {
            reasons.push(CodeSandboxReadinessReason::ProcessTreeContainmentUnavailable);
        }
        if !capabilities.acl_confinement {
            reasons.push(CodeSandboxReadinessReason::AclConfinementUnavailable);
        }
        if !capabilities.network_denied {
            reasons.push(CodeSandboxReadinessReason::NetworkIsolationUnavailable);
        }
        Self {
            ready: reasons.is_empty(),
            reasons,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::code_execution::application::{
        SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
    };

    struct Backend(SandboxBackendCapabilities);

    impl SandboxProcessBackend for Backend {
        fn capabilities(&self) -> SandboxBackendCapabilities {
            self.0
        }

        fn launch(
            &self,
            _request: SandboxLaunchRequest,
        ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
            Err(SandboxBackendError::IsolationUnavailable)
        }
    }

    #[test]
    fn network_isolation_is_required_even_when_other_capabilities_exist() {
        let backend = Backend(SandboxBackendCapabilities {
            restricted_identity: true,
            job_cpu_limit: true,
            job_memory_limit: true,
            job_process_limit: true,
            kill_process_tree: true,
            acl_confinement: true,
            network_denied: false,
        });
        assert_eq!(
            CodeSandboxReadiness::probe(&backend),
            CodeSandboxReadiness {
                ready: false,
                reasons: vec![CodeSandboxReadinessReason::NetworkIsolationUnavailable],
            }
        );
    }
}
