use crate::contexts::operations::application::{ApplicationError, OperationService};
use serde_json::Value;

pub(crate) use crate::contexts::operations::application::ApplicationError as OperationsError;
pub(crate) use crate::contexts::operations::application::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLog,
    OperationLogPort,
};
pub(crate) use crate::contexts::operations::domain::{OperationKind, OperationTask};

#[derive(Clone)]
pub(crate) struct OperationsApi {
    service: OperationService,
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

    pub(crate) fn get(&self, operation_id: &str) -> Result<OperationTask, ApplicationError> {
        self.service.get(operation_id)
    }

    pub(crate) fn list(&self) -> Result<Vec<OperationTask>, ApplicationError> {
        self.service.list()
    }
}
