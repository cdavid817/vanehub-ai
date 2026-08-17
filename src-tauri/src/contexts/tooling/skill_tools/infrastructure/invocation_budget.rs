use crate::contexts::tooling::skill_tools::application::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::application::SkillToolInvocationBudgetPort;
use crate::contexts::tooling::skill_tools::domain::SkillToolLimits;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeEnforcementStrength {
    WindowsJobObject,
    UnixProcessGroup,
    PortableBestEffort,
}

impl NativeEnforcementStrength {
    pub(crate) const fn current() -> Self {
        if cfg!(windows) {
            Self::WindowsJobObject
        } else if cfg!(unix) {
            Self::UnixProcessGroup
        } else {
            Self::PortableBestEffort
        }
    }

    pub(crate) const fn has_hard_cpu_or_memory_limit(self) -> bool {
        false
    }
}

#[derive(Debug, Default)]
struct Usage {
    host_calls: u32,
    output_bytes: u64,
    file_bytes: u64,
    network_bytes: u64,
    child_processes: u32,
    active_jobs: u32,
}

#[derive(Clone)]
pub(crate) struct SkillToolInvocationBudget {
    inner: Arc<BudgetInner>,
}

struct BudgetInner {
    limits: SkillToolLimits,
    started: Instant,
    usage: Mutex<Usage>,
}

pub(crate) struct SkillToolInvocationPermit {
    inner: Arc<BudgetInner>,
    children: u32,
    jobs: u32,
}

impl SkillToolInvocationBudget {
    pub(crate) fn new(limits: SkillToolLimits) -> Self {
        Self {
            inner: Arc::new(BudgetInner {
                limits,
                started: Instant::now(),
                usage: Mutex::new(Usage::default()),
            }),
        }
    }

    pub(crate) fn remaining_time(&self) -> Result<Duration, SkillToolApplicationError> {
        Duration::from_millis(self.inner.limits.wall_time_milliseconds)
            .checked_sub(self.inner.started.elapsed())
            .ok_or_else(|| limit("wall-time"))
    }

    pub(crate) fn reserve_host_call(&self) -> Result<(), SkillToolApplicationError> {
        self.reserve(Reservation {
            host_calls: 1,
            ..Reservation::default()
        })
        .map(drop)
    }

    pub(crate) fn consume_output(&self, bytes: u64) -> Result<(), SkillToolApplicationError> {
        self.reserve(Reservation {
            output_bytes: bytes,
            ..Reservation::default()
        })
        .map(drop)
    }

    pub(crate) fn consume_file(&self, bytes: u64) -> Result<(), SkillToolApplicationError> {
        self.reserve(Reservation {
            file_bytes: bytes,
            ..Reservation::default()
        })
        .map(drop)
    }

    pub(crate) fn consume_network(&self, bytes: u64) -> Result<(), SkillToolApplicationError> {
        self.reserve(Reservation {
            network_bytes: bytes,
            ..Reservation::default()
        })
        .map(drop)
    }

    pub(crate) fn enter_child(
        &self,
    ) -> Result<SkillToolInvocationPermit, SkillToolApplicationError> {
        self.reserve(Reservation {
            host_calls: 1,
            children: 1,
            jobs: 1,
            ..Reservation::default()
        })
    }

