use crate::contexts::agent_runtime::application::{
    ArtifactPort, NativeToolErrorCode, NativeToolOperation, NativeToolPortRequest,
    NativeToolResultEnvelope, NativeToolResultStatus, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::artifacts::application::{
    ArtifactCreator, ArtifactDescriptor, ArtifactEvidenceKind, ArtifactService,
    ArtifactServiceError, ArtifactVisibility,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

pub(crate) struct ArtifactNativeToolAdapter {
    service: Arc<ArtifactService>,
}

impl ArtifactNativeToolAdapter {
    pub(crate) fn new(service: Arc<ArtifactService>) -> Self {
        Self { service }
    }
}

impl ArtifactPort for ArtifactNativeToolAdapter {
    fn execute_artifact(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        if request.context.is_cancelled() {
            return failure(
                NativeToolResultStatus::Cancelled,
                NativeToolErrorCode::Cancelled,
                "Artifact operation was cancelled.",
            );
        }
        if request.context.deadline_reached() {
            return failure(
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::DeadlineExceeded,
                "Artifact operation deadline was reached.",
            );
        }
        let result = match request.input.operation {
            NativeToolOperation::ArtifactRead => self.execute_read(&request.input.value),
            NativeToolOperation::ArtifactPublish => self.execute_publish(&request.input.value),
            _ => Err(ArtifactServiceError::InvalidRequest),
        };
        match result {
            Ok(output) => success(output),
            Err(error) => service_failure(error),
        }
    }
}

impl ArtifactNativeToolAdapter {
    fn execute_read(&self, input: &Value) -> Result<Value, ArtifactServiceError> {
        match string(input, "operation")? {
            "list" => {
                let limit = usize_value(input, "limit", 50)?;
                let artifacts = self
                    .service
                    .list_metadata(limit)?
                    .iter()
                    .map(descriptor_json)
                    .collect::<Vec<_>>();
                Ok(json!({"artifacts": artifacts}))
            }
            "metadata" => Ok(descriptor_json(
                &self.service.metadata(string(input, "artifact_id")?)?,
            )),
            "read_text" => {
                let preview = self.service.read_text(
                    string(input, "artifact_id")?,
                    u64_value(input, "offset", 0)?,
                    usize_value(input, "limit", 16_384)?,
                )?;
                Ok(json!({
                    "contract_version": preview.contract_version,
                    "artifact_id": preview.artifact_id,
                    "content_hash": preview.content_hash,
                    "media_type": preview.media_type,
                    "offset": preview.offset,
                    "next_offset": preview.next_offset,
                    "text": preview.text,
                    "truncated": preview.truncated,
                }))
            }
            _ => Err(ArtifactServiceError::InvalidRequest),
        }
    }

    fn execute_publish(&self, input: &Value) -> Result<Value, ArtifactServiceError> {
        if string(input, "operation")? != "publish" {
            return Err(ArtifactServiceError::InvalidRequest);
        }
        let visibility = match string(input, "visibility")? {
            "private" => ArtifactVisibility::Private,
            "session" => ArtifactVisibility::Session,
            _ => return Err(ArtifactServiceError::InvalidRequest),
        };
        let publication = self.service.publish(
            string(input, "artifact_id")?,
            visibility,
            &Utc::now().to_rfc3339(),
        )?;
        Ok(json!({
            "contract_version": publication.contract_version,
            "reference": publication.reference,
            "artifact_id": publication.artifact_id,
            "content_hash": publication.content_hash,
            "visibility": visibility_name(publication.visibility),
            "published_at": publication.published_at,
        }))
    }
}

fn descriptor_json(artifact: &ArtifactDescriptor) -> Value {
    json!({
        "contract_version": artifact.contract_version,
        "id": artifact.id,
        "content_hash": artifact.content_hash,
        "size_bytes": artifact.size_bytes,
        "media_type": artifact.media_type,
        "display_name": artifact.display_name,
        "creator": creator_json(&artifact.creator),
        "evidence_kind": evidence_name(artifact.evidence_kind),
        "visibility": visibility_name(artifact.visibility),
        "source_operation_id": artifact.source_operation_id,
        "source_artifact_ids": artifact.source_artifact_ids,
        "created_at": artifact.created_at,
        "expires_at": artifact.expires_at,
    })
}

fn creator_json(creator: &ArtifactCreator) -> Value {
    json!({"kind": creator.kind, "id": creator.id})
}

const fn evidence_name(value: ArtifactEvidenceKind) -> &'static str {
    match value {
        ArtifactEvidenceKind::HostVerified => "host_verified",
        ArtifactEvidenceKind::ProviderReported => "provider_reported",
        ArtifactEvidenceKind::UntrustedExternal => "untrusted_external",
    }
}

const fn visibility_name(value: ArtifactVisibility) -> &'static str {
    match value {
        ArtifactVisibility::Private => "private",
        ArtifactVisibility::Session => "session",
    }
}

fn string<'a>(input: &'a Value, name: &str) -> Result<&'a str, ArtifactServiceError> {
    input
        .get(name)
        .and_then(Value::as_str)
        .ok_or(ArtifactServiceError::InvalidRequest)
}

fn u64_value(input: &Value, name: &str, default: u64) -> Result<u64, ArtifactServiceError> {
    match input.get(name) {
        Some(value) => value.as_u64().ok_or(ArtifactServiceError::InvalidRequest),
        None => Ok(default),
    }
}

fn usize_value(input: &Value, name: &str, default: usize) -> Result<usize, ArtifactServiceError> {
    usize::try_from(u64_value(input, name, default as u64)?)
        .map_err(|_| ArtifactServiceError::InvalidPage)
}

fn success(output: Value) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status: NativeToolResultStatus::Succeeded,
        output: Some(output),
        error_code: None,
        safe_error: None,
        truncated: false,
        metadata: BTreeMap::new(),
    }
}

fn failure(
    status: NativeToolResultStatus,
    code: NativeToolErrorCode,
    safe_error: &str,
) -> NativeToolResultEnvelope {
    NativeToolResultEnvelope {
        contract_version: NATIVE_TOOL_CONTRACT_VERSION,
        status,
        output: None,
        error_code: Some(code),
        safe_error: Some(safe_error.to_owned()),
        truncated: false,
        metadata: BTreeMap::new(),
    }
}

fn service_failure(error: ArtifactServiceError) -> NativeToolResultEnvelope {
    let (status, code, message) = match error {
        ArtifactServiceError::InvalidRequest | ArtifactServiceError::InvalidPage => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InvalidInput,
            "Artifact request is invalid.",
        ),
        ArtifactServiceError::NotFound => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::Unavailable,
            "Artifact was not found.",
        ),
        ArtifactServiceError::UnsupportedPreview => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InvalidInput,
            "Artifact does not support text preview.",
        ),
        ArtifactServiceError::Blob(_) => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::IntegrityFailure,
            "Artifact integrity verification failed.",
        ),
        ArtifactServiceError::CatalogFailure | ArtifactServiceError::PublicationFailure => (
            NativeToolResultStatus::Failed,
            NativeToolErrorCode::InternalFailure,
            "Artifact service is unavailable.",
        ),
    };
    failure(status, code, message)
}

#[cfg(test)]
#[path = "native_tool_adapter_tests.rs"]
mod tests;
