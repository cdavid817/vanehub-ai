//! The OnePiece OCR tool, backed by the shared local-media PaddleOCR runtime.
//!
//! This is the whole of OnePiece's OCR implementation now. It owns no worker, no model
//! configuration, and no temporary directory: it resolves the managed artifact, hands its *bytes*
//! to `local_media::api`, and maps the shared result back onto the tool's contract. Taking bytes
//! rather than a path is what keeps the tool schema from gaining a host-path parameter as a side
//! effect of sharing the runtime.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use chrono::Utc;
use serde_json::{json, Value};

use crate::contexts::agent_runtime::application::{
    normalize_ocr_result, NativeToolErrorCode, NativeToolPortRequest, NativeToolProgress,
    NativeToolProgressPhase, NativeToolResultEnvelope, NativeToolResultStatus, NormalizedOcrResult,
    OcrInferencePort, OcrResultBlock, OcrResultIdentity, OcrResultPoint,
    IMAGE_ARTIFACT_METADATA_KEY, NATIVE_TOOL_CONTRACT_VERSION,
};
use crate::contexts::artifacts::application::{
    ArtifactCreateRequest, ArtifactCreator, ArtifactDescriptor, ArtifactEvidenceKind,
    ArtifactService, ArtifactVisibility,
};
use crate::contexts::local_media::api::{LocalMediaApi, OcrPage, SharedOcrResult};

/// How long to wait for the shared operation to settle before giving up.
///
/// The tool's own deadline is the real bound; this is the backstop for the case where the operation
/// never reaches a terminal state at all, which would otherwise hang the Agent's turn.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

pub(crate) struct LocalMediaOcrAdapter {
    local_media: LocalMediaApi,
    artifacts: Arc<ArtifactService>,
}

impl LocalMediaOcrAdapter {
    pub(crate) fn new(local_media: LocalMediaApi, artifacts: Arc<ArtifactService>) -> Self {
        Self {
            local_media,
            artifacts,
        }
    }

    fn execute_inner(
        &self,
        request: &NativeToolPortRequest,
    ) -> Result<NativeToolResultEnvelope, OcrToolError> {
        let input = parse_input(&request.input.value)?;
        if request.context.is_cancelled() {
            return Err(OcrToolError::Cancelled);
        }
        request.context.progress.publish(NativeToolProgress {
            sequence: 1,
            phase: NativeToolProgressPhase::Started,
            message: Some("Preparing local OCR input".to_owned()),
            metadata: BTreeMap::new(),
        });

        let (descriptor, bytes) = read_admitted_artifact(&self.artifacts, &input.artifact_id)?;
        request.context.progress.publish(NativeToolProgress {
            sequence: 2,
            phase: NativeToolProgressPhase::Updated,
            message: Some("Running shared local OCR".to_owned()),
            metadata: BTreeMap::new(),
        });

        let started = Instant::now();
        let prepared = self
            .local_media
            .prepare_artifact_ocr(&bytes, &descriptor.display_name)
            .map_err(|_| OcrToolError::Execution)?;
        let operation_id = prepared.operation_id.clone();
        // Executed inline rather than spawned: the native-tool port is already running on its own
        // worker thread with the Agent's deadline attached, so a second hop would only add a
        // lifetime this adapter would then have to manage.
        self.local_media.execute(prepared);

        let ocr = self.await_result(&operation_id, request)?;
        let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);

        let blocks = blocks_from_pages(&ocr.pages);
        let warnings = ocr
            .warnings
            .iter()
            .map(|warning| warning.code.clone())
            .chain(ocr.is_empty().then(|| "no_text_detected".to_owned()))
            .collect();
        let result = normalize_ocr_result(
            OcrResultIdentity {
                operation_id: &request.context.call_id,
                artifact_id: &descriptor.id,
                content_hash: descriptor.content_hash.trim_start_matches("sha256:"),
                engine_name: &ocr.provenance.engine,
                engine_version: ocr
                    .provenance
                    .engine_version
                    .as_deref()
                    .unwrap_or("unknown"),
                languages: input.languages,
                duration_ms,
                truncated: ocr.truncated,
                warnings,
            },
            blocks,
        )
        .ok_or(OcrToolError::Protocol)?;

