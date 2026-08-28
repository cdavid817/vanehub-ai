use std::sync::Arc;

use crate::contexts::personalization::application::{MemoryHealthPort, MigrationStatePort};
use crate::contexts::personalization::domain::MemoryRuntimeHealth;

/// Health as the durable row alone reports it.
///
/// The startup orchestration refines this with what one process observed — it alone knows that
/// another process holds maintenance right now. Everything else reads health from here, so a
/// surface that never runs maintenance still gets an answer, and it is the same answer the row
/// gives the orchestration.
#[derive(Clone)]
pub(crate) struct DurableMemoryHealth {
    state: Arc<dyn MigrationStatePort>,
}

impl DurableMemoryHealth {
    pub(crate) fn new(state: Arc<dyn MigrationStatePort>) -> Self {
        Self { state }
    }
}

impl MemoryHealthPort for DurableMemoryHealth {
    fn health(&self) -> MemoryRuntimeHealth {
        match self.state.load() {
            Ok(state) => state.health(),
            // An unreadable marker is not an argument for using the data behind it.
            Err(_) => MemoryRuntimeHealth::Failed,
        }
    }
}
