use super::{
    CodeExecutionResult, CodeExecutionStatus, CodeRuntime, CodeServiceError, SandboxBackendError,
    CODE_EXECUTION_CONTRACT_VERSION,
};
use std::collections::BTreeMap;
use std::path::Path;
use std::time::Instant;

pub(super) fn minimal_environment(runtime: CodeRuntime) -> BTreeMap<String, String> {
    let mut environment = BTreeMap::from([
        ("LANG".to_owned(), "C.UTF-8".to_owned()),
        ("NO_PROXY".to_owned(), "*".to_owned()),
    ]);
    if runtime == CodeRuntime::Python {
        environment.insert("PYTHONDONTWRITEBYTECODE".to_owned(), "1".to_owned());
        environment.insert("PYTHONNOUSERSITE".to_owned(), "1".to_owned());
    }
    #[cfg(windows)]
    add_windows_environment(&mut environment);
    environment
}

#[cfg(windows)]
fn add_windows_environment(environment: &mut BTreeMap<String, String>) {
    let system_root = std::env::var("SystemRoot")
        .or_else(|_| std::env::var("WINDIR"))
        .unwrap_or_else(|_| "C:\\Windows".to_owned());
    environment.insert("SYSTEMROOT".to_owned(), system_root.clone());
    environment.insert("WINDIR".to_owned(), system_root.clone());
    environment.insert(
        "SYSTEMDRIVE".to_owned(),
        std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_owned()),
    );
    environment.insert("PATH".to_owned(), format!("{system_root}\\System32"));
    environment.insert(
        "COMSPEC".to_owned(),
        format!("{system_root}\\System32\\cmd.exe"),
    );
    environment.insert("PATHEXT".to_owned(), ".COM;.EXE;.BAT;.CMD".to_owned());
    for key in ["LOCALAPPDATA", "TEMP", "TMP"] {
        if let Ok(value) = std::env::var(key) {
            environment.insert(key.to_owned(), value);
        }
    }
}

pub(super) fn bounded_text(bytes: Vec<u8>, limit: u64) -> (String, bool) {
    let limit = usize::try_from(limit).unwrap_or(usize::MAX);
    let truncated = bytes.len() > limit;
    let admitted = &bytes[..bytes.len().min(limit)];
    (String::from_utf8_lossy(admitted).into_owned(), truncated)
}

pub(super) fn safe_output_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !matches!(name, "." | "..")
        && !name.contains(['/', '\\', ':', '\0'])
}

pub(super) fn output_media_type(
    name: &str,
    bytes: &[u8],
) -> Result<&'static str, CodeServiceError> {
    let extension = Path::new(name)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    match extension.as_str() {
        "txt" => Ok("text/plain"),
        "csv" => Ok("text/csv"),
        "json" if serde_json::from_slice::<serde_json::Value>(bytes).is_ok() => {
            Ok("application/json")
        }
        "png" if bytes.starts_with(b"\x89PNG\r\n\x1a\n") => Ok("image/png"),
        "jpg" | "jpeg" if bytes.starts_with(&[0xff, 0xd8, 0xff]) => Ok("image/jpeg"),
        _ => Err(CodeServiceError::OutputRejected),
    }
}

pub(super) fn failed_result(
    execution_id: &str,
    started_at: Instant,
    status: CodeExecutionStatus,
    message: &str,
) -> CodeExecutionResult {
    CodeExecutionResult {
        contract_version: CODE_EXECUTION_CONTRACT_VERSION,
        execution_id: execution_id.to_owned(),
        status,
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        stdout_truncated: false,
        stderr_truncated: false,
        duration_ms: elapsed_ms(started_at),
        limit_reason: None,
        outputs: Vec::new(),
        safe_error: Some(message.to_owned()),
    }
}

pub(super) fn elapsed_ms(started_at: Instant) -> u64 {
    u64::try_from(started_at.elapsed().as_millis()).unwrap_or(u64::MAX)
}

pub(super) fn map_backend_launch(error: SandboxBackendError) -> CodeServiceError {
    match error {
        SandboxBackendError::IsolationUnavailable => CodeServiceError::IsolationUnavailable,
        SandboxBackendError::InvalidLaunch => CodeServiceError::InvalidRequest,
        #[cfg(any(windows, test))]
        SandboxBackendError::SpawnFailed => CodeServiceError::SpawnFailure,
        #[cfg(windows)]
        SandboxBackendError::JobSetupFailed
        | SandboxBackendError::AclSetupFailed
        | SandboxBackendError::ProcessCreationFailed(_)
        | SandboxBackendError::JobAssignmentFailed
        | SandboxBackendError::ResumeFailed
        | SandboxBackendError::WaitFailed
        | SandboxBackendError::TerminationFailed => CodeServiceError::SpawnFailure,
    }
}

pub(super) fn limited_result(
    execution_id: &str,
    started_at: Instant,
    status: CodeExecutionStatus,
    reason: &str,
    safe_error: &str,
) -> CodeExecutionResult {
    let mut result = failed_result(execution_id, started_at, status, safe_error);
    result.limit_reason = Some(reason.to_owned());
    result
}