    fn reserve(
        &self,
        reservation: Reservation,
    ) -> Result<SkillToolInvocationPermit, SkillToolApplicationError> {
        self.remaining_time()?;
        let mut usage = self.inner.usage.lock().map_err(|_| limit("state"))?;
        let next = Usage {
            host_calls: checked_u32(usage.host_calls, reservation.host_calls, "host-calls")?,
            output_bytes: checked_u64(
                usage.output_bytes,
                reservation.output_bytes,
                "output-bytes",
            )?,
            file_bytes: checked_u64(usage.file_bytes, reservation.file_bytes, "file-bytes")?,
            network_bytes: checked_u64(
                usage.network_bytes,
                reservation.network_bytes,
                "network-bytes",
            )?,
            child_processes: checked_u32(usage.child_processes, reservation.children, "children")?,
            active_jobs: checked_u32(usage.active_jobs, reservation.jobs, "concurrency")?,
        };
        if next.host_calls > self.inner.limits.host_calls
            || next.output_bytes > self.inner.limits.output_bytes
            || next.file_bytes > self.inner.limits.file_bytes
            || next.network_bytes > self.inner.limits.network_bytes
            || next.child_processes > self.inner.limits.child_processes
            || next.active_jobs > self.inner.limits.concurrency
        {
            return Err(limit("aggregate"));
        }
        *usage = next;
        Ok(SkillToolInvocationPermit {
            inner: Arc::clone(&self.inner),
            children: reservation.children,
            jobs: reservation.jobs,
        })
    }
}

impl SkillToolInvocationBudgetPort for SkillToolInvocationBudget {
    fn reserve_host_call(&self) -> Result<(), SkillToolApplicationError> {
        SkillToolInvocationBudget::reserve_host_call(self)
    }

    fn consume_output(&self, bytes: u64) -> Result<(), SkillToolApplicationError> {
        SkillToolInvocationBudget::consume_output(self, bytes)
    }
}

#[derive(Default)]
struct Reservation {
    host_calls: u32,
    output_bytes: u64,
    file_bytes: u64,
    network_bytes: u64,
    children: u32,
    jobs: u32,
}

fn checked_u32(current: u32, addition: u32, name: &str) -> Result<u32, SkillToolApplicationError> {
    current.checked_add(addition).ok_or_else(|| limit(name))
}

fn checked_u64(current: u64, addition: u64, name: &str) -> Result<u64, SkillToolApplicationError> {
    current.checked_add(addition).ok_or_else(|| limit(name))
}

impl Drop for SkillToolInvocationPermit {
    fn drop(&mut self) {
        if self.children == 0 && self.jobs == 0 {
            return;
        }
        if let Ok(mut usage) = self.inner.usage.lock() {
            usage.active_jobs = usage.active_jobs.saturating_sub(self.jobs);
        }
    }
}

fn limit(name: &str) -> SkillToolApplicationError {
    SkillToolApplicationError::ResourceLimit(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::DEFAULT_SKILL_TOOL_LIMITS;

    #[test]
    fn cloned_nested_budgets_share_atomic_aggregate_counters() {
        let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
        limits.file_bytes = 4;
        let parent = SkillToolInvocationBudget::new(limits);
        let nested = parent.clone();
        parent.consume_file(3).expect("parent bytes");
        assert!(nested.consume_file(2).is_err());
    }

    #[test]
    fn child_count_is_aggregate_while_job_permits_release_on_drop() {
        let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
        limits.child_processes = 2;
        limits.concurrency = 1;
        let budget = SkillToolInvocationBudget::new(limits);
        let permit = budget.enter_child().expect("first");
        assert!(budget.enter_child().is_err());
        drop(permit);
        let second = budget.enter_child().expect("job permit released");
        drop(second);
        assert!(budget.enter_child().is_err());
    }

    #[test]
    fn enforcement_strength_never_claims_native_cpu_or_memory_isolation() {
        assert!(!NativeEnforcementStrength::current().has_hard_cpu_or_memory_limit());
    }

    #[test]
    fn concurrent_budget_exhaustion_admits_only_the_atomic_ceiling() {
        let mut limits = DEFAULT_SKILL_TOOL_LIMITS;
        limits.file_bytes = 4;
        let budget = SkillToolInvocationBudget::new(limits);
        let workers = (0..8)
            .map(|_| {
                let nested = budget.clone();
                std::thread::spawn(move || nested.consume_file(1).is_ok())
            })
            .collect::<Vec<_>>();
        let admitted = workers
            .into_iter()
            .map(|worker| worker.join().expect("worker"))
            .filter(|admitted| *admitted)
            .count();
        assert_eq!(admitted, 4);
    }
}
