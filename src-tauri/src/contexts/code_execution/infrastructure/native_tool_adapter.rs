use crate::contexts::agent_runtime::application::{
    CodeExecutionPort, NativeToolErrorCode, NativeToolPortRequest, NativeToolResultEnvelope,
    NativeToolResultStatus, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::code_execution::application::{
    CodeExecutionLimits, CodeExecutionRequest, CodeExecutionResult, CodeExecutionService,
    CodeExecutionStatus, CodeInputArtifact, CodeRuntime, CodeServiceError,
    CODE_EXECUTION_CONTRACT_VERSION,
};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct CodeExecutionNativeToolAdapter {
    service: Arc<CodeExecutionService>,
}

impl CodeExecutionNativeToolAdapter {
    pub(crate) fn new(service: Arc<CodeExecutionService>) -> Self {
        Self { service }
    }

    fn execute(
        &self,
        request: NativeToolPortRequest,
    ) -> Result<CodeExecutionResult, CodeServiceError> {
        let input = request
            .input
            .value
            .as_object()
            .ok_or(CodeServiceError::InvalidRequest)?;
        let runtime = match string(input, "runtime")? {
            "python" => CodeRuntime::Python,
            "javascript" => CodeRuntime::JavaScript,
            _ => return Err(CodeServiceError::InvalidRequest),
        };
        let arguments = input
            .get("arguments")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .map(|value| {
                        value
                            .as_str()
                            .map(str::to_owned)
                            .ok_or(CodeServiceError::InvalidRequest)
                    })
                    .collect::<Result<Vec<_>, _>>()
            })
            .transpose()?
            .unwrap_or_default();
        let inputs = parse_inputs(input)?;
        self.service.execute(
            CodeExecutionRequest {
                contract_version: CODE_EXECUTION_CONTRACT_VERSION,
                execution_id: request.context.call_id,
                runtime,
                source: string(input, "source")?.to_owned(),
                arguments,
                inputs,
                requested_limits: parse_limits(input.get("limits"))?,
            },
            request.context.cancelled,
        )
    }
}

impl CodeExecutionPort for CodeExecutionNativeToolAdapter {
    fn execute_code(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        match self.execute(request) {
            Ok(result) => result_envelope(result),
            Err(error) => error_envelope(error),
        }
    }
}

fn parse_inputs(input: &Map<String, Value>) -> Result<Vec<CodeInputArtifact>, CodeServiceError> {
    input
        .get("inputs")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(|value| {
                    let value = value.as_object().ok_or(CodeServiceError::InvalidRequest)?;
                    Ok(CodeInputArtifact {
                        artifact_id: string(value, "artifact_id")?.to_owned(),
                        content_hash: string(value, "content_hash")?.to_owned(),
                    })
                })
                .collect::<Result<Vec<_>, CodeServiceError>>()
        })
        .transpose()
        .map(Option::unwrap_or_default)
}

fn parse_limits(value: Option<&Value>) -> Result<Option<CodeExecutionLimits>, CodeServiceError> {
    let Some(value) = value else {
        return Ok(None);
    };
    let object = value.as_object().ok_or(CodeServiceError::InvalidRequest)?;
    let defaults = CodeExecutionLimits::HARD_CEILING;
    Ok(Some(CodeExecutionLimits {
        wall_time_ms: number(object, "wall_time_ms", defaults.wall_time_ms)?,
        cpu_time_ms: number(object, "cpu_time_ms", defaults.cpu_time_ms)?,
        memory_bytes: number(object, "memory_bytes", defaults.memory_bytes)?,
        process_count: u32_number(object, "process_count", defaults.process_count)?,
        stdout_bytes: number(object, "stdout_bytes", defaults.stdout_bytes)?,
        stderr_bytes: number(object, "stderr_bytes", defaults.stderr_bytes)?,
        filesystem_bytes: number(object, "filesystem_bytes", defaults.filesystem_bytes)?,
        file_count: u32_number(object, "file_count", defaults.file_count)?,
        event_count: u32_number(object, "event_count", defaults.event_count)?,
    }))
}

fn string<'a>(input: &'a Map<String, Value>, name: &str) -> Result<&'a str, CodeServiceError> {
    input
        .get(name)
        .and_then(Value::as_str)
        .ok_or(CodeServiceError::InvalidRequest)
}

