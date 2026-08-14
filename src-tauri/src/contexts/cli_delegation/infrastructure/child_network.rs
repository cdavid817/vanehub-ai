use crate::contexts::cli_delegation::application::DelegationChildNetworkPort;
use crate::contexts::code_execution::application::SandboxProcessBackend;
use std::sync::Arc;

pub(crate) struct SandboxChildNetworkAdapter {
    backend: Arc<dyn SandboxProcessBackend>,
}

impl SandboxChildNetworkAdapter {
    pub(crate) fn new(backend: Arc<dyn SandboxProcessBackend>) -> Self {
        Self { backend }
    }
}

impl DelegationChildNetworkPort for SandboxChildNetworkAdapter {
    fn network_denied(&self) -> bool {
        let capabilities = self.backend.capabilities();
        capabilities.network_denied
            && capabilities.restricted_identity
            && capabilities.kill_process_tree
            && capabilities.acl_confinement
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::code_execution::application::{
        SandboxBackendCapabilities, SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
    };

    struct Backend(SandboxBackendCapabilities);

    impl SandboxProcessBackend for Backend {
        fn capabilities(&self) -> SandboxBackendCapabilities {
            self.0
        }

        fn launch(
            &self,
            _: SandboxLaunchRequest,
        ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
            Err(SandboxBackendError::IsolationUnavailable)
        }
    }

    #[test]
    fn readiness_requires_network_identity_tree_and_acl_isolation_together() {
        let ready = SandboxBackendCapabilities {
            restricted_identity: true,
            job_cpu_limit: true,
            job_memory_limit: true,
            job_process_limit: true,
            kill_process_tree: true,
            acl_confinement: true,
            network_denied: true,
        };
        assert!(SandboxChildNetworkAdapter::new(Arc::new(Backend(ready))).network_denied());
        assert!(
            !SandboxChildNetworkAdapter::new(Arc::new(Backend(SandboxBackendCapabilities {
                network_denied: false,
                ..ready
            })))
            .network_denied()
        );
    }
}