        let published = self.seal_outputs(request, &result, input.publish)?;
        let output = serde_json::to_value(&result).map_err(|_| OcrToolError::Protocol)?;
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
                // The source artifact is what OCR read. The previous implementation could also
                // attach a page rasterized by a separate pdfium sidecar; that sidecar was never
                // shipped and is gone, so a PDF source now declares itself here exactly as the old
                // code's own fallback did for every case it could not rasterize.
                (
                    IMAGE_ARTIFACT_METADATA_KEY.to_owned(),
                    json!(result.source_artifact_id),
                ),
                (
                    "published_artifact_count".to_owned(),
                    json!(published.len()),
                ),
            ]),
        })
    }

    /// Poll the shared operation until it settles, the tool's deadline passes, or the call is
    /// cancelled.
    fn await_result(
        &self,
        operation_id: &str,
        request: &NativeToolPortRequest,
    ) -> Result<SharedOcrResult, OcrToolError> {
        loop {
            if request.context.is_cancelled() {
                self.local_media.cancel_operation(operation_id);
                return Err(OcrToolError::Cancelled);
            }
            if Instant::now() >= request.context.deadline {
                self.local_media.cancel_operation(operation_id);
                return Err(OcrToolError::Limit);
            }
            match self.local_media.get_ocr_result(operation_id) {
                Ok(Some(result)) => return Ok(result),
                Ok(None) => std::thread::sleep(POLL_INTERVAL),
                // A cancelled operation must not read as a crash: the Agent's retry policy branches
                // on the envelope status, and retrying a deliberate cancellation is wrong.
                Err(error) if error.is_cancelled() => return Err(OcrToolError::Cancelled),
                Err(_) => return Err(OcrToolError::Execution),
            }
        }
    }

    fn seal_outputs(
        &self,
        request: &NativeToolPortRequest,
        result: &NormalizedOcrResult,
        publish: bool,
    ) -> Result<Vec<Value>, OcrToolError> {
        if !publish {
            return Ok(Vec::new());
        }
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
            .map_err(|_| OcrToolError::Artifact)?;
        let mut json_request = base;
        json_request.display_name = "ocr-result.json".to_owned();
        let value = serde_json::to_value(result).map_err(|_| OcrToolError::Protocol)?;
        let structured = self
            .artifacts
            .create_json(json_request, &value)
            .map_err(|_| OcrToolError::Artifact)?;
        [text, structured]
            .into_iter()
            .map(|artifact| self.publish(artifact))
            .collect()
    }

    fn publish(&self, artifact: ArtifactDescriptor) -> Result<Value, OcrToolError> {
        let reference = self
            .artifacts
            .publish(
                &artifact.id,
                ArtifactVisibility::Session,
                &Utc::now().to_rfc3339(),
            )
            .map_err(|_| OcrToolError::Artifact)?;
        Ok(json!({
            "artifact_id": artifact.id,
            "content_hash": artifact.content_hash,
            "media_type": artifact.media_type,
            "size_bytes": artifact.size_bytes,
            "reference": reference.reference,
            "source_artifact_ids": artifact.source_artifact_ids
        }))
    }
}

/// Read an artifact's bytes, re-verifying the content hash and the declared length.
///
/// A free function rather than a method so it can be driven against an `ArtifactService` built from
/// port doubles, without a whole local-media runtime standing behind it.
///
/// The length is checked as the read proceeds and again when it ends. A chunk source that stops
/// early -- a truncated blob, an artifact rewritten between the metadata read and the last chunk, a
/// storage bug -- still produces bytes that sniff as a valid PNG or JPEG and whose header
/// dimensions parse, so OCR would run on a partial document and return a plausible-looking result
/// carrying full provenance. The declared length is the cheap end-to-end assertion that catches it,
/// and it has to fail before anything is handed to admission or inference.
fn read_admitted_artifact(
    artifacts: &ArtifactService,
    artifact_id: &str,
) -> Result<(ArtifactDescriptor, Vec<u8>), OcrToolError> {
    let descriptor = artifacts
        .metadata(artifact_id)
        .map_err(|_| OcrToolError::Admission)?;
    let capacity = usize::try_from(descriptor.size_bytes).map_err(|_| OcrToolError::Limit)?;
    let mut admitted = Vec::with_capacity(capacity);
    let mut offset = 0_u64;
    loop {
        let chunk = artifacts
            .download_chunk(artifact_id, offset, 1_048_576)
            .map_err(|_| OcrToolError::Admission)?;
        if chunk.offset != offset || chunk.content_hash != descriptor.content_hash {
            return Err(OcrToolError::Admission);
        }
        admitted.extend_from_slice(&chunk.bytes);
        // Checked, and against the declared size rather than only against the final total: a
        // source that overruns is refused at the chunk that overran rather than after it has been
        // allowed to grow the buffer past what the metadata promised.
        let read = u64::try_from(admitted.len()).map_err(|_| OcrToolError::Limit)?;
        if read > descriptor.size_bytes {
            return Err(OcrToolError::Admission);
        }
        let Some(next) = chunk.next_offset else {
            // The end of the stream has to agree with the metadata exactly.
            if read != descriptor.size_bytes {
                return Err(OcrToolError::Admission);
            }
            break;
        };
        if next <= offset || next > descriptor.size_bytes {
            return Err(OcrToolError::Admission);
        }
        offset = next;
    }
    Ok((descriptor, admitted))
}

