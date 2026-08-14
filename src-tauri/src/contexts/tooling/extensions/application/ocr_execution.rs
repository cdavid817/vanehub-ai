use super::{
    AdmittedOcrArtifact, OcrAdmissionLimits, PaddleOcrEngineResult, PaddleOcrInferenceInput,
    PaddleOcrInferenceLimits, PaddleOcrInferenceRequest, PaddleOcrInputKind,
    PADDLEOCR_INFERENCE_PROTOCOL_VERSION,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RenderedOcrPage {
    pub(crate) page_number: u32,
    pub(crate) path: PathBuf,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) size_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OcrExecutionError {
    InvalidRequest,
    RenderFailure,
    WorkerFailure,
    Cancelled,
    LimitExceeded,
    ProtocolViolation,
}

pub(crate) trait ManagedPdfRendererPort: Send + Sync {
    fn page_count(
        &self,
        pdf_path: &Path,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u32, OcrExecutionError>;

    fn render_pages(
        &self,
        pdf_path: &Path,
        page_numbers: &[u32],
        output_directory: &Path,
        limits: OcrAdmissionLimits,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Vec<RenderedOcrPage>, OcrExecutionError>;
}

pub(crate) trait PaddleOcrWorkerPort: Send + Sync {
    fn execute(
        &self,
        request: &PaddleOcrInferenceRequest,
        cancelled: Arc<AtomicBool>,
    ) -> Result<PaddleOcrEngineResult, OcrExecutionError>;
}

pub(crate) struct ManagedOcrExecutionService {
    renderer: Arc<dyn ManagedPdfRendererPort>,
    worker: Arc<dyn PaddleOcrWorkerPort>,
}

impl ManagedOcrExecutionService {
    pub(crate) fn new(
        renderer: Arc<dyn ManagedPdfRendererPort>,
        worker: Arc<dyn PaddleOcrWorkerPort>,
    ) -> Self {
        Self { renderer, worker }
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn execute(
        &self,
        operation_id: &str,
        artifact: &AdmittedOcrArtifact,
        languages: Vec<String>,
        admission_limits: OcrAdmissionLimits,
        inference_limits: PaddleOcrInferenceLimits,
        render_directory: &Path,
        cancelled: Arc<AtomicBool>,
    ) -> Result<PaddleOcrEngineResult, OcrExecutionError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(OcrExecutionError::Cancelled);
        }
        let inputs = if artifact.media_type == "application/pdf" {
            self.render_pdf(
                artifact,
                admission_limits,
                render_directory,
                cancelled.clone(),
            )?
        } else {
            vec![PaddleOcrInferenceInput {
                staged_path: artifact.staged_path.clone(),
                source_sha256: artifact.content_hash.clone(),
                kind: PaddleOcrInputKind::Image,
                page_number: None,
            }]
        };
        let request = PaddleOcrInferenceRequest {
            protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
            operation_id: operation_id.to_owned(),
            inputs,
            languages,
            limits: inference_limits,
        };
        request
            .validate()
            .map_err(|_| OcrExecutionError::InvalidRequest)?;
        let result = self.worker.execute(&request, cancelled)?;
        validate_result(&result, inference_limits)?;
        Ok(result)
    }

    fn render_pdf(
        &self,
        artifact: &AdmittedOcrArtifact,
        limits: OcrAdmissionLimits,
        render_directory: &Path,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Vec<PaddleOcrInferenceInput>, OcrExecutionError> {
        let page_count = self
            .renderer
            .page_count(&artifact.staged_path, cancelled.clone())?;
        if page_count == 0
            || artifact.pages.is_empty()
            || artifact.pages.len() > limits.pdf_pages as usize
            || artifact.pages.iter().any(|page| *page > page_count)
        {
            return Err(OcrExecutionError::LimitExceeded);
        }
        let rendered = self.renderer.render_pages(
            &artifact.staged_path,
            &artifact.pages,
            render_directory,
            limits,
            cancelled,
        )?;
        if rendered.len() != artifact.pages.len() {
            return Err(OcrExecutionError::ProtocolViolation);
        }
        rendered
            .into_iter()
            .zip(&artifact.pages)
            .map(|(page, expected)| {
                let pixels = u64::from(page.width).saturating_mul(u64::from(page.height));
                if page.page_number != *expected
                    || page.size_bytes > limits.rendered_page_bytes
                    || page.width > limits.image_width
                    || page.height > limits.image_height
                    || pixels > limits.image_pixels
                    || !page.path.starts_with(render_directory)
                {
                    return Err(OcrExecutionError::LimitExceeded);
                }
                Ok(PaddleOcrInferenceInput {
                    staged_path: page.path,
                    source_sha256: artifact.content_hash.clone(),
                    kind: PaddleOcrInputKind::RenderedPdfPage,
                    page_number: Some(page.page_number),
                })
            })
            .collect()
    }
}

fn validate_result(
    result: &PaddleOcrEngineResult,
    limits: PaddleOcrInferenceLimits,
) -> Result<(), OcrExecutionError> {
    if result.protocol_version != PADDLEOCR_INFERENCE_PROTOCOL_VERSION
        || result.blocks.len() > limits.output_blocks as usize
        || result.warnings.len() > 32
        || result
            .blocks
            .iter()
            .map(|block| block.text.chars().count() as u64)
            .sum::<u64>()
            > limits.output_characters
        || result.blocks.iter().any(|block| {
            block.text.contains('\0')
                || block
                    .confidence
                    .is_some_and(|confidence| !(0.0..=1.0).contains(&confidence))
                || block
                    .polygon
                    .as_ref()
                    .is_some_and(|polygon| polygon.len() > 16)
        })
    {
        Err(OcrExecutionError::ProtocolViolation)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Renderer;
    impl ManagedPdfRendererPort for Renderer {
        fn page_count(&self, _: &Path, _: Arc<AtomicBool>) -> Result<u32, OcrExecutionError> {
            Ok(2)
        }
        fn render_pages(
            &self,
            _: &Path,
            pages: &[u32],
            output: &Path,
            _: OcrAdmissionLimits,
            _: Arc<AtomicBool>,
        ) -> Result<Vec<RenderedOcrPage>, OcrExecutionError> {
            Ok(pages
                .iter()
                .map(|page| RenderedOcrPage {
                    page_number: *page,
                    path: output.join(format!("page-{page}.png")),
                    width: 100,
                    height: 100,
                    size_bytes: 100,
                })
                .collect())
        }
    }
    struct Worker;
    impl PaddleOcrWorkerPort for Worker {
        fn execute(
            &self,
            request: &PaddleOcrInferenceRequest,
            _: Arc<AtomicBool>,
        ) -> Result<PaddleOcrEngineResult, OcrExecutionError> {
            assert_eq!(request.inputs[0].page_number, Some(2));
            Ok(PaddleOcrEngineResult {
                protocol_version: PADDLEOCR_INFERENCE_PROTOCOL_VERSION.to_owned(),
                engine_name: "paddleocr".to_owned(),
                engine_version: "3.2.0".to_owned(),
                blocks: Vec::new(),
                warnings: Vec::new(),
                truncated: false,
            })
        }
    }

    #[test]
    fn pdf_pages_are_authoritatively_checked_rendered_and_bounded_before_worker() {
        let root = tempfile::tempdir().expect("root");
        let artifact = AdmittedOcrArtifact {
            artifact_id: "artifact".to_owned(),
            content_hash: "a".repeat(64),
            media_type: "application/pdf".to_owned(),
            staged_path: root.path().join("source.pdf"),
            pages: vec![2],
        };
        let service = ManagedOcrExecutionService::new(Arc::new(Renderer), Arc::new(Worker));
        let result = service.execute(
            "operation",
            &artifact,
            vec!["en".to_owned()],
            OcrAdmissionLimits::HARD_CEILING,
            PaddleOcrInferenceLimits::HARD_CEILING,
            root.path(),
            Arc::new(AtomicBool::new(false)),
        );
        assert!(result.is_ok());
    }
}
