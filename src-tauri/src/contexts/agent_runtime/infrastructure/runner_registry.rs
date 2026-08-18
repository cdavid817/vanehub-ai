use crate::contexts::agent_runtime::application::{
    AgentRunner, PreparedRunnerLaunch, RunnerCapabilities, RunnerError, RunnerErrorKind,
    RunnerEvent, RunnerHandle, RunnerInspection, RunnerKind, RunnerLaunchSpec, RunnerReference,
    RunnerSelection,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const MAX_ACTIVE_RUNNERS: usize = 32;
const MAX_ACTIVE_PER_KIND: usize = 24;
const MAX_ACTIVE_PER_SSH_TARGET: usize = 8;

#[derive(Clone)]
struct Admission {
    kind: RunnerKind,
    target_id: Option<String>,
}

#[derive(Default)]
struct ResourceState {
    reservations: HashMap<String, Admission>,
    handles: HashMap<String, Admission>,
}

#[derive(Clone, Copy)]
struct ResourceLimits {
    global: usize,
    per_kind: usize,
    per_ssh_target: usize,
}

pub(crate) struct RunnerRegistry {
    runners: HashMap<RunnerKind, Arc<dyn AgentRunner>>,
    resources: Mutex<ResourceState>,
    sequence: AtomicU64,
    limits: ResourceLimits,
}

impl RunnerRegistry {
    pub(crate) fn new(runners: Vec<Arc<dyn AgentRunner>>) -> Result<Self, RunnerError> {
        let mut indexed = HashMap::new();
        for runner in runners {
            let _capabilities = runner.capabilities();
            if indexed.insert(runner.kind(), runner).is_some() {
                return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
            }
        }
        if !indexed.contains_key(&RunnerKind::Local) {
            return Err(RunnerError::new(RunnerErrorKind::InvalidSelection));
        }
        Ok(Self {
            runners: indexed,
            resources: Mutex::new(ResourceState::default()),
            sequence: AtomicU64::new(0),
            limits: ResourceLimits {
                global: MAX_ACTIVE_RUNNERS,
                per_kind: MAX_ACTIVE_PER_KIND,
                per_ssh_target: MAX_ACTIVE_PER_SSH_TARGET,
            },
        })
    }

    fn runner(&self, kind: RunnerKind) -> Result<&Arc<dyn AgentRunner>, RunnerError> {
        self.runners
            .get(&kind)
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::UnsupportedCapability))
    }

    fn reserve(&self, selection: &RunnerSelection) -> Result<String, RunnerError> {
        let mut state = self
            .resources
            .lock()
            .map_err(|_| RunnerError::new(RunnerErrorKind::ResourceExhausted))?;
        let admissions = state.reservations.values().chain(state.handles.values());
        let all = admissions.cloned().collect::<Vec<_>>();
        let kind_count = all
            .iter()
            .filter(|item| item.kind == selection.kind)
            .count();
        let target_count = selection.target_id.as_ref().map_or(0, |target| {
            all.iter()
                .filter(|item| item.target_id.as_ref() == Some(target))
                .count()
        });
        if all.len() >= self.limits.global
            || kind_count >= self.limits.per_kind
            || (selection.kind == RunnerKind::Ssh && target_count >= self.limits.per_ssh_target)
        {
            return Err(RunnerError::new(RunnerErrorKind::ResourceExhausted));
        }
        let id = format!(
            "runner-admission-{}",
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        );
        state.reservations.insert(
            id.clone(),
            Admission {
                kind: selection.kind,
                target_id: selection.target_id.clone(),
            },
        );
        Ok(id)
    }

    fn release_reservation(&self, id: &str) {
        if let Ok(mut state) = self.resources.lock() {
            state.reservations.remove(id);
        }
    }
}