fn number(input: &Map<String, Value>, name: &str, default: u64) -> Result<u64, CodeServiceError> {
    match input.get(name) {
        Some(value) => value.as_u64().ok_or(CodeServiceError::InvalidRequest),
        None => Ok(default),
    }
}

fn u32_number(
    input: &Map<String, Value>,
    name: &str,
    default: u32,
) -> Result<u32, CodeServiceError> {
    u32::try_from(number(input, name, u64::from(default))?)
        .map_err(|_| CodeServiceError::InvalidRequest)
}

fn result_envelope(result: CodeExecutionResult) -> NativeToolResultEnvelope {
    let status = match result.status {
        CodeExecutionStatus::Succeeded => NativeToolResultStatus::Succeeded,
        CodeExecutionStatus::Cancelled => NativeToolResultStatus::Cancelled,
        CodeExecutionStatus::LimitExceeded | CodeExecutionStatus::SandboxViolation => {
            NativeToolResultStatus::LimitExceeded
        }
        CodeExecutionStatus::Failed
        | CodeExecutionStatus::TimedOut
        | CodeExecutionStatus::CleanupFailed => NativeToolResultStatus::Failed,
    };
    let outputs = result
        .outputs
        .into_iter()
        .map(|output| {
            json!({
                "artifact_id": output.artifact_id,
                "content_hash": output.content_hash,
                "relative_name": output.relative_name,
                "size_bytes": output.size_bytes,
                "media_type": output.media_type,
            })
        })
        .collect::<Vec<_>>();
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output: Some(json!({
            "contract_version": result.contract_version,
            "execution_id": result.execution_id,
            "status": status_name(result.status),
            "exit_code": result.exit_code,
            "stdout": result.stdout,
            "stderr": result.stderr,
            "stdout_truncated": result.stdout_truncated,
            "stderr_truncated": result.stderr_truncated,
            "duration_ms": result.duration_ms,
            "limit_reason": result.limit_reason,
            "outputs": outputs,
            "safe_error": result.safe_error,
        })),
        error_code: result_error_code(result.status),
        safe_error: None,
        truncated: result.stdout_truncated || result.stderr_truncated,
        metadata: BTreeMap::new(),
    }
}

const fn status_name(status: CodeExecutionStatus) -> &'static str {
    match status {
        CodeExecutionStatus::Succeeded => "succeeded",
        CodeExecutionStatus::Failed => "failed",
        CodeExecutionStatus::Cancelled => "cancelled",
        CodeExecutionStatus::TimedOut => "timed_out",
        CodeExecutionStatus::LimitExceeded => "limit_exceeded",
        CodeExecutionStatus::SandboxViolation => "sandbox_violation",
        CodeExecutionStatus::CleanupFailed => "cleanup_failed",
    }
}

const fn result_error_code(status: CodeExecutionStatus) -> Option<NativeToolErrorCode> {
    match status {
        CodeExecutionStatus::Succeeded => None,
        CodeExecutionStatus::Cancelled => Some(NativeToolErrorCode::Cancelled),
        CodeExecutionStatus::TimedOut => Some(NativeToolErrorCode::DeadlineExceeded),
        CodeExecutionStatus::LimitExceeded | CodeExecutionStatus::SandboxViolation => {
            Some(NativeToolErrorCode::LimitExceeded)
        }
        CodeExecutionStatus::Failed | CodeExecutionStatus::CleanupFailed => {
            Some(NativeToolErrorCode::ExternalFailure)
        }
    }
}

fn error_envelope(error: CodeServiceError) -> NativeToolResultEnvelope {
    let (status, code, message) = match error {
        CodeServiceError::InvalidRequest => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InvalidInput,
            "Code execution input is invalid.",
        ),
        CodeServiceError::IsolationUnavailable | CodeServiceError::RuntimeUnavailable => (
            NativeToolResultStatus::Unavailable,
            NativeToolErrorCode::Unavailable,
            "Code execution runtime is unavailable.",
        ),
        CodeServiceError::OutputRejected | CodeServiceError::ArtifactFailure => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::IntegrityFailure,
            "Code execution output could not be admitted safely.",
        ),
        CodeServiceError::WorkspaceFailure | CodeServiceError::WaitFailure => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InternalFailure,
            "Code execution failed in the isolated runtime.",
        ),
        #[cfg(any(windows, test))]
        CodeServiceError::SpawnFailure => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InternalFailure,
            "Code execution failed in the isolated runtime.",
        ),
    };
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output: None,
        error_code: Some(code),
        safe_error: Some(message.to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}
