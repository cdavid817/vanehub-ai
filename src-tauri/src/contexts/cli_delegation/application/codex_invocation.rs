#![allow(clippy::enum_variant_names)]

use super::DelegationMode;
use std::path::{Path, PathBuf};

const MAX_PROMPT_BYTES: usize = 384 * 1024;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct CodexInvocationProfile {
    pub(crate) model: Option<String>,
    pub(crate) reasoning_effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodexDelegationInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) working_directory: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) stdin: Vec<u8>,
    pub(crate) final_output: PathBuf,
}

pub(crate) struct CodexInvocationRequest<'a> {
    pub(crate) executable: PathBuf,
    pub(crate) workspace: PathBuf,
    pub(crate) schema_file: &'a Path,
    pub(crate) final_output: &'a Path,
    pub(crate) task_prompt: &'a str,
    pub(crate) mode: DelegationMode,
    pub(crate) profile: CodexInvocationProfile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodexInvocationError {
    InvalidExecutable,
    InvalidWorkspace,
    InvalidControllerFile,
    InvalidPrompt,
    InvalidProfile,
}

pub(crate) struct CodexDelegationInvocationBuilder;

impl CodexDelegationInvocationBuilder {
    pub(crate) fn build(
        request: CodexInvocationRequest<'_>,
    ) -> Result<CodexDelegationInvocation, CodexInvocationError> {
        validate(&request)?;
        let sandbox = match request.mode {
            DelegationMode::Analyze => "read-only",
            DelegationMode::Edit => "workspace-write",
        };
        let mut args = vec![
            "exec".into(),
            "--json".into(),
            "--ephemeral".into(),
            "--ignore-user-config".into(),
            "--ignore-rules".into(),
            "--strict-config".into(),
            "--sandbox".into(),
            sandbox.into(),
            "-c".into(),
            "approval_policy=\"never\"".into(),
            "-c".into(),
            "mcp_servers={}".into(),
            "-c".into(),
            "features.hooks=false".into(),
            "--output-schema".into(),
            request.schema_file.to_string_lossy().into_owned(),
            "--output-last-message".into(),
            request.final_output.to_string_lossy().into_owned(),
        ];
        if let Some(model) = request.profile.model {
            args.extend(["--model".into(), model]);
        }
        if let Some(effort) = request.profile.reasoning_effort {
            args.extend(["-c".into(), format!("model_reasoning_effort=\"{effort}\"")]);
        }
        args.push("-".into());
        Ok(CodexDelegationInvocation {
            executable: request.executable,
            working_directory: request.workspace,
            args,
            stdin: request.task_prompt.as_bytes().to_vec(),
            final_output: request.final_output.to_path_buf(),
        })
    }
}

fn validate(request: &CodexInvocationRequest<'_>) -> Result<(), CodexInvocationError> {
    if !request.executable.is_absolute() {
        return Err(CodexInvocationError::InvalidExecutable);
    }
    if !request.workspace.is_absolute() {
        return Err(CodexInvocationError::InvalidWorkspace);
    }
    if !request.schema_file.is_absolute() || !request.final_output.is_absolute() {
        return Err(CodexInvocationError::InvalidControllerFile);
    }
    if request.task_prompt.trim().is_empty()
        || request.task_prompt.len() > MAX_PROMPT_BYTES
        || request.task_prompt.contains('\0')
    {
        return Err(CodexInvocationError::InvalidPrompt);
    }
    if !request
        .profile
        .model
        .as_deref()
        .is_none_or(valid_profile_value)
        || !request
            .profile
            .reasoning_effort
            .as_deref()
            .is_none_or(valid_effort)
    {
        return Err(CodexInvocationError::InvalidProfile);
    }
    Ok(())
}

fn valid_profile_value(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b':'))
}

fn valid_effort(value: &str) -> bool {
    matches!(value, "low" | "medium" | "high" | "xhigh")
}

#[cfg(test)]
#[path = "codex_invocation_tests.rs"]
mod tests;
