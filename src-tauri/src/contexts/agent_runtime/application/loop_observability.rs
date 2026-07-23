use super::{
    AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError, AgentTaskPort, LoopLog,
    LoopLoggingPort, LoopOperationContext,
};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct LoopOperationObserver {
    operations: Arc<dyn AgentTaskPort>,
    logging: Arc<dyn LoopLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActiveLoopOperation {
    pub(crate) id: String,
    pub(crate) context: LoopOperationContext,
}

impl LoopOperationObserver {
    pub(crate) fn new(
        operations: Arc<dyn AgentTaskPort>,
        logging: Arc<dyn LoopLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
    ) -> Self {
        Self {
            operations,
            logging,
            clock,
        }
    }

    pub(crate) fn start(
        &self,
        context: LoopOperationContext,
        message: &str,
    ) -> Result<ActiveLoopOperation, AgentRuntimeApplicationError> {
        let operation = self.operations.start_loop_operation(&context, message)?;
        let active = ActiveLoopOperation {
            id: operation.id,
            context,
        };
        self.record(
            &active.context,
            Some(&active.id),
            AgentLogLevel::Info,
            message,
        )?;
        Ok(active)
    }

    pub(crate) fn record(
        &self,
        context: &LoopOperationContext,
        operation_id: Option<&str>,
        level: AgentLogLevel,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.logging.record_loop(LoopLog {
            level,
            category: format!("loop.{}", context.kind.as_str()),
            message: message.to_string(),
            context: context.clone(),
            operation_id: operation_id.map(str::to_string),
            occurred_at: self.clock.now(),
        })
    }

    pub(crate) fn complete(
        &self,
        operation: &ActiveLoopOperation,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.record(
            &operation.context,
            Some(&operation.id),
            AgentLogLevel::Info,
            message,
        )?;
        self.operations.complete(&operation.id)
    }

    pub(crate) fn fail(
        &self,
        operation: &ActiveLoopOperation,
        error: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.record(
            &operation.context,
            Some(&operation.id),
            AgentLogLevel::Error,
            error,
        )?;
        self.operations.fail(&operation.id, error.to_string())
    }

    pub(crate) fn cancel(
        &self,
        operation: &ActiveLoopOperation,
        message: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.record(
            &operation.context,
            Some(&operation.id),
            AgentLogLevel::Warn,
            message,
        )?;
        self.operations.cancel(&operation.id)
    }
}
