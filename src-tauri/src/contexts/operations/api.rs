//! Published in-process facade for observable operations and unified logging contracts.
//!
//! Native contexts use this boundary to start and finish operations, append page-visible output,
//! correlate execution traces, and emit redacted diagnostics without owning the logging storage.

use crate::contexts::operations::application::{ApplicationError, OperationService};
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use crate::contexts::operations::application::AgentRunService;
pub(crate) use crate::contexts::operations::application::ApplicationError as OperationsError;
pub(crate) use crate::contexts::operations::application::CreateAgentRun;
pub(crate) use crate::contexts::operations::application::RunListFilter;
pub(crate) use crate::contexts::operations::application::RunPage;
pub(crate) use crate::contexts::operations::application::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLog,
    OperationLogPort,
};
use crate::contexts::operations::domain::OperationRecoveryEvidence;
pub(crate) use crate::contexts::operations::domain::{
    AgentRun, RunEvent, RunLink, RunOwner, RunRecoveryPolicy, RunState, RunTrigger,
};
pub(crate) use crate::contexts::operations::domain::{OperationKind, OperationTask};

#[derive(Clone)]
/// Application facade for operation lifecycle and correlation.
pub(crate) struct OperationsApi {
    service: OperationService,
}

#[derive(Clone)]
pub(crate) struct AgentRunsApi {
    service: AgentRunService,
}

impl AgentRunsApi {
    pub(crate) fn new(service: AgentRunService) -> Self {
        Self { service }
    }
    pub(crate) fn create(&self, input: CreateAgentRun) -> Result<AgentRun, ApplicationError> {
        self.service.create(input)
    }
    pub(crate) fn get(&self, id: &str) -> Result<AgentRun, ApplicationError> {
        self.service.get(id)
    }
    pub(crate) fn list(
        &self,
        filter: RunListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<RunPage, ApplicationError> {
        self.service.list(filter, offset, limit)
    }
    pub(crate) fn events(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ApplicationError> {
        self.service.events(id, offset, limit)
    }
    pub(crate) fn cancel(&self, id: &str, version: u64) -> Result<AgentRun, ApplicationError> {
        self.service.cancel_tree(
            id,
            version,
            RunTrigger::CancelUser,
            format!("user-cancel:{id}:{version}"),
        )
    }
    pub(crate) fn resume(&self, id: &str, version: u64) -> Result<AgentRun, ApplicationError> {
        self.service
            .resume(id, version, format!("user-resume:{id}:{version}"))
    }
    pub(crate) fn reconcile_after_restart(&self) -> Result<usize, ApplicationError> {
        self.service.reconcile_after_restart()
    }
    pub(crate) fn transition(
        &self,
        id: &str,
        trigger: RunTrigger,
        reason: Option<String>,
    ) -> Result<AgentRun, ApplicationError> {
        let run = self.service.get(id)?;
        self.service.transition(
            id,
            run.version,
            trigger,
            reason,
            format!("native:{id}:{}:{trigger:?}", run.version),
        )
    }
}

impl OperationsApi {
    pub(crate) fn new(service: OperationService) -> Self {
        Self { service }
    }

    pub(crate) fn start(
        &self,
        kind: OperationKind,
        related_entity_id: Option<String>,
        message: Option<String>,
    ) -> Result<OperationTask, ApplicationError> {
        self.service.start(kind, related_entity_id, message)
    }

    pub(crate) fn append_log(
        &self,
        operation_id: &str,
        line: String,
    ) -> Result<OperationTask, ApplicationError> {
        self.service.append_log(operation_id, line)
    }

    pub(crate) fn correlate_execution(
        &self,
        operation_id: &str,
        run_id: String,
        trace_id: String,
    ) -> Result<OperationTask, ApplicationError> {
        self.service
            .correlate_execution(operation_id, run_id, trace_id)
    }

    pub(crate) fn complete(
        &self,
        operation_id: &str,
        result: Option<Value>,
    ) -> Result<OperationTask, ApplicationError> {
        self.service.complete(operation_id, result)
    }

    pub(crate) fn fail(
        &self,
        operation_id: &str,
        error: String,
    ) -> Result<OperationTask, ApplicationError> {
        self.service.fail(operation_id, error)
    }

    pub(crate) fn cancel(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.service.cancel(operation_id)
    }

    pub(crate) fn cancellation_flag(
        &self,
        operation_id: &str,
    ) -> Result<Arc<AtomicBool>, ApplicationError> {
        self.service.cancellation_flag(operation_id)
    }

    pub(crate) fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.service.get(operation_id)
    }

    pub(crate) fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
        self.service.list()
    }

    pub(crate) fn list_recovery_evidence(
        &self,
        execution_run_id: &str,
        limit: usize,
    ) -> Result<Vec<OperationRecoveryEvidence>, ApplicationError> {
        self.service.list_recovery_evidence(execution_run_id, limit)
    }
}