impl AgentRunner for RunnerRegistry {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Local
    }

    fn capabilities(&self) -> RunnerCapabilities {
        self.runners
            .get(&RunnerKind::Local)
            .expect("RunnerRegistry validates Local at construction")
            .capabilities()
    }

    fn prepare(
        &self,
        selection: &RunnerSelection,
        spec: RunnerLaunchSpec,
    ) -> Result<PreparedRunnerLaunch, RunnerError> {
        let runner = self.runner(selection.kind)?;
        let admission_id = self.reserve(selection)?;
        match runner.prepare(selection, spec) {
            Ok(mut prepared) => {
                prepared.admission_id = Some(admission_id);
                Ok(prepared)
            }
            Err(error) => {
                self.release_reservation(&admission_id);
                Err(error)
            }
        }
    }

    fn spawn(&self, prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
        let admission_id = prepared
            .admission_id
            .as_deref()
            .ok_or_else(|| RunnerError::new(RunnerErrorKind::ResourceExhausted))?
            .to_string();
        let kind = prepared.reference.kind;
        let runner = self.runner(kind)?;
        match runner.spawn(prepared) {
            Ok(handle) => {
                let Ok(mut state) = self.resources.lock() else {
                    let _ = runner.cancel(&handle);
                    let _ = runner.cleanup(&handle);
                    return Err(RunnerError::new(RunnerErrorKind::ResourceExhausted));
                };
                let admission = state
                    .reservations
                    .remove(&admission_id)
                    .ok_or_else(|| RunnerError::new(RunnerErrorKind::ResourceExhausted))?;
                state.handles.insert(handle.id.clone(), admission);
                Ok(handle)
            }
            Err(error) => {
                self.release_reservation(&admission_id);
                Err(error)
            }
        }
    }

    fn send_input(&self, handle: &RunnerHandle, content: &[u8]) -> Result<(), RunnerError> {
        self.runner(handle.reference.kind)?
            .send_input(handle, content)
    }

    fn next_event(&self, handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
        self.runner(handle.reference.kind)?.next_event(handle)
    }

    fn cancel(&self, handle: &RunnerHandle) -> Result<bool, RunnerError> {
        self.runner(handle.reference.kind)?.cancel(handle)
    }

    fn inspect(&self, handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
        self.runner(handle.reference.kind)?.inspect(handle)
    }

    fn cleanup(&self, handle: &RunnerHandle) -> Result<(), RunnerError> {
        self.runner(handle.reference.kind)?.cleanup(handle)?;
        if let Ok(mut state) = self.resources.lock() {
            state.handles.remove(&handle.id);
        }
        Ok(())
    }

    fn recover(
        &self,
        reference: &RunnerReference,
        process_reference: Option<&str>,
    ) -> Result<RunnerInspection, RunnerError> {
        self.runner(reference.kind)?
            .recover(reference, process_reference)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::infrastructure::LocalRunner;
    use std::sync::atomic::AtomicUsize;

    struct CountingRunner(Arc<AtomicUsize>);

    impl AgentRunner for CountingRunner {
        fn kind(&self) -> RunnerKind {
            RunnerKind::Local
        }

        fn capabilities(&self) -> RunnerCapabilities {
            LocalRunner::new().capabilities()
        }

        fn prepare(
            &self,
            _selection: &RunnerSelection,
            spec: RunnerLaunchSpec,
        ) -> Result<PreparedRunnerLaunch, RunnerError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(PreparedRunnerLaunch {
                reference: RunnerReference::local(),
                spec,
                preparation_id: None,
                admission_id: None,
            })
        }

        fn spawn(&self, _prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
            unreachable!()
        }
        fn send_input(&self, _: &RunnerHandle, _: &[u8]) -> Result<(), RunnerError> {
            unreachable!()
        }
        fn next_event(&self, _: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
            unreachable!()
        }
        fn cancel(&self, _: &RunnerHandle) -> Result<bool, RunnerError> {
            unreachable!()
        }
        fn inspect(&self, _: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
            unreachable!()
        }
        fn cleanup(&self, _: &RunnerHandle) -> Result<(), RunnerError> {
            unreachable!()
        }
        fn recover(
            &self,
            _: &RunnerReference,
            _: Option<&str>,
        ) -> Result<RunnerInspection, RunnerError> {
            unreachable!()
        }
    }

    fn launch() -> RunnerLaunchSpec {
        RunnerLaunchSpec {
            session_id: Some("session-1".into()),
            executable: "fixture".into(),
            arguments: Vec::new(),
            cwd: Some("workspace".into()),
            environment: Default::default(),
            pipe_stdin: false,
        }
    }

    #[test]
    fn registry_requires_local_and_rejects_duplicate_kinds() {
        assert_eq!(
            RunnerRegistry::new(Vec::new())
                .err()
                .expect("missing Local")
                .kind,
            RunnerErrorKind::InvalidSelection
        );
        let first: Arc<dyn AgentRunner> = Arc::new(LocalRunner::new());
        let second: Arc<dyn AgentRunner> = Arc::new(LocalRunner::new());
        assert_eq!(
            RunnerRegistry::new(vec![first, second])
                .err()
                .expect("duplicate Local")
                .kind,
            RunnerErrorKind::InvalidSelection
        );
    }

    #[test]
    fn quota_rejection_is_atomic_and_has_no_runner_side_effect() {
        let calls = Arc::new(AtomicUsize::new(0));
        let registry =
            RunnerRegistry::new(vec![Arc::new(CountingRunner(calls.clone()))]).expect("registry");
        for _ in 0..MAX_ACTIVE_PER_KIND {
            registry
                .prepare(&RunnerSelection::local(), launch())
                .expect("within quota");
        }
        assert_eq!(
            registry
                .prepare(&RunnerSelection::local(), launch())
                .expect_err("quota")
                .kind,
            RunnerErrorKind::ResourceExhausted
        );
        assert_eq!(calls.load(Ordering::SeqCst), MAX_ACTIVE_PER_KIND);
    }
}
