//! Composer and artifact OCR.
//!
//! Both entry points converge on one staged file, one worker slot, and one result shape. The only
//! difference is how the bytes get into staging: the composer supplies a user-picked path, OnePiece
//! supplies verified artifact bytes and never a host path.

use super::cleanup::OperationMediaGuard;
use super::ports::ClaimedInput;
use super::service::{LocalMediaApplicationService, LocalMediaJob, PreparedLocalMediaOperation};
use super::worker_contract::{OcrWorkerRequest, WorkerCall, WorkerReply};
use crate::contexts::local_media::domain::{
    derive_plain_text, normalize_recognized_text, ComposerScopeId, LocalMediaEngine,
    LocalMediaError, LocalMediaErrorCode, LocalMediaOperationKind, LocalMediaOperationResult,
    LocalMediaPhase, LocalMediaProfileSnapshot, OcrLine, OcrPage, OcrProvenance, OcrResult,
    OcrSourceSummary, OcrWarning, StagedInputId, StagedOcrSource,
};
use std::path::Path;

impl LocalMediaApplicationService {
    /// Admit a user-selected file. The caller-supplied path is used exactly once, here, and is not
    /// retained: what comes back is an opaque id plus display metadata.
    pub(crate) fn stage_ocr_source(
        &self,
        source: &Path,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Ocr, &profile)?;
        let staged = self.inner.temp.stage_ocr_source(source)?;
        self.inner.diagnostics.record(
            "ocr.staged",
            &[
                ("mediaType", staged.media_type.as_str().to_string()),
                ("byteLength", staged.byte_length.to_string()),
            ],
        );
        Ok(staged)
    }

    /// Admit already-verified bytes from a managed artifact. Sharing the runtime must not add a
    /// host-path parameter to the OnePiece tool schema, so this takes bytes rather than a path.
    pub(crate) fn stage_ocr_artifact(
        &self,
        bytes: &[u8],
        display_name: &str,
    ) -> Result<StagedOcrSource, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Ocr, &profile)?;
        self.inner.temp.stage_bytes(bytes, display_name)
    }

    pub(crate) fn prepare_ocr(
        &self,
        staged_input_id: &StagedInputId,
        composer_scope: Option<ComposerScopeId>,
    ) -> Result<PreparedLocalMediaOperation, LocalMediaError> {
        let profile = self.inner.repository.load()?;
        self.ensure_engine_usable(LocalMediaEngine::Ocr, &profile)?;

        // Claim before allocating the operation: a reused staged id must fail without leaving an
        // orphan operation the composer would then poll forever.
        let claimed = self.inner.temp.claim(staged_input_id)?;

        let (operation_id, snapshot, accepted_at) = self.accept_operation(
            LocalMediaOperationKind::Ocr,
            LocalMediaEngine::Ocr,
            &profile,
            composer_scope,
        )?;

        Ok(PreparedLocalMediaOperation {
            operation_id,
            kind: LocalMediaOperationKind::Ocr,
            accepted_at,
            job: LocalMediaJob::Ocr { snapshot, claimed },
        })
    }

    pub(super) fn run_ocr(
        &self,
        operation_id: &str,
        snapshot: &LocalMediaProfileSnapshot,
        claimed: &ClaimedInput,
    ) {
        let mut guard = OperationMediaGuard::new(self.inner.temp.clone(), operation_id);
        // The staged directory belongs to this operation now; the guard covers the operation
        // directory, so the staged copy is released explicitly on every exit below.
        let staged_id = claimed.staged_input_id.clone();

        self.advance(operation_id, LocalMediaPhase::Queued);
        let flag = self.inner.operations.cancellation_flag(operation_id);
        let limits = snapshot.limits();

        let call = WorkerCall::Ocr(OcrWorkerRequest {
            source_path: claimed.path.clone(),
            media_type: claimed.source.media_type,
            max_pdf_pages: limits.max_pdf_pages,
            max_output_characters: limits.max_output_characters,
        });

        self.advance(operation_id, LocalMediaPhase::Processing);
        let outcome = self.inner.workers.call(snapshot, call, flag);
        let now_ms = self.inner.clock.now_ms();
        self.inner.temp.cleanup_staged(&staged_id);

        match outcome {
            Ok(WorkerReply::Ocr(reply)) => {
                let pages: Vec<OcrPage> = reply
                    .pages
                    .iter()
                    .map(|page| OcrPage {
                        page_number: page.page_number,
                        text: normalize_recognized_text(&page.text),
                        line_count: page.line_count,
                        lines: page
                            .lines
                            .iter()
                            .map(|line| OcrLine {
                                text: normalize_recognized_text(&line.text),
                                confidence: line.confidence,
                                polygon: line.polygon.clone(),
                            })
                            .collect(),
                    })
                    .collect();
                let plain_text = derive_plain_text(&pages);
                let mut warnings = Vec::new();
                if reply.truncated {
                    warnings.push(OcrWarning {
                        code: "OUTPUT_TRUNCATED".to_string(),
                        message_key: "localMedia.warnings.outputTruncated".to_string(),
                        page_number: None,
                    });
                }
                let result = OcrResult {
                    source: OcrSourceSummary {
                        display_name: claimed.source.display_name.clone(),
                        media_type: claimed.source.media_type,
                        page_count: pages.len() as u32,
                    },
                    character_count: plain_text.chars().count() as u32,
                    plain_text,
                    pages,
                    warnings,
                    provenance: OcrProvenance {
                        engine: "paddleocr".to_string(),
                        engine_version: reply.engine_version.clone(),
                        profile_revision: snapshot.profile_revision(),
                        language: snapshot.ocr().language.clone(),
                        model_identity: reply.model_identity.clone(),
                    },
                    truncated: reply.truncated,
                };

                // Recognizing nothing is a successful read with an informational outcome, not a
                // worker failure: the review dialog explains it and the draft is untouched.
                self.inner.diagnostics.record(
                    "ocr.completed",
                    &[
                        ("pageCount", result.source.page_count.to_string()),
                        ("characterCount", result.character_count.to_string()),
                        ("noTextDetected", result.is_empty().to_string()),
                    ],
                );
                self.inner.store.succeed(
                    operation_id,
                    LocalMediaOperationResult::Ocr(result),
                    now_ms,
                );
                self.inner.operations.succeed(operation_id);
            }
            Ok(_) => {
                self.settle_failure(
                    operation_id,
                    LocalMediaError::new(LocalMediaErrorCode::WorkerProtocolError),
                    now_ms,
                );
            }
            Err(error) => self.settle_failure(operation_id, error, now_ms),
        }

        guard.disarm();
        self.inner.temp.cleanup_operation(operation_id);
    }

    /// Shared terminal-failure path. Cancellation is recorded as cancelled rather than failed so
    /// the composer does not show an error the user caused deliberately.
    pub(super) fn settle_failure(&self, operation_id: &str, error: LocalMediaError, now_ms: u64) {
        if error.is_cancelled() {
            self.inner.operations.cancel(operation_id);
            self.inner.store.cancel(operation_id, now_ms);
            return;
        }
        self.inner.diagnostics.record(
            "operation.failed",
            &[("code", error.code().as_str().to_string())],
        );
        self.inner
            .operations
            .fail(operation_id, error.code().as_str());
        self.inner.store.fail(operation_id, error, now_ms);
    }
}
