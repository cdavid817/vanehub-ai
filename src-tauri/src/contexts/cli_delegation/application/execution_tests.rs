use super::*;
use std::sync::atomic::AtomicUsize;

struct Process {
    observation: Option<DelegationExecutionObservation>,
    terminated: Arc<AtomicBool>,
    cancel_on_wait: Option<Arc<AtomicBool>>,
}

impl DelegationOwnedProcess for Process {
    fn wait_until(
        &mut self,
        _: Instant,
    ) -> Result<Option<DelegationExecutionObservation>, DelegationExecutionError> {
        if let Some(cancelled) = &self.cancel_on_wait {
            cancelled.store(true, Ordering::Release);
        }
        Ok(self.observation.take())
    }

    fn terminate_tree(&mut self, _: Duration) -> Result<(), DelegationExecutionError> {
        self.terminated.store(true, Ordering::Release);
        Ok(())
    }
}

struct Launcher {
    observation: DelegationExecutionObservation,
    terminated: Arc<AtomicBool>,
    cleaned: Arc<AtomicUsize>,
}

impl DelegationProcessLauncher for Launcher {
    fn launch(
        &self,
        _: &DelegationExecutionRequest,
    ) -> Result<Box<dyn DelegationOwnedProcess>, DelegationExecutionError> {
        Ok(Box::new(Process {
            observation: Some(self.observation.clone()),
            terminated: self.terminated.clone(),
            cancel_on_wait: None,
        }))
    }

    fn cleanup(&self, _: &DelegationExecutionRequest) -> Result<(), DelegationExecutionError> {
        self.cleaned.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

struct CancellingLauncher {
    cancelled: Arc<AtomicBool>,
    terminated: Arc<AtomicBool>,
    cleaned: Arc<AtomicUsize>,
}

impl DelegationProcessLauncher for CancellingLauncher {
    fn launch(
        &self,
        _: &DelegationExecutionRequest,
    ) -> Result<Box<dyn DelegationOwnedProcess>, DelegationExecutionError> {
        Ok(Box::new(Process {
            observation: None,
            terminated: self.terminated.clone(),
            cancel_on_wait: Some(self.cancelled.clone()),
        }))
    }

    fn cleanup(&self, _: &DelegationExecutionRequest) -> Result<(), DelegationExecutionError> {
        self.cleaned.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}

fn request() -> DelegationExecutionRequest {
    let root = if cfg!(windows) {
        "C:/delegation"
    } else {
        "/delegation"
    };
    DelegationExecutionRequest {
        executable: PathBuf::from(format!("{root}/cli")),
        arguments: vec!["exec".into()],
        working_directory: PathBuf::from(format!("{root}/workspace")),
        environment: BTreeMap::new(),
        stdin: b"task".to_vec(),
        limits: DelegationExecutionLimits {
            wall_time: Duration::from_secs(1),
            stdout_bytes: 8,
            stderr_bytes: 8,
            events: 2,
        },
    }
}

fn runner(
    observation: DelegationExecutionObservation,
) -> (DelegationExecutionRunner, Arc<AtomicBool>, Arc<AtomicUsize>) {
    let terminated = Arc::new(AtomicBool::new(false));
    let cleaned = Arc::new(AtomicUsize::new(0));
    (
        DelegationExecutionRunner::new(Arc::new(Launcher {
            observation,
            terminated: terminated.clone(),
            cleaned: cleaned.clone(),
        })),
        terminated,
        cleaned,
    )
}

#[test]
fn bounded_success_is_observed_and_always_cleaned() {
    let (runner, terminated, cleaned) = runner(DelegationExecutionObservation {
        exit_code: 0,
        stdout: b"{}\n".to_vec(),
        stderr: Vec::new(),
        event_count: 1,
    });
    assert_eq!(
        runner
            .run(&request(), &AtomicBool::new(false))
            .expect("run")
            .exit_code,
        0
    );
    assert!(!terminated.load(Ordering::Acquire));
    assert_eq!(cleaned.load(Ordering::Acquire), 1);
}

#[test]
fn cancellation_terminates_the_owned_tree_and_cleans_up() {
    let (runner, terminated, cleaned) = runner(DelegationExecutionObservation {
        exit_code: 0,
        stdout: Vec::new(),
        stderr: Vec::new(),
        event_count: 0,
    });
    assert_eq!(
        runner.run(&request(), &AtomicBool::new(true)),
        Err(DelegationExecutionError::Cancelled)
    );
    assert!(
        !terminated.load(Ordering::Acquire),
        "no process was launched"
    );
    assert_eq!(cleaned.load(Ordering::Acquire), 1);
}

#[test]
fn cancellation_after_launch_terminates_the_owned_tree() {
    let cancelled = Arc::new(AtomicBool::new(false));
    let terminated = Arc::new(AtomicBool::new(false));
    let cleaned = Arc::new(AtomicUsize::new(0));
    let runner = DelegationExecutionRunner::new(Arc::new(CancellingLauncher {
        cancelled: cancelled.clone(),
        terminated: terminated.clone(),
        cleaned: cleaned.clone(),
    }));
    assert_eq!(
        runner.run(&request(), &cancelled),
        Err(DelegationExecutionError::Cancelled)
    );
    assert!(terminated.load(Ordering::Acquire));
    assert_eq!(cleaned.load(Ordering::Acquire), 1);
}

#[test]
fn output_and_event_limits_fail_without_returning_partial_success() {
    let (runner, _, cleaned) = runner(DelegationExecutionObservation {
        exit_code: 0,
        stdout: vec![b'x'; 9],
        stderr: Vec::new(),
        event_count: 3,
    });
    assert_eq!(
        runner.run(&request(), &AtomicBool::new(false)),
        Err(DelegationExecutionError::OutputLimit)
    );
    assert_eq!(cleaned.load(Ordering::Acquire), 1);
}
