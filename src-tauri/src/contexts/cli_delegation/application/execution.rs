use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const POLL_INTERVAL: Duration = Duration::from_millis(100);
const TERMINATE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DelegationExecutionLimits {
    pub(crate) wall_time: Duration,
    pub(crate) stdout_bytes: usize,
    pub(crate) stderr_bytes: usize,
    pub(crate) events: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationExecutionRequest {
    pub(crate) executable: PathBuf,
    pub(crate) arguments: Vec<String>,
    pub(crate) working_directory: PathBuf,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) stdin: Vec<u8>,
    pub(crate) limits: DelegationExecutionLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DelegationExecutionObservation {
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) event_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DelegationExecutionError {
    InvalidRequest,
    SpawnFailed,
    Cancelled,
    TimedOut,
    OutputLimit,
    EventLimit,
    ProcessTreeTerminationFailed,
    WaitFailed,
    CleanupFailed,
}

pub(crate) trait DelegationOwnedProcess: Send {
    fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<DelegationExecutionObservation>, DelegationExecutionError>;
    fn terminate_tree(&mut self, timeout: Duration) -> Result<(), DelegationExecutionError>;
}

pub(crate) trait DelegationProcessLauncher: Send + Sync {
    fn launch(
        &self,
        request: &DelegationExecutionRequest,
    ) -> Result<Box<dyn DelegationOwnedProcess>, DelegationExecutionError>;
    fn cleanup(&self, request: &DelegationExecutionRequest)
        -> Result<(), DelegationExecutionError>;
}

pub(crate) struct DelegationExecutionRunner {
    launcher: Arc<dyn DelegationProcessLauncher>,
}

impl DelegationExecutionRunner {
    pub(crate) fn new(launcher: Arc<dyn DelegationProcessLauncher>) -> Self {
        Self { launcher }
    }

    pub(crate) fn run(
        &self,
        request: &DelegationExecutionRequest,
        cancelled: &AtomicBool,
    ) -> Result<DelegationExecutionObservation, DelegationExecutionError> {
        validate(request)?;
        if cancelled.load(Ordering::Acquire) {
            self.launcher.cleanup(request)?;
            return Err(DelegationExecutionError::Cancelled);
        }
        let mut process = self
            .launcher
            .launch(request)
            .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        let deadline = Instant::now() + request.limits.wall_time;
        let result = loop {
            if cancelled.load(Ordering::Acquire) {
                break stop(&mut *process, DelegationExecutionError::Cancelled);
            }
            let now = Instant::now();
            if now >= deadline {
                break stop(&mut *process, DelegationExecutionError::TimedOut);
            }
            match process.wait_until((now + POLL_INTERVAL).min(deadline)) {
                Ok(Some(observation)) => break enforce(observation, request.limits),
                Ok(None) => continue,
                Err(_) => break stop(&mut *process, DelegationExecutionError::WaitFailed),
            }
        };
        let cleanup = self.launcher.cleanup(request);
        match (result, cleanup) {
            (_, Err(_)) => Err(DelegationExecutionError::CleanupFailed),
            (result, Ok(())) => result,
        }
    }
}

fn stop(
    process: &mut dyn DelegationOwnedProcess,
    reason: DelegationExecutionError,
) -> Result<DelegationExecutionObservation, DelegationExecutionError> {
    process
        .terminate_tree(TERMINATE_TIMEOUT)
        .map_err(|_| DelegationExecutionError::ProcessTreeTerminationFailed)?;
    Err(reason)
}

fn validate(request: &DelegationExecutionRequest) -> Result<(), DelegationExecutionError> {
    if !request.executable.is_absolute()
        || !request.working_directory.is_absolute()
        || request.limits.wall_time.is_zero()
        || request.limits.stdout_bytes == 0
        || request.limits.stderr_bytes == 0
        || request.limits.events == 0
    {
        return Err(DelegationExecutionError::InvalidRequest);
    }
    Ok(())
}

fn enforce(
    observation: DelegationExecutionObservation,
    limits: DelegationExecutionLimits,
) -> Result<DelegationExecutionObservation, DelegationExecutionError> {
    if observation.stdout.len() > limits.stdout_bytes
        || observation.stderr.len() > limits.stderr_bytes
    {
        return Err(DelegationExecutionError::OutputLimit);
    }
    if observation.event_count > limits.events {
        return Err(DelegationExecutionError::EventLimit);
    }
    Ok(observation)
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
