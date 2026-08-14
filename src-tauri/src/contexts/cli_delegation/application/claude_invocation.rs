#![allow(clippy::enum_variant_names)]

use super::DelegationMode;
use std::path::{Path, PathBuf};
use uuid::Uuid;

const MAX_SCHEMA_BYTES: usize = 64 * 1024;
const MAX_PROMPT_BYTES: usize = 384 * 1024;
const MAX_TURNS: u16 = 64;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ClaudeInvocationProfile {
    pub(crate) model: Option<String>,
    pub(crate) effort: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClaudeDelegationInvocation {
    pub(crate) executable: PathBuf,
    pub(crate) working_directory: PathBuf,
    pub(crate) args: Vec<String>,
    pub(crate) stdin: Vec<u8>,
    pub(crate) session_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaudeInvocationError {
    InvalidExecutable,
    InvalidWorkspace,
    InvalidControllerFile,
    InvalidPrompt,
    InvalidSchema,
    InvalidLimits,
    InvalidProfile,
}

pub(crate) struct ClaudeDelegationInvocationBuilder;

impl ClaudeDelegationInvocationBuilder {
    pub(crate) fn build(
        request: ClaudeInvocationRequest<'_>,
    ) -> Result<ClaudeDelegationInvocation, ClaudeInvocationError> {
        let ClaudeInvocationRequest {
            executable,
            workspace,
            settings_file,
            empty_mcp_config,
            task_prompt,
            schema_json,
            mode,
            maximum_turns,
            maximum_budget_microusd,
            profile,
        } = request;
        validate_inputs(
            &executable,
            &workspace,
            settings_file,
            empty_mcp_config,
            task_prompt,
            schema_json,
            maximum_turns,
            &profile,
        )?;
        let session_id = Uuid::new_v4().to_string();
        let mut args = fixed_args(
            &session_id,
            settings_file,
            empty_mcp_config,
            schema_json,
            mode,
            maximum_turns,
        );
        if let Some(budget) = maximum_budget_microusd {
            if budget == 0 {
                return Err(ClaudeInvocationError::InvalidLimits);
            }
            args.extend(["--max-budget-usd".to_string(), format_usd_micros(budget)]);
        }
        if let Some(model) = profile.model {
            args.extend(["--model".to_string(), model]);
        }
        if let Some(effort) = profile.effort {
            args.extend(["--effort".to_string(), effort]);
        }
        Ok(ClaudeDelegationInvocation {
            executable,
            working_directory: workspace,
            args,
            stdin: task_prompt.as_bytes().to_vec(),
            session_id,
        })
    }
}

pub(crate) struct ClaudeInvocationRequest<'a> {
    pub(crate) executable: PathBuf,
    pub(crate) workspace: PathBuf,
    pub(crate) settings_file: &'a Path,
    pub(crate) empty_mcp_config: &'a Path,
    pub(crate) task_prompt: &'a str,
    pub(crate) schema_json: &'a str,
    pub(crate) mode: DelegationMode,
    pub(crate) maximum_turns: u16,
    pub(crate) maximum_budget_microusd: Option<u64>,
    pub(crate) profile: ClaudeInvocationProfile,
}

fn fixed_args(
    session_id: &str,
    settings_file: &Path,
    empty_mcp_config: &Path,
    schema_json: &str,
    mode: DelegationMode,
    maximum_turns: u16,
) -> Vec<String> {
    let (permission_mode, tools) = match mode {
        DelegationMode::Analyze => ("plan", "Read,Glob,Grep"),
        DelegationMode::Edit => ("acceptEdits", "Read,Glob,Grep,Edit,Write"),
    };
    vec![
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--include-partial-messages".into(),
        "--verbose".into(),
        "--session-id".into(),
        session_id.into(),
        "--no-session-persistence".into(),
        "--no-chrome".into(),
        "--safe-mode".into(),
        "--disable-slash-commands".into(),
        "--setting-sources".into(),
        "".into(),
        "--settings".into(),
        settings_file.to_string_lossy().into_owned(),
        "--strict-mcp-config".into(),
        "--mcp-config".into(),
        empty_mcp_config.to_string_lossy().into_owned(),
        "--permission-mode".into(),
        permission_mode.into(),
        "--tools".into(),
        tools.into(),
        "--json-schema".into(),
        schema_json.into(),
        "--max-turns".into(),
        maximum_turns.to_string(),
    ]
}

#[allow(clippy::too_many_arguments)]
fn validate_inputs(
    executable: &Path,
    workspace: &Path,
    settings_file: &Path,
    empty_mcp_config: &Path,
    prompt: &str,
    schema: &str,
    maximum_turns: u16,
    profile: &ClaudeInvocationProfile,
) -> Result<(), ClaudeInvocationError> {
    if !executable.is_absolute() {
        return Err(ClaudeInvocationError::InvalidExecutable);
    }
    if !workspace.is_absolute() {
        return Err(ClaudeInvocationError::InvalidWorkspace);
    }
    if !settings_file.is_absolute() || !empty_mcp_config.is_absolute() {
        return Err(ClaudeInvocationError::InvalidControllerFile);
    }
    if prompt.trim().is_empty() || prompt.len() > MAX_PROMPT_BYTES || prompt.contains('\0') {
        return Err(ClaudeInvocationError::InvalidPrompt);
    }
    if schema.len() > MAX_SCHEMA_BYTES || serde_json::from_str::<serde_json::Value>(schema).is_err()
    {
        return Err(ClaudeInvocationError::InvalidSchema);
    }
    if maximum_turns == 0 || maximum_turns > MAX_TURNS {
        return Err(ClaudeInvocationError::InvalidLimits);
    }
    if !profile.model.as_deref().is_none_or(valid_profile_value)
        || !profile.effort.as_deref().is_none_or(valid_effort)
    {
        return Err(ClaudeInvocationError::InvalidProfile);
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
    matches!(value, "low" | "medium" | "high" | "xhigh" | "max")
}

fn format_usd_micros(value: u64) -> String {
    format!("{}.{:06}", value / 1_000_000, value % 1_000_000)
}

#[cfg(test)]
#[path = "claude_invocation_tests.rs"]
mod tests;
