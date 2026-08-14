use super::DelegationTarget;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationProviderConnection {
    pub(crate) target: DelegationTarget,
    pub(crate) executable: PathBuf,
    pub(crate) provider_network_allowed: bool,
    pub(crate) child_commands_network_denied: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationNetworkError {
    InvalidExecutable,
    ProviderConnectionUnavailable,
    ChildNetworkIsolationUnavailable,
}

pub(crate) trait DelegationChildNetworkPort: Send + Sync {
    fn network_denied(&self) -> bool;
}

pub(crate) struct DelegationNetworkPolicy {
    child_network: Arc<dyn DelegationChildNetworkPort>,
}

impl DelegationNetworkPolicy {
    pub(crate) fn new(child_network: Arc<dyn DelegationChildNetworkPort>) -> Self {
        Self { child_network }
    }

    pub(crate) fn admit_provider(
        &self,
        target: DelegationTarget,
        executable: PathBuf,
        provider_connection_available: bool,
    ) -> Result<DelegationProviderConnection, DelegationNetworkError> {
        if !executable.is_absolute() {
            return Err(DelegationNetworkError::InvalidExecutable);
        }
        if !provider_connection_available {
            return Err(DelegationNetworkError::ProviderConnectionUnavailable);
        }
        if !self.child_network.network_denied() {
            return Err(DelegationNetworkError::ChildNetworkIsolationUnavailable);
        }
        Ok(DelegationProviderConnection {
            target,
            executable,
            provider_network_allowed: true,
            child_commands_network_denied: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct ChildNetwork(bool);

    impl DelegationChildNetworkPort for ChildNetwork {
        fn network_denied(&self) -> bool {
            self.0
        }
    }

    fn executable() -> PathBuf {
        if cfg!(windows) {
            PathBuf::from("C:/tools/codex.exe")
        } else {
            PathBuf::from("/usr/bin/codex")
        }
    }

    #[test]
    fn provider_connectivity_is_admitted_only_with_independent_child_network_denial() {
        let admitted = DelegationNetworkPolicy::new(Arc::new(ChildNetwork(true)))
            .admit_provider(DelegationTarget::CodexCli, executable(), true)
            .expect("admitted");
        assert!(admitted.provider_network_allowed);
        assert!(admitted.child_commands_network_denied);

        assert_eq!(
            DelegationNetworkPolicy::new(Arc::new(ChildNetwork(false))).admit_provider(
                DelegationTarget::ClaudeCode,
                executable(),
                true
            ),
            Err(DelegationNetworkError::ChildNetworkIsolationUnavailable)
        );
    }

    #[test]
    fn missing_provider_connectivity_or_relative_executable_fails_closed() {
        let policy = DelegationNetworkPolicy::new(Arc::new(ChildNetwork(true)));
        assert_eq!(
            policy.admit_provider(DelegationTarget::CodexCli, executable(), false),
            Err(DelegationNetworkError::ProviderConnectionUnavailable)
        );
        assert_eq!(
            policy.admit_provider(DelegationTarget::CodexCli, PathBuf::from("codex"), true),
            Err(DelegationNetworkError::InvalidExecutable)
        );
    }
}
