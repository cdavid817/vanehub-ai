use super::CodeExecutionLimits;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SandboxBackendCapabilities {
    pub(crate) restricted_identity: bool,
    pub(crate) job_cpu_limit: bool,
    pub(crate) job_memory_limit: bool,
    pub(crate) job_process_limit: bool,
    pub(crate) kill_process_tree: bool,
    pub(crate) acl_confinement: bool,
    pub(crate) network_denied: bool,
}

impl SandboxBackendCapabilities {
    pub(crate) const fn ready(self) -> bool {
        self.restricted_identity
            && self.job_cpu_limit
            && self.job_memory_limit
            && self.job_process_limit
            && self.kill_process_tree
            && self.acl_confinement
            && self.network_denied
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandboxLaunchRequest {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) limits: CodeExecutionLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SandboxBackendError {
    IsolationUnavailable,
    InvalidLaunch,
    SpawnFailed,
    JobSetupFailed,
    AclSetupFailed,
    ProcessCreationFailed(u32),
    JobAssignmentFailed,
    ResumeFailed,
    WaitFailed,
    TerminationFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SandboxProcessObservation {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) cpu_time_ms: Option<u64>,
    pub(crate) peak_memory_bytes: Option<u64>,
}

pub(crate) trait SandboxProcess: Send {
    fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<SandboxProcessObservation>, SandboxBackendError>;

    fn terminate_tree(&mut self, timeout: Duration) -> Result<(), SandboxBackendError>;
}

pub(crate) trait SandboxProcessBackend: Send + Sync {
    fn capabilities(&self) -> SandboxBackendCapabilities;

    fn launch(
        &self,
        request: SandboxLaunchRequest,
    ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readiness_requires_every_independent_isolation_property() {
        let complete = SandboxBackendCapabilities {
            restricted_identity: true,
            job_cpu_limit: true,
            job_memory_limit: true,
            job_process_limit: true,
            kill_process_tree: true,
            acl_confinement: true,
            network_denied: true,
        };
        assert!(complete.ready());
        for weakened in [
            SandboxBackendCapabilities {
                restricted_identity: false,
                ..complete
            },
            SandboxBackendCapabilities {
                job_memory_limit: false,
                ..complete
            },
            SandboxBackendCapabilities {
                acl_confinement: false,
                ..complete
            },
            SandboxBackendCapabilities {
                network_denied: false,
                ..complete
            },
        ] {
            assert!(!weakened.ready());
        }
    }
}
