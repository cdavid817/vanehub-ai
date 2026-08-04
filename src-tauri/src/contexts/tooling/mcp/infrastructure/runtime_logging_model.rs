use crate::contexts::tooling::mcp::domain::{McpFailureCode, ServerConfiguration, TransportType};
use crate::platform::logging::LogLevel;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone)]
pub(crate) struct McpRuntimeLogContext {
    pub(super) server_name: String,
    pub(super) transport: String,
    pub(super) operation_id: Option<String>,
    pub(super) run_id: Option<String>,
    pub(super) trace_id: Option<String>,
    pub(super) span_id: Option<String>,
    pub(super) executable_class: Option<&'static str>,
    pub(super) argument_count: Option<usize>,
}

impl McpRuntimeLogContext {
    pub(crate) fn for_server(server: &ServerConfiguration, operation_id: Option<&str>) -> Self {
        let (executable_class, argument_count) = if server.transport_type() == TransportType::Stdio
        {
            (
                server.command().map(classify_executable),
                Some(server.args().map_or(0, <[String]>::len)),
            )
        } else {
            (None, None)
        };
        Self {
            server_name: server.name().as_str().to_string(),
            transport: server.transport_type().as_str().to_string(),
            operation_id: operation_id.map(str::to_string),
            run_id: None,
            trace_id: None,
            span_id: None,
            executable_class,
            argument_count,
        }
    }

    pub(crate) fn for_relay(
        transport: &'static str,
        run_id: Option<&str>,
        trace_id: Option<&str>,
        span_id: Option<&str>,
    ) -> Self {
        Self {
            server_name: "managed-relay".to_string(),
            transport: transport.to_string(),
            operation_id: None,
            run_id: run_id.map(str::to_string),
            trace_id: trace_id.map(str::to_string),
            span_id: span_id.map(str::to_string),
            executable_class: None,
            argument_count: None,
        }
    }

    pub(crate) fn with_command(mut self, executable: &str, argument_count: usize) -> Self {
        self.executable_class = Some(classify_executable(executable));
        self.argument_count = Some(argument_count);
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RuntimePhase {
    Command,
    Spawn,
    Protocol,
    Exit,
    Cleanup,
    Relay,
}

impl RuntimePhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Command => "command",
            Self::Spawn => "spawn",
            Self::Protocol => "protocol",
            Self::Exit => "exit",
            Self::Cleanup => "cleanup",
            Self::Relay => "relay",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum RuntimeOutcome {
    Started,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Terminated,
}

impl RuntimeOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Started => "started",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::Cancelled => "cancelled",
            Self::Terminated => "terminated",
        }
    }
}

pub(super) struct RuntimeDiagnostic<'a> {
    pub(super) context: &'a McpRuntimeLogContext,
    pub(super) phase: RuntimePhase,
    pub(super) outcome: RuntimeOutcome,
    pub(super) error_code: Option<McpFailureCode>,
    pub(super) duration: Option<Duration>,
    pub(super) stderr_observed_bytes: Option<u64>,
    pub(super) stderr_truncated: Option<bool>,
    pub(super) exit_code: Option<i32>,
}

pub(super) fn render(
    diagnostic: &RuntimeDiagnostic<'_>,
) -> (LogLevel, &'static str, BTreeMap<String, String>) {
    let mut context = BTreeMap::from([
        (
            "serverName".to_string(),
            diagnostic.context.server_name.clone(),
        ),
        (
            "transport".to_string(),
            diagnostic.context.transport.clone(),
        ),
        ("phase".to_string(), diagnostic.phase.as_str().to_string()),
        (
            "outcome".to_string(),
            diagnostic.outcome.as_str().to_string(),
        ),
    ]);
    insert_optional(
        &mut context,
        "operationId",
        diagnostic.context.operation_id.as_deref(),
    );
    insert_optional(&mut context, "runId", diagnostic.context.run_id.as_deref());
    insert_optional(
        &mut context,
        "traceId",
        diagnostic.context.trace_id.as_deref(),
    );
    insert_optional(
        &mut context,
        "spanId",
        diagnostic.context.span_id.as_deref(),
    );
    insert_optional(
        &mut context,
        "executableClass",
        diagnostic.context.executable_class,
    );
    if let Some(count) = diagnostic.context.argument_count {
        context.insert("argumentCount".to_string(), count.to_string());
    }
    if let Some(code) = diagnostic.error_code {
        context.insert("errorCode".to_string(), code.as_str().to_string());
    }
    if let Some(duration) = diagnostic.duration {
        context.insert("durationMs".to_string(), duration.as_millis().to_string());
    }
    if let Some(bytes) = diagnostic.stderr_observed_bytes {
        context.insert("stderrObservedBytes".to_string(), bytes.to_string());
    }
    if let Some(truncated) = diagnostic.stderr_truncated {
        context.insert("stderrTruncated".to_string(), truncated.to_string());
    }
    if let Some(exit_code) = diagnostic.exit_code {
        context.insert("exitCode".to_string(), exit_code.to_string());
    }
    let failed = matches!(
        diagnostic.outcome,
        RuntimeOutcome::Failed | RuntimeOutcome::TimedOut
    );
    (
        if failed {
            LogLevel::Warn
        } else {
            LogLevel::Info
        },
        "MCP runtime lifecycle event",
        context,
    )
}

fn insert_optional(context: &mut BTreeMap<String, String>, key: &str, value: Option<&str>) {
    if let Some(value) = value {
        context.insert(key.to_string(), value.to_string());
    }
}

fn classify_executable(executable: &str) -> &'static str {
    let path = PathBuf::from(executable);
    if path.is_absolute() {
        "absolute_path"
    } else if executable.contains(['/', '\\']) {
        "relative_path"
    } else {
        "path_lookup"
    }
}
