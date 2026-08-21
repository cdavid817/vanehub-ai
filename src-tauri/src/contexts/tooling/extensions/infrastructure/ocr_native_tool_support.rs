use crate::contexts::agent_runtime::application::{
    NativeToolErrorCode, NativeToolPortRequest, NativeToolResultEnvelope, NativeToolResultStatus,
    NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::artifacts::api::{ArtifactDescriptor, ArtifactPublicationReference};
use crate::contexts::tooling::extensions::application::{
    OcrAdmissionError, OcrExecutionError, PaddleOcrInferenceLimits,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Instant;
use uuid::Uuid;

pub(super) struct OcrInput {
    pub(super) artifact_id: String,
    pub(super) pages: Vec<u32>,
    pub(super) languages: Vec<String>,
    pub(super) publish: bool,
}

pub(super) fn parse_input(value: &Value) -> Result<OcrInput, AdapterError> {
    let object = value.as_object().ok_or(AdapterError::InvalidInput)?;
    let artifact_id = object
        .get("artifact_id")
        .and_then(Value::as_str)
        .ok_or(AdapterError::InvalidInput)?
        .to_owned();
    let pages = object
        .get("pages")
        .and_then(Value::as_array)
        .map(|pages| {
            pages
                .iter()
                .map(|page| {
                    page.as_u64()
                        .and_then(|value| u32::try_from(value).ok())
                        .ok_or(AdapterError::InvalidInput)
                })
                .collect()
        })
        .transpose()?
        .unwrap_or_default();
    let languages = object
        .get("languages")
        .and_then(Value::as_array)
        .ok_or(AdapterError::InvalidInput)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(AdapterError::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let publish = object
        .get("publish")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(OcrInput {
        artifact_id,
        pages,
        languages,
        publish,
    })
}

pub(super) fn inference_limits(
    request: &NativeToolPortRequest,
) -> Result<PaddleOcrInferenceLimits, AdapterError> {
    let remaining = request
        .context
        .deadline
        .checked_duration_since(Instant::now())
        .ok_or(AdapterError::Limit)?;
    let duration_ms = u64::try_from(remaining.as_millis())
        .unwrap_or(u64::MAX)
        .min(PaddleOcrInferenceLimits::HARD_CEILING.duration_ms);
    if duration_ms == 0 {
        return Err(AdapterError::Limit);
    }
    Ok(PaddleOcrInferenceLimits {
        duration_ms,
        ..PaddleOcrInferenceLimits::HARD_CEILING
    })
}

pub(super) fn publication_json(
    artifact: &ArtifactDescriptor,
    reference: &ArtifactPublicationReference,
) -> Value {
    json!({
        "artifact_id": artifact.id,
        "content_hash": artifact.content_hash,
        "media_type": artifact.media_type,
        "size_bytes": artifact.size_bytes,
        "reference": reference.reference,
        "source_artifact_ids": artifact.source_artifact_ids
    })
}

pub(super) struct OcrWorkspace {
    pub(super) root: PathBuf,
    cleaned: bool,
}

impl OcrWorkspace {
    pub(super) fn create(operations_root: &Path) -> Result<Self, AdapterError> {
        ensure_owned_directory(operations_root)?;
        let root = operations_root.join(format!("ocr-{}", Uuid::new_v4()));
        std::fs::create_dir(&root).map_err(|_| AdapterError::Workspace)?;
        for name in ["inputs", "work", "outputs"] {
            std::fs::create_dir(root.join(name)).map_err(|_| AdapterError::Workspace)?;
        }
        Ok(Self {
            root,
            cleaned: false,
        })
    }

    pub(super) fn cleanup(mut self) -> Result<(), AdapterError> {
        std::fs::remove_dir_all(&self.root).map_err(|_| AdapterError::Cleanup)?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for OcrWorkspace {
    fn drop(&mut self) {
        if !self.cleaned {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}

fn ensure_owned_directory(path: &Path) -> Result<(), AdapterError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(AdapterError::Workspace),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir_all(path).map_err(|_| AdapterError::Workspace)
        }
        Err(_) => Err(AdapterError::Workspace),
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum AdapterError {
    InvalidInput,
    Admission(OcrAdmissionError),
    Execution(OcrExecutionError),
    Workspace,
    Cleanup,
    Artifact,
    Protocol,
    Cancelled,
    Limit,
}

impl AdapterError {
    pub(super) fn envelope(self) -> NativeToolResultEnvelope {
        let (status, code, message) = match self {
            Self::Cancelled | Self::Execution(OcrExecutionError::Cancelled) => (
                NativeToolResultStatus::Cancelled,
                NativeToolErrorCode::Cancelled,
                "The local OCR operation was cancelled.",
            ),
            Self::Limit
            | Self::Admission(OcrAdmissionError::LimitExceeded)
            | Self::Execution(OcrExecutionError::LimitExceeded) => (
                NativeToolResultStatus::LimitExceeded,
                NativeToolErrorCode::LimitExceeded,
                "The local OCR operation reached a hard limit.",
            ),
            Self::Admission(OcrAdmissionError::IntegrityFailure) => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::IntegrityFailure,
                "The source Artifact failed integrity verification.",
            ),
            Self::InvalidInput | Self::Admission(OcrAdmissionError::InvalidRequest) => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::InvalidInput,
                "The OCR request is invalid.",
            ),
            Self::Cleanup => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::InternalFailure,
                "The private OCR workspace could not be cleaned safely.",
            ),
            _ => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::ExternalFailure,
                "The managed local OCR operation failed.",
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
}
