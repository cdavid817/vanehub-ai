use crate::contexts::cli_delegation::application::{
    DelegationExecutionError, DelegationExecutionObservation, DelegationExecutionRequest,
    DelegationOwnedProcess, DelegationProcessLauncher,
};
use crate::platform::process::{BlockingStderrDrain, ManagedChild};
use std::io::Write;
use std::time::{Duration, Instant};

#[derive(Debug, Default)]
pub(crate) struct ManagedDelegationProcessLauncher;

impl DelegationProcessLauncher for ManagedDelegationProcessLauncher {
    fn launch(
        &self,
        request: &DelegationExecutionRequest,
    ) -> Result<Box<dyn DelegationOwnedProcess>, DelegationExecutionError> {
        let executable = request
            .executable
            .to_str()
            .ok_or(DelegationExecutionError::InvalidRequest)?;
        let mut child = ManagedChild::spawn_in(
            executable,
            &request.arguments,
            &request.environment,
            Some(&request.working_directory),
        )
        .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        let mut stdin = child
            .take_stdin()
            .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        stdin
            .write_all(&request.stdin)
            .and_then(|_| stdin.flush())
            .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        drop(stdin);
        let stdout = child
            .take_stdout()
            .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        let stderr = child
            .take_stderr()
            .map_err(|_| DelegationExecutionError::SpawnFailed)?;
        Ok(Box::new(ManagedDelegationProcess {
            child,
            stdout: Some(BlockingStderrDrain::spawn(
                stdout,
                request.limits.stdout_bytes.saturating_add(1),
            )),
            stderr: Some(BlockingStderrDrain::spawn(
                stderr,
                request.limits.stderr_bytes.saturating_add(1),
            )),
        }))
    }

    fn cleanup(
        &self,
        _request: &DelegationExecutionRequest,
    ) -> Result<(), DelegationExecutionError> {
        Ok(())
    }
}

struct ManagedDelegationProcess {
    child: ManagedChild,
    stdout: Option<BlockingStderrDrain>,
    stderr: Option<BlockingStderrDrain>,
}

impl DelegationOwnedProcess for ManagedDelegationProcess {
    fn wait_until(
        &mut self,
        deadline: Instant,
    ) -> Result<Option<DelegationExecutionObservation>, DelegationExecutionError> {
        let status = self
            .child
            .wait_until(deadline)
            .map_err(|_| DelegationExecutionError::WaitFailed)?;
        let Some(status) = status else {
            return Ok(None);
        };
        let stdout = finish(self.stdout.take())?;
        let stderr = finish(self.stderr.take())?;
        if stdout.observed_bytes() > stdout.retained().len() as u64
            || stderr.observed_bytes() > stderr.retained().len() as u64
        {
            return Err(DelegationExecutionError::OutputLimit);
        }
        let event_count = stdout
            .retained()
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count();
        Ok(Some(DelegationExecutionObservation {
            exit_code: status.code().unwrap_or(-1),
            stdout: stdout.retained().to_vec(),
            stderr: stderr.retained().to_vec(),
            event_count,
        }))
    }

    fn terminate_tree(&mut self, timeout: Duration) -> Result<(), DelegationExecutionError> {
        self.child
            .shutdown(Instant::now() + timeout)
            .map(|_| ())
            .map_err(|_| DelegationExecutionError::ProcessTreeTerminationFailed)
    }
}

fn finish(
    drain: Option<BlockingStderrDrain>,
) -> Result<crate::platform::process::StderrCapture, DelegationExecutionError> {
    drain
        .ok_or(DelegationExecutionError::WaitFailed)?
        .finish(Duration::from_secs(2))
        .map_err(|_| DelegationExecutionError::WaitFailed)
}