/// Flatten the shared page/line result into the tool's block list.
///
/// `order` restarts per page and follows engine reading order, which is the same projection the
/// previous implementation produced from its own worker.
fn blocks_from_pages(pages: &[OcrPage]) -> Vec<OcrResultBlock> {
    let mut blocks = Vec::new();
    for page in pages {
        for (index, line) in page.lines.iter().enumerate() {
            blocks.push(OcrResultBlock {
                page_number: page.page_number,
                order: index as u32 + 1,
                text: line.text.clone(),
                polygon: line.polygon.as_ref().map(|points| {
                    points
                        .iter()
                        .map(|(x, y)| OcrResultPoint { x: *x, y: *y })
                        .collect()
                }),
                confidence: line.confidence,
            });
        }
    }
    blocks
}

pub(crate) struct OcrInput {
    pub(crate) artifact_id: String,
    pub(crate) languages: Vec<String>,
    pub(crate) publish: bool,
}

/// Parse the tool schema's input.
///
/// `pages` is accepted and ignored: the shared runtime bounds PDF pages by profile policy rather
/// than by a caller-selected subset, and silently accepting a field the engine no longer honours
/// would be worse than the schema keeping it for compatibility.
pub(crate) fn parse_input(value: &Value) -> Result<OcrInput, OcrToolError> {
    let object = value.as_object().ok_or(OcrToolError::InvalidInput)?;
    let artifact_id = object
        .get("artifact_id")
        .and_then(Value::as_str)
        .ok_or(OcrToolError::InvalidInput)?
        .to_owned();
    let languages = object
        .get("languages")
        .and_then(Value::as_array)
        .ok_or(OcrToolError::InvalidInput)?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or(OcrToolError::InvalidInput)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let publish = object
        .get("publish")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    Ok(OcrInput {
        artifact_id,
        languages,
        publish,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OcrToolError {
    InvalidInput,
    Admission,
    Execution,
    Artifact,
    Protocol,
    Cancelled,
    Limit,
}

impl OcrToolError {
    pub(crate) fn envelope(self) -> NativeToolResultEnvelope {
        let (status, code, message) = match self {
            Self::Cancelled => (
                NativeToolResultStatus::Cancelled,
                NativeToolErrorCode::Cancelled,
                "The local OCR operation was cancelled.",
            ),
            Self::Limit => (
                NativeToolResultStatus::LimitExceeded,
                NativeToolErrorCode::LimitExceeded,
                "The local OCR operation reached a hard limit.",
            ),
            Self::Admission => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::IntegrityFailure,
                "The source Artifact failed integrity verification.",
            ),
            Self::InvalidInput => (
                NativeToolResultStatus::Failed,
                NativeToolErrorCode::InvalidInput,
                "The OCR request is invalid.",
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

impl OcrInferencePort for LocalMediaOcrAdapter {
    fn execute_ocr(&self, request: NativeToolPortRequest) -> NativeToolResultEnvelope {
        self.execute_inner(&request)
            .unwrap_or_else(OcrToolError::envelope)
    }
}

#[cfg(test)]
#[path = "local_media_ocr_adapter_tests.rs"]
mod tests;
