#![allow(clippy::enum_variant_names)]

use super::CodeRuntime;

pub(crate) const CODE_EXECUTION_CONTRACT_VERSION: u16 = 1;
const MAX_SOURCE_BYTES: usize = 128 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeInputArtifact {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeExecutionRequest {
    pub(crate) contract_version: u16,
    pub(crate) execution_id: String,
    pub(crate) runtime: CodeRuntime,
    pub(crate) source: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) inputs: Vec<CodeInputArtifact>,
    pub(crate) requested_limits: Option<CodeExecutionLimits>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CodeExecutionLimits {
    pub(crate) wall_time_ms: u64,
    pub(crate) cpu_time_ms: u64,
    pub(crate) memory_bytes: u64,
    pub(crate) process_count: u32,
    pub(crate) stdout_bytes: u64,
    pub(crate) stderr_bytes: u64,
    pub(crate) filesystem_bytes: u64,
    pub(crate) file_count: u32,
    pub(crate) event_count: u32,
}

impl CodeExecutionLimits {
    pub(crate) const HARD_CEILING: Self = Self {
        wall_time_ms: 30_000,
        cpu_time_ms: 20_000,
        memory_bytes: 256 * 1024 * 1024,
        process_count: 2,
        stdout_bytes: 1024 * 1024,
        stderr_bytes: 1024 * 1024,
        filesystem_bytes: 64 * 1024 * 1024,
        file_count: 64,
        event_count: 200,
    };

    pub(crate) fn effective(requested: Option<Self>) -> Result<Self, CodeContractError> {
        let value = requested.unwrap_or(Self::HARD_CEILING);
        if value.any_zero() || !value.within(Self::HARD_CEILING) {
            return Err(CodeContractError::InvalidLimits);
        }
        Ok(value)
    }

    fn any_zero(self) -> bool {
        self.wall_time_ms == 0
            || self.cpu_time_ms == 0
            || self.memory_bytes == 0
            || self.process_count == 0
            || self.stdout_bytes == 0
            || self.stderr_bytes == 0
            || self.filesystem_bytes == 0
            || self.file_count == 0
            || self.event_count == 0
    }

    fn within(self, ceiling: Self) -> bool {
        self.wall_time_ms <= ceiling.wall_time_ms
            && self.cpu_time_ms <= ceiling.cpu_time_ms
            && self.memory_bytes <= ceiling.memory_bytes
            && self.process_count <= ceiling.process_count
            && self.stdout_bytes <= ceiling.stdout_bytes
            && self.stderr_bytes <= ceiling.stderr_bytes
            && self.filesystem_bytes <= ceiling.filesystem_bytes
            && self.file_count <= ceiling.file_count
            && self.event_count <= ceiling.event_count
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeExecutionStatus {
    Succeeded,
    Failed,
    Cancelled,
    TimedOut,
    LimitExceeded,
    SandboxViolation,
    CleanupFailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeOutputArtifact {
    pub(crate) artifact_id: String,
    pub(crate) content_hash: String,
    pub(crate) relative_name: String,
    pub(crate) size_bytes: u64,
    pub(crate) media_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeExecutionResult {
    pub(crate) contract_version: u16,
    pub(crate) execution_id: String,
    pub(crate) status: CodeExecutionStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) stdout_truncated: bool,
    pub(crate) stderr_truncated: bool,
    pub(crate) duration_ms: u64,
    pub(crate) limit_reason: Option<String>,
    pub(crate) outputs: Vec<CodeOutputArtifact>,
    pub(crate) safe_error: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeContractError {
    InvalidVersion,
    InvalidIdentity,
    InvalidSource,
    InvalidArguments,
    InvalidInputs,
    InvalidLimits,
}

impl CodeExecutionRequest {
    pub(crate) fn validate(&self) -> Result<CodeExecutionLimits, CodeContractError> {
        if self.contract_version != CODE_EXECUTION_CONTRACT_VERSION {
            return Err(CodeContractError::InvalidVersion);
        }
        if !valid_id(&self.execution_id) {
            return Err(CodeContractError::InvalidIdentity);
        }
        if self.source.is_empty()
            || self.source.len() > MAX_SOURCE_BYTES
            || self.source.contains('\0')
        {
            return Err(CodeContractError::InvalidSource);
        }
        if self.arguments.len() > 16
            || self.arguments.iter().any(|argument| {
                argument.is_empty() || argument.len() > 256 || argument.contains('\0')
            })
        {
            return Err(CodeContractError::InvalidArguments);
        }
        if self.inputs.len() > 16
            || self.inputs.iter().any(|input| {
                !valid_id(&input.artifact_id)
                    || input.content_hash.len() != 64
                    || !input
                        .content_hash
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit())
            })
        {
            return Err(CodeContractError::InvalidInputs);
        }
        CodeExecutionLimits::effective(self.requested_limits)
    }
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> CodeExecutionRequest {
        CodeExecutionRequest {
            contract_version: CODE_EXECUTION_CONTRACT_VERSION,
            execution_id: "execution-1".to_owned(),
            runtime: CodeRuntime::Python,
            source: "print('ok')".to_owned(),
            arguments: vec!["input".to_owned()],
            inputs: vec![CodeInputArtifact {
                artifact_id: "artifact-1".to_owned(),
                content_hash: "a".repeat(64),
            }],
            requested_limits: None,
        }
    }

    #[test]
    fn controller_ceiling_is_the_default_and_call_can_only_lower_it() {
        assert_eq!(request().validate(), Ok(CodeExecutionLimits::HARD_CEILING));
        let mut lowered = CodeExecutionLimits::HARD_CEILING;
        lowered.wall_time_ms = 1_000;
        let mut request = request();
        request.requested_limits = Some(lowered);
        assert_eq!(request.validate(), Ok(lowered));
        let mut raised = lowered;
        raised.memory_bytes = CodeExecutionLimits::HARD_CEILING.memory_bytes + 1;
        request.requested_limits = Some(raised);
        assert_eq!(request.validate(), Err(CodeContractError::InvalidLimits));
    }

    #[test]
    fn source_arguments_inputs_and_identity_are_bounded() {
        let mut invalid = request();
        invalid.source = "x".repeat(MAX_SOURCE_BYTES + 1);
        assert_eq!(invalid.validate(), Err(CodeContractError::InvalidSource));
        let mut invalid = request();
        invalid.arguments = vec!["x".to_owned(); 17];
        assert_eq!(invalid.validate(), Err(CodeContractError::InvalidArguments));
        let mut invalid = request();
        invalid.inputs[0].content_hash = "not-a-hash".to_owned();
        assert_eq!(invalid.validate(), Err(CodeContractError::InvalidInputs));
    }
}
