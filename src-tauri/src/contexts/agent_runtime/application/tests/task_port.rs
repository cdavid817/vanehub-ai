use super::*;

impl AgentTaskPort for FakeWorld {
    fn finish_canonical_run(
        &self,
        run_id: &str,
        outcome: CanonicalRunOutcome,
        _reason: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::CanonicalRunFinished(
                run_id.to_string(),
                outcome,
            ));
        Ok(())
    }

    fn start_agent_launch(
        &self,
        agent_id: &str,
        _message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Started(agent_id.to_string()));
        Ok(AgentOperation {
            id: "operation-1".to_string(),
            related_agent_id: Some(agent_id.to_string()),
            message: None,
        })
    }

    fn start_agent_generation(
        &self,
        agent_id: &str,
        _session_id: &str,
        _message_id: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Started(agent_id.to_string()));
        Ok(AgentOperation {
            id: "generation-operation-1".to_string(),
            related_agent_id: Some(agent_id.to_string()),
            message: Some("Generating response".to_string()),
        })
    }

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        Ok(AgentOperation {
            id: format!("loop-{}", context.kind.as_str()),
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(
        &self,
        operation_id: &str,
        _line: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Logged(operation_id.to_string()));
        Ok(())
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Completed(operation_id.to_string()));
        Ok(())
    }

    fn fail(&self, operation_id: &str, _error: String) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Failed(operation_id.to_string()));
        Ok(())
    }

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(OperationEvent::Cancelled(operation_id.to_string()));
        Ok(())
    }
}
