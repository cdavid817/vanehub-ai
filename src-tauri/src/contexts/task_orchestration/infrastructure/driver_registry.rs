use crate::contexts::task_orchestration::application::PlanApplicationError;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct NativePlanDriverRegistry {
    active: Arc<Mutex<HashSet<String>>>,
}

impl NativePlanDriverRegistry {
    pub(crate) fn activate(&self, run_id: &str) -> Result<bool, PlanApplicationError> {
        self.active
            .lock()
            .map_err(|_| PlanApplicationError::Storage("Plan driver registry unavailable".into()))
            .map(|mut active| active.insert(run_id.to_string()))
    }

    pub(crate) fn deactivate(&self, run_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            active.remove(run_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activation_is_singleton_until_the_driver_deactivates() {
        let registry = NativePlanDriverRegistry::default();
        assert!(registry.activate("run-1").expect("first"));
        assert!(!registry.activate("run-1").expect("duplicate"));
        assert!(registry.activate("run-2").expect("independent"));
        registry.deactivate("run-1");
        assert!(registry.activate("run-1").expect("reactivate"));
    }
}
