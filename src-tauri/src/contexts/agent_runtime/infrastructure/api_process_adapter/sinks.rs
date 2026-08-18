//! The evidence-counting `AgentProcessEventSink` decorator and the counts it reports.

use crate::contexts::agent_runtime::application::{
    AgentProcessEventSink, AgentRuntimeApplicationError, GenerationProcessEvent,
};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub(super) struct EvidenceToolCounts {
    pub(super) attempts: u32,
    pub(super) failures: u32,
}

pub(super) struct EvidenceCountingSink {
    inner: Arc<dyn AgentProcessEventSink>,
    attempts: AtomicU64,
    failures: AtomicU64,
}

impl EvidenceCountingSink {
    pub(super) fn new(inner: Arc<dyn AgentProcessEventSink>) -> Self {
        Self {
            inner,
            attempts: AtomicU64::new(0),
            failures: AtomicU64::new(0),
        }
    }

    pub(super) fn counts(&self) -> EvidenceToolCounts {
        EvidenceToolCounts {
            attempts: self
                .attempts
                .load(Ordering::Relaxed)
                .min(u64::from(u32::MAX)) as u32,
            failures: self
                .failures
                .load(Ordering::Relaxed)
                .min(u64::from(u32::MAX)) as u32,
        }
    }
}

impl AgentProcessEventSink for EvidenceCountingSink {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        if let GenerationProcessEvent::ToolLifecycle(tool) = &event {
            if matches!(
                tool.phase,
                crate::contexts::agent_runtime::application::ToolLifecyclePhase::Completed
                    | crate::contexts::agent_runtime::application::ToolLifecyclePhase::Failed
                    | crate::contexts::agent_runtime::application::ToolLifecyclePhase::Cancelled
            ) {
                self.attempts.fetch_add(1, Ordering::Relaxed);
            }
            if tool.phase == crate::contexts::agent_runtime::application::ToolLifecyclePhase::Failed
            {
                self.failures.fetch_add(1, Ordering::Relaxed);
            }
        }
        self.inner.handle(event)
    }
}
