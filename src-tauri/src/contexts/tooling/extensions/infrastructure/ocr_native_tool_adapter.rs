use super::ocr_native_tool_support::{
    inference_limits, parse_input, publication_json, AdapterError, OcrWorkspace,
};
use super::{ManagedPaddleOcrWorker, ManagedPdfiumRenderer, OwnedOcrStagingAdapter};
use crate::contexts::agent_runtime::application::{
    NativeToolPortRequest, NativeToolProgress, NativeToolProgressPhase, NativeToolResultEnvelope,
    NativeToolResultStatus, OcrInferencePort, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactDescriptor, ArtifactEvidenceKind,
    ArtifactService, ArtifactVisibility,
};
use crate::contexts::code_execution::application::SandboxProcessBackend;
use crate::contexts::tooling::extensions::application::{
    normalize_ocr_result, ArtifactOcrSourceAdapter, ManagedOcrExecutionService,
    NormalizedOcrResult, OcrAdmissionLimits, OcrArtifactAdmissionService, OcrArtifactRequest,
};
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

pub(crate) struct OcrNativeToolAdapter {
    install_path: PathBuf,
    operations_root: PathBuf,
    backend: Arc<dyn SandboxProcessBackend>,
    artifacts: Arc<ArtifactService>,
}

impl OcrNativeToolAdapter {
    pub(crate) fn new(
        install_path: PathBuf,
        operations_root: PathBuf,
        backend: Arc<dyn SandboxProcessBackend>,
        artifacts: Arc<ArtifactService>,
    ) -> Self {
        Self {
            install_path,
            operations_root,
            backend,
            artifacts,
        }
    }

    fn execute_inner(
        &self,
        request: NativeToolPortRequest,
    ) -> Result<NativeToolResultEnvelope, AdapterError> {
        let input = parse_input(&request.input.value)?;
        if request.context.is_cancelled() {
            return Err(AdapterError::Cancelled);
        }
        request.context.progress.publish(NativeToolProgress {
            sequence: 1,
            phase: NativeToolProgressPhase::Started,
            message: Some("Preparing local OCR input".to_owned()),
            metadata: BTreeMap::new(),
        });
        let workspace = OcrWorkspace::create(&self.operations_root)?;
        let admission = OcrArtifactAdmissionService::new(
            Arc::new(ArtifactOcrSourceAdapter::new(self.artifacts.clone())),
            Arc::new(OwnedOcrStagingAdapter),
        );
        let artifact = admission
            .admit(
                OcrArtifactRequest {
                    artifact_id: input.artifact_id.clone(),
                    pages: input.pages,
                    limits: OcrAdmissionLimits::HARD_CEILING,
                },
                &workspace.root.join("inputs"),
            )
            .map_err(AdapterError::Admission)?;
        request.context.progress.publish(NativeToolProgress {
            sequence: 2,
            phase: NativeToolProgressPhase::Updated,
            message: Some("Running managed local OCR".to_owned()),
            metadata: BTreeMap::new(),
        });
        let renderer = Arc::new(ManagedPdfiumRenderer::new(
            self.install_path.clone(),
            workspace.root.clone(),
            self.backend.clone(),
        ));
        let worker = Arc::new(ManagedPaddleOcrWorker::new(
            self.install_path.clone(),
            workspace.root.clone(),
            self.backend.clone(),
        ));
        let execution = ManagedOcrExecutionService::new(renderer, worker);
        let started = Instant::now();
        let limits = inference_limits(&request)?;
        let engine = execution
            .execute(
                &request.context.call_id,
                &artifact,
                input.languages.clone(),
                OcrAdmissionLimits::HARD_CEILING,
                limits,
                &workspace.root.join("outputs/rendered"),
                request.context.cancelled.clone(),
            )
            .map_err(AdapterError::Execution)?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let result = normalize_ocr_result(
            &request.context.call_id,
            &artifact.artifact_id,
            &artifact.content_hash,
            input.languages,
            duration_ms,
            engine,
        )
        .map_err(AdapterError::Execution)?;
        workspace.cleanup()?;
        let published = if input.publish {
            self.seal_outputs(&request, &result)?
        } else {
            Vec::new()
        };
        let output = serde_json::to_value(&result).map_err(|_| AdapterError::Protocol)?;
        Ok(NativeToolResultEnvelope {
            contract_version: NATIVE_TOOL_CONTRACT_VERSION,
            status: NativeToolResultStatus::Succeeded,
            output: Some(json!({"result": output, "artifacts": published})),
            error_code: None,
            safe_error: None,
            truncated: result.truncated,
            metadata: BTreeMap::from([
                (
                    "source_artifact_id".to_owned(),
                    json!(result.source_artifact_id),
                ),
                (
                    "published_artifact_count".to_owned(),
                    json!(published.len()),
                ),
            ]),
        })
    }

    fn seal_outputs(
        &self,
        request: &NativeToolPortRequest,
        result: &NormalizedOcrResult,
    ) -> Result<Vec<Value>, AdapterError> {
        let base = ArtifactCreateRequest {
            operation_id: request.context.call_id.clone(),
            display_name: "ocr-result.txt".to_owned(),
            media_type: "text/plain".to_owned(),
            creator: ArtifactCreator {
                kind: "native_tool".to_owned(),
                id: request.context.agent_id.clone(),
            },
            evidence_kind: ArtifactEvidenceKind::HostVerified,
            visibility: ArtifactVisibility::Session,
            source_artifact_ids: vec![result.source_artifact_id.clone()],
            created_at: Utc::now().to_rfc3339(),
            expires_at: None,
        };
        let text = self
            .artifacts
            .create_text(base.clone(), &result.text)
            .map_err(|_| AdapterError::Artifact)?;
        let mut json_request = base;
        json_request.display_name = "ocr-result.json".to_owned();
        let value = serde_json::to_value(result).map_err(|_| AdapterError::Protocol)?;
        let structured = self
            .artifacts
            .create_json(json_request, &value)
            .map_err(|_| AdapterError::Artifact)?;
        [text, structured]
            .into_iter()
            .map(|artifact| self.publish(artifact))
            .collect()
    }

    fn publish(&self, artifact: ArtifactDescriptor) -> Result<Value, AdapterError> {
        let reference = self
            .artifacts
            .publish(
                &artifact.id,
                ArtifactVisibility::Session,
                &Utc::now().to_rfc3339(),
            )
            .map_err(|_| AdapterError::Artifact)?;
        Ok(publication_json(&artifact, &reference))
    }
}

impl OcrInferencePort for OcrNativeToolAdapter {
    fn execute_ocr(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.execute_inner(request)
            .unwrap_or_else(AdapterError::envelope)
    }
}

#[cfg(test)]
#[path = "ocr_native_tool_adapter_tests.rs"]
mod tests;
