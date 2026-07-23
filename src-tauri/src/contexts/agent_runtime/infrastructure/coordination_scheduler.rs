use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, CoordinationApplicationService,
};
use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::thread;

#[derive(Clone)]
pub(crate) struct NativeCoordinationScheduler {
    service: CoordinationApplicationService,
    active: Arc<Mutex<HashSet<String>>>,
}

impl NativeCoordinationScheduler {
    pub(crate) fn new(service: CoordinationApplicationService) -> Self {
        Self {
            service,
            active: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub(crate) fn schedule(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        {
            let mut active = self
                .active
                .lock()
                .map_err(|error| AgentRuntimeApplicationError::Coordination(error.to_string()))?;
            if !active.insert(run_id.to_string()) {
                return Ok(());
            }
        }
        let service = self.service.clone();
        let active = self.active.clone();
        let run_id = run_id.to_string();
        thread::spawn(move || {
            if let Err(error) = service.execute(&run_id) {
                if service.handle_scheduler_failure(&run_id, &error).is_err() {
                    service.record_scheduler_settlement_failure(&run_id);
                }
            }
            if let Ok(mut leases) = active.lock() {
                leases.remove(&run_id);
            }
        });
        Ok(())
    }
}
