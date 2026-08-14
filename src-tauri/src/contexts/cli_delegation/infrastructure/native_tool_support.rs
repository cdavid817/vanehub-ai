use crate::contexts::agent_runtime::application::{
    NativeToolErrorCode, NativeToolResultEnvelope, NativeToolResultStatus,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::cli_delegation::application::{
    ClaudeDelegationAdapter, DelegationAgentReportV1, DelegationArtifactInput,
    DelegationArtifactPort, DelegationMode, DelegationReportNormalizer,
};
use crate::platform::git::GitAdapter;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

pub(super) const REPORT_SCHEMA: &str = r#"{"type":"object","additionalProperties":false,"properties":{"schema_version":{"const":1},"outcome":{"enum":["completed","blocked","needs_input"]},"summary":{"type":"string"},"findings":{"type":"array","items":{"type":"string"}},"actions_taken":{"type":"array","items":{"type":"string"}},"verification_claims":{"type":"array","items":{"type":"string"}},"risks":{"type":"array","items":{"type":"string"}},"follow_ups":{"type":"array","items":{"type":"string"}},"limitations":{"type":"array","items":{"type":"string"}}},"required":["schema_version","outcome","summary","findings","actions_taken","verification_claims","risks","follow_ups","limitations"]}"#;

pub(super) struct ArtifactInputs(pub(super) Arc<ArtifactService>);

pub(super) struct DelegationInput {
    pub(super) mode: DelegationMode,
    pub(super) task: String,
    pub(super) context_summary: Option<String>,
    pub(super) artifact_ids: Vec<String>,
}

pub(super) fn parse_input(value: &Value) -> Result<DelegationInput, &'static str> {
    let object = value.as_object().ok_or("invalid_input")?;
    if object.get("target").and_then(Value::as_str) != Some("claude_code") {
        return Err("target_unavailable");
    }
    let mode = match object.get("mode").and_then(Value::as_str) {
        Some("analyze") => DelegationMode::Analyze,
        Some("edit") => DelegationMode::Edit,
        _ => return Err("invalid_input"),
    };
    let task = object
        .get("task")
        .and_then(Value::as_str)
        .filter(|task| !task.trim().is_empty())
        .ok_or("invalid_input")?
        .to_owned();
    let artifact_ids = object
        .get("artifact_ids")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .map(Value::as_str)
                .collect::<Option<Vec<_>>>()
                .ok_or("invalid_input")
                .map(|values| values.into_iter().map(str::to_owned).collect())
        })
        .transpose()?
        .unwrap_or_default();
    Ok(DelegationInput {
        mode,
        task,
        context_summary: object
            .get("context_summary")
            .and_then(Value::as_str)
            .map(str::to_owned),
        artifact_ids,
    })
}

impl DelegationArtifactPort for ArtifactInputs {
    fn read_verified(&self, id: &str) -> Result<DelegationArtifactInput, ()> {
        let artifact = self.0.metadata(id).map_err(|_| ())?;
        let size = usize::try_from(artifact.size_bytes).map_err(|_| ())?;
        let chunk = self.0.download_chunk(id, 0, size).map_err(|_| ())?;
        if chunk.next_offset.is_some() {
            return Err(());
        }
        Ok(DelegationArtifactInput {
            id: artifact.id,
            content_hash: artifact.content_hash,
            display_name: artifact.display_name,
            bytes: chunk.bytes,
        })
    }
}

pub(super) fn git_head(root: &Path) -> Result<String, &'static str> {
    let output = GitAdapter::default()
        .execute(
            root,
            &["rev-parse".to_owned(), "HEAD".to_owned()],
            Duration::from_secs(10),
        )
        .map_err(|_| "workspace_failure")?;
    if !output.status.success() {
        return Err("workspace_failure");
    }
    String::from_utf8(output.stdout)
        .map(|value| value.trim().to_owned())
        .map_err(|_| "workspace_failure")
}

pub(super) fn parse_report(
    stdout: &[u8],
    exit: i32,
) -> Result<DelegationAgentReportV1, &'static str> {
    let mut adapter = ClaudeDelegationAdapter::new();
    for line in stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        adapter
            .decode_stdout_line(line)
            .map_err(|_| "protocol_failure")?;
    }
    let value = adapter
        .finalize(Some(exit))
        .map_err(|_| "protocol_failure")?;
    DelegationReportNormalizer::normalize(value).map_err(|_| "report_failure")
}

pub(super) fn prompt(task: &str, context: Option<&str>, paths: &[PathBuf]) -> String {
    format!(
        "{task}\n\n<untrusted_context>\n{}\nArtifact input paths:\n{}\n</untrusted_context>",
        context.unwrap_or(""),
        paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n")
    )
}

pub(super) fn sha256(bytes: &[u8]) -> String {
    prefixed_hex(&Sha256::digest(bytes))
}

fn prefixed_hex(bytes: &[u8]) -> String {
    let hex = bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}

pub(super) fn executable_fingerprint(path: &Path) -> Result<String, &'static str> {
    let canonical = path.canonicalize().map_err(|_| "target_unavailable")?;
    let metadata = std::fs::symlink_metadata(&canonical).map_err(|_| "target_unavailable")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("target_unavailable");
    }
    let mut file = std::fs::File::open(canonical).map_err(|_| "target_unavailable")?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(|_| "target_unavailable")?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(prefixed_hex(&digest.finalize()))
}

pub(super) fn envelope(
    status: NativeToolResultStatus,
    output: Option<Value>,
    error_code: Option<NativeToolErrorCode>,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output,
        error_code,
        safe_error: error_code.map(|_| "CLI delegation failed safely.".to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}
