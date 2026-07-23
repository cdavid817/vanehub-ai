use crate::contexts::execution_observability::api::{
    ExecutionFidelity, ExecutionIdentityPort, ExecutionRun, ExecutionSource, ExecutionSpan,
    ExecutionStatus, ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes,
};
use crate::contexts::execution_observability::infrastructure::{
    CompositeExecutionTelemetry, RandomExecutionIdentity, SqliteExecutionTimelineRepository,
};
use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::mcp::api::McpApi;
use crate::contexts::tooling::mcp::application::{
    McpApplicationError, McpApplicationService, McpTelemetryPort,
};
use crate::contexts::tooling::mcp::domain::{ConnectionOutcome, TransportType};
use crate::contexts::tooling::mcp::infrastructure::{
    CurrentProjectPathAdapter, McpOperationAdapter, RmcpConnectionAdapter,
    SqliteMcpServerRepository, SystemMcpClock, UnifiedMcpLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex;

struct NativeMcpTelemetryAdapter {
    identity: RandomExecutionIdentity,
    settings: Arc<SqliteExecutionTimelineRepository>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    contexts: Mutex<
        std::collections::BTreeMap<
            String,
            crate::contexts::execution_observability::api::ExecutionContext,
        >,
    >,
}

impl NativeMcpTelemetryAdapter {
    fn new(database: NativeDatabase) -> Self {
        let settings = Arc::new(SqliteExecutionTimelineRepository::new(database));
        let telemetry = Arc::new(CompositeExecutionTelemetry::new(
            settings.clone(),
            Vec::new(),
        ));
        Self {
            identity: RandomExecutionIdentity,
            settings,
            telemetry,
            contexts: Mutex::new(std::collections::BTreeMap::new()),
        }
    }
}

impl McpTelemetryPort for NativeMcpTelemetryAdapter {
    fn start_connection_test(
        &self,
        operation_id: &str,
        server_name: &str,
        transport: TransportType,
        started_at: &str,
    ) -> Result<String, McpApplicationError> {
        let settings = self.settings.load_settings().map_err(|_| {
            McpApplicationError::Storage("observability settings unavailable".to_string())
        })?;
        let context = self.identity.next_context(
            settings.capture_policy,
            settings.sampling_ratio,
            settings.mcp_relay_enabled,
        );
        let attributes = SafeAttributes::try_from_entries([
            (
                "rpc.system".to_string(),
                SafeAttributeValue::String("mcp".to_string()),
            ),
            (
                "rpc.method".to_string(),
                SafeAttributeValue::String("tools/list".to_string()),
            ),
            (
                "network.transport".to_string(),
                SafeAttributeValue::String(transport.as_str().to_string()),
            ),
            (
                "mcp.server.name".to_string(),
                SafeAttributeValue::String(server_name.to_string()),
            ),
        ])
        .unwrap_or_default();
        let run = ExecutionRun {
            context: context.clone(),
            source: ExecutionSource::Desktop,
            status: ExecutionStatus::Running,
            started_at: started_at.to_string(),
            ended_at: None,
            error_classification: None,
            session_id: None,
            user_message_id: None,
            assistant_message_id: None,
            operation_id: Some(operation_id.to_string()),
            agent_id: None,
            provider_session_id: None,
            attributes: attributes.clone(),
            links: Vec::new(),
        };
        self.telemetry
            .start_run(&run)
            .and_then(|()| {
                self.telemetry.start_span(&ExecutionSpan {
                    context: context.clone(),
                    parent_span_id: None,
                    name: "mcp.connection.test".to_string(),
                    status: ExecutionStatus::Running,
                    fidelity: ExecutionFidelity::Native,
                    started_at: started_at.to_string(),
                    ended_at: None,
                    error_classification: None,
                    attributes,
                    links: Vec::new(),
                })
            })
            .map_err(|_| McpApplicationError::Storage("MCP telemetry unavailable".to_string()))?;
        let observation_id = context.run_id.as_str().to_string();
        self.contexts
            .lock()
            .map_err(|_| {
                McpApplicationError::Storage("MCP telemetry state unavailable".to_string())
            })?
            .insert(observation_id.clone(), context);
        Ok(observation_id)
    }

    fn finish_connection_test(
        &self,
        observation_id: &str,
        outcome: &ConnectionOutcome,
        ended_at: &str,
    ) -> Result<(), McpApplicationError> {
        let context = self
            .contexts
            .lock()
            .map_err(|_| {
                McpApplicationError::Storage("MCP telemetry state unavailable".to_string())
            })?
            .remove(observation_id)
            .ok_or_else(|| McpApplicationError::Storage("MCP observation not found".to_string()))?;
        let (status, classification) = if outcome.is_success() {
            (ExecutionStatus::Succeeded, None)
        } else {
            (ExecutionStatus::Failed, Some("mcp_connection_failed"))
        };
        self.telemetry
            .finish_span(
                &context.run_id,
                &context.span_id,
                status,
                ended_at,
                classification,
            )
            .and_then(|()| {
                self.telemetry
                    .finish_run(&context.run_id, status, ended_at, classification)
            })
            .map_err(|_| McpApplicationError::Storage("MCP telemetry unavailable".to_string()))
    }
}

pub(crate) fn assemble_mcp_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> McpApi {
    let telemetry = Arc::new(NativeMcpTelemetryAdapter::new(database.clone()));
    let operation_logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    McpApi::new(McpApplicationService::new(
        Arc::new(SqliteMcpServerRepository::new(database)),
        Arc::new(RmcpConnectionAdapter::default()),
        Arc::new(McpOperationAdapter::new(operations)),
        Arc::new(SystemMcpClock),
        Arc::new(UnifiedMcpLoggingAdapter::new(operation_logging)),
        Arc::new(CurrentProjectPathAdapter),
        telemetry,
    ))
}
