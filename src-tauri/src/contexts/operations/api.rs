//! Published in-process facade for observable operations and unified logging contracts.
//!
//! Native contexts use this boundary to start and finish operations, append page-visible output,
//! correlate execution traces, and emit redacted diagnostics without owning the logging storage.

pub(crate) use super::application::{OperationsEvidencePort, OperationsEvidenceSignal};
use crate::contexts::operations::application::{ApplicationError, OperationService};
use serde_json::Value;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

pub(crate) use crate::contexts::operations::application::ApplicationError as OperationsError;
pub(crate) use crate::contexts::operations::application::CreateAgentRun;
pub(crate) use crate::contexts::operations::application::RunListFilter;
pub(crate) use crate::contexts::operations::application::RunPage;
use crate::contexts::operations::application::{AgentRunService, MissionControlService};
pub(crate) use crate::contexts::operations::application::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLog,
    OperationLogPort,
};
pub(crate) use crate::contexts::operations::application::{
    MissionControlOverview, MissionControlQuery, MissionControlRunDetail,
};
pub(crate) use crate::contexts::operations::application::{
    RunOwnerRecoveryPort, RunRecoveryDecision,
};
use crate::contexts::operations::domain::OperationRecoveryEvidence;
pub(crate) use crate::contexts::operations::domain::{
    AgentRun, RunEvent, RunLink, RunOwner, RunRecoveryPolicy, RunRunner, RunRunnerKind,
    RunRunnerRecovery, RunState, RunTrigger,
};
pub(crate) use crate::contexts::operations::domain::{
    OperationKind, OperationProgress, OperationTask,
};

#[derive(Clone)]
/// Application facade for operation lifecycle and correlation.
pub(crate) struct OperationsApi {
    service: OperationService,
}

#[derive(Clone)]
pub(crate) struct AgentRunsApi {
    service: AgentRunService,
    mission_control: MissionControlService,
}

impl AgentRunsApi {
    pub(crate) fn new(service: AgentRunService, mission_control: MissionControlService) -> Self {
        Self {
            service,
            mission_control,
        }
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
    pub(crate) fn mission_control_overview(
        &self,
        query: MissionControlQuery,
    ) -> Result<MissionControlOverview, ApplicationError> {
        self.mission_control.overview(query)
    }
    pub(crate) fn mission_control_run(
        &self,
        run_id: &str,
    ) -> Result<MissionControlRunDetail, ApplicationError> {
        self.mission_control.detail(run_id)
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

    pub(crate) fn report_progress(
        &self,
        operation_id: &str,
        progress: OperationProgress,
    ) -> Result<OperationTask, ApplicationError> {
        self.service.report_progress(operation_id, progress)
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
