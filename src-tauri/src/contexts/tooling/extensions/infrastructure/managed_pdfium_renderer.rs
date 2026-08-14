use super::pdfium_protocol::{
    admitted_regular_file, ensure_directory, minimal_environment, read_bounded_json,
    renderer_process_limits, validate_render_result, verified_runtime, write_new_json,
    ControlFiles, OcrAdmissionLimitsWire, RendererRequest, RendererResult, PROTOCOL_VERSION,
    RENDER_DURATION_MS,
};
use crate::contexts::code_execution::application::{SandboxLaunchRequest, SandboxProcessBackend};
use crate::contexts::tooling::extensions::application::{
    ManagedPdfRendererPort, OcrAdmissionLimits, OcrExecutionError, RenderedOcrPage,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) struct ManagedPdfiumRenderer {
    install_path: PathBuf,
    execution_root: PathBuf,
    backend: Arc<dyn SandboxProcessBackend>,
}

impl ManagedPdfiumRenderer {
    pub(crate) fn new(
        install_path: PathBuf,
        execution_root: PathBuf,
        backend: Arc<dyn SandboxProcessBackend>,
    ) -> Self {
        Self {
            install_path,
            execution_root,
            backend,
        }
    }

    fn execute(
        &self,
        request_name: &str,
        result_name: &str,
        request: &RendererRequest<'_>,
        cancelled: Arc<AtomicBool>,
    ) -> Result<RendererResult, OcrExecutionError> {
        if cancelled.load(Ordering::Acquire) {
            return Err(OcrExecutionError::Cancelled);
        }
        let (binary, manifest) = verified_runtime(&self.install_path)?;
        let work = self.execution_root.join("work");
        let outputs = self.execution_root.join("outputs");
        ensure_directory(&work)?;
        ensure_directory(&outputs)?;
        let request_path = work.join(request_name);
        let result_path = outputs.join(result_name);
        let controls = ControlFiles::new(request_path.clone(), result_path.clone());
        write_new_json(&request_path, request)?;
        let launch = SandboxLaunchRequest {
            executable: binary,
            arguments: vec![
                "--protocol".to_owned(),
                PROTOCOL_VERSION.to_owned(),
                "--request".to_owned(),
                request_path.to_string_lossy().into_owned(),
                "--result".to_owned(),
                result_path.to_string_lossy().into_owned(),
            ],
            working_directory: work,
            environment: minimal_environment(),
            limits: renderer_process_limits(),
        };
        let mut process = self
            .backend
            .launch(launch)
            .map_err(|_| OcrExecutionError::RenderFailure)?;
        let deadline = Instant::now() + Duration::from_millis(RENDER_DURATION_MS);
        let observation = loop {
            if cancelled.load(Ordering::Acquire) {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| OcrExecutionError::RenderFailure)?;
                return Err(OcrExecutionError::Cancelled);
            }
            if Instant::now() >= deadline {
                process
                    .terminate_tree(Duration::from_secs(2))
                    .map_err(|_| OcrExecutionError::RenderFailure)?;
                return Err(OcrExecutionError::LimitExceeded);
            }
            if let Some(value) = process
                .wait_until((Instant::now() + Duration::from_millis(50)).min(deadline))
                .map_err(|_| OcrExecutionError::RenderFailure)?
            {
                break value;
            }
        };
        if observation.exit_code != 0
            || observation.stdout.len() > 64 * 1024
            || observation.stderr.len() > 64 * 1024
        {
            return Err(OcrExecutionError::RenderFailure);
        }
        let result: RendererResult = read_bounded_json(&result_path)?;
        drop(controls);
        if result.protocol_version != PROTOCOL_VERSION
            || result.renderer_version != manifest.renderer_version
        {
            return Err(OcrExecutionError::ProtocolViolation);
        }
        Ok(result)
    }

    fn validate_source(&self, source: &Path) -> Result<(), OcrExecutionError> {
        let inputs = self.execution_root.join("inputs");
        let canonical_inputs = inputs
            .canonicalize()
            .map_err(|_| OcrExecutionError::InvalidRequest)?;
        let canonical_source = source
            .canonicalize()
            .map_err(|_| OcrExecutionError::InvalidRequest)?;
        admitted_regular_file(&canonical_inputs, &canonical_source, 64 * 1024 * 1024)?;
        Ok(())
    }
}

impl ManagedPdfRendererPort for ManagedPdfiumRenderer {
    fn page_count(
        &self,
        pdf_path: &Path,
        cancelled: Arc<AtomicBool>,
    ) -> Result<u32, OcrExecutionError> {
        self.validate_source(pdf_path)?;
        let request = RendererRequest {
            protocol_version: PROTOCOL_VERSION,
            action: "inspect",
            source_path: pdf_path,
            page_numbers: &[],
            output_directory: None,
            limits: None,
        };
        let result = self.execute(
            "pdfium-inspect-request.json",
            "pdfium-inspect-result.json",
            &request,
            cancelled,
        )?;
        if result.action != "inspect" || !result.pages.is_empty() {
            return Err(OcrExecutionError::ProtocolViolation);
        }
        result
            .page_count
            .filter(|count| (1..=100_000).contains(count))
            .ok_or(OcrExecutionError::ProtocolViolation)
    }

    fn render_pages(
        &self,
        pdf_path: &Path,
        page_numbers: &[u32],
        output_directory: &Path,
        limits: OcrAdmissionLimits,
        cancelled: Arc<AtomicBool>,
    ) -> Result<Vec<RenderedOcrPage>, OcrExecutionError> {
        self.validate_source(pdf_path)?;
        if page_numbers.is_empty()
            || page_numbers.len() > limits.pdf_pages as usize
            || page_numbers.contains(&0)
            || page_numbers.windows(2).any(|pair| pair[0] >= pair[1])
            || output_directory != self.execution_root.join("outputs").join("rendered")
        {
            return Err(OcrExecutionError::InvalidRequest);
        }
        ensure_directory(output_directory)?;
        let wire_limits = OcrAdmissionLimitsWire {
            image_width: limits.image_width,
            image_height: limits.image_height,
            image_pixels: limits.image_pixels,
            rendered_page_bytes: limits.rendered_page_bytes,
        };
        let request = RendererRequest {
            protocol_version: PROTOCOL_VERSION,
            action: "render",
            source_path: pdf_path,
            page_numbers,
            output_directory: Some(output_directory),
            limits: Some(wire_limits),
        };
        let result = self.execute(
            "pdfium-render-request.json",
            "pdfium-render-result.json",
            &request,
            cancelled,
        )?;
        validate_render_result(result, page_numbers, output_directory, limits)
    }
}

#[cfg(test)]
#[path = "managed_pdfium_renderer_tests.rs"]
mod tests;
