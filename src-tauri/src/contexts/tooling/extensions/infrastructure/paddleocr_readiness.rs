use super::installation_adapter::venv_python;
use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError, PaddleOcrInferenceInspection, PaddleOcrInferenceInspectionPort,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Read;
use std::path::{Path, PathBuf};

const INFERENCE_MANIFEST: &str = ".vanehub-ocr-inference.json";
const INFERENCE_WORKER: &str = "paddleocr_worker.py";
const MAX_MANIFEST_BYTES: u64 = 4 * 1024;

#[derive(Debug, Default)]
pub(crate) struct ManagedPaddleOcrReadinessInspector;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InferenceManifest {
    protocol_version: String,
    engine_version: String,
    worker_sha256: String,
}

pub(super) struct VerifiedPaddleOcrRuntime {
    pub(super) interpreter: PathBuf,
    pub(super) worker: PathBuf,
    pub(super) protocol_version: String,
    pub(super) engine_version: String,
}

impl PaddleOcrInferenceInspectionPort for ManagedPaddleOcrReadinessInspector {
    fn inspect(
        &self,
        install_path: &str,
    ) -> Result<PaddleOcrInferenceInspection, ExtensionApplicationError> {
        let target = PathBuf::from(install_path);
        if !target.is_absolute() {
            return Err(readiness_error("managed OCR install path is invalid"));
        }
        let canonical = target
            .canonicalize()
            .map_err(|_| readiness_error("managed OCR install path is unavailable"))?;
        let runtime = verified_inference_runtime(&canonical)?;
        let renderer = super::pdfium_protocol::verified_runtime(&canonical);
        Ok(PaddleOcrInferenceInspection {
            protocol_version: runtime.protocol_version,
            engine_version: runtime.engine_version,
            interpreter_ready: true,
            worker_ready: true,
            renderer_ready: renderer.is_ok(),
            renderer_version: renderer.ok().map(|(_, manifest)| manifest.renderer_version),
        })
    }
}

pub(super) fn verified_inference_runtime(
    install_path: &Path,
) -> Result<VerifiedPaddleOcrRuntime, ExtensionApplicationError> {
    let canonical = install_path
        .canonicalize()
        .map_err(|_| readiness_error("managed OCR install path is unavailable"))?;
    let interpreter = venv_python(&canonical);
    let worker = canonical.join(INFERENCE_WORKER);
    if !admitted_file(&canonical, &interpreter) || !admitted_file(&canonical, &worker) {
        return Err(readiness_error("OCR inference runtime is incomplete"));
    }
    let manifest_path = canonical.join(INFERENCE_MANIFEST);
    let metadata = std::fs::symlink_metadata(&manifest_path)
        .map_err(|_| readiness_error("OCR inference manifest is unavailable"))?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > MAX_MANIFEST_BYTES
    {
        return Err(readiness_error("OCR inference manifest is invalid"));
    }
    let bytes = std::fs::read(&manifest_path)
        .map_err(|_| readiness_error("OCR inference manifest is unreadable"))?;
    let manifest: InferenceManifest = serde_json::from_slice(&bytes)
        .map_err(|_| readiness_error("OCR inference manifest is invalid"))?;
    if !safe_version(&manifest.engine_version) {
        return Err(readiness_error("OCR engine version is invalid"));
    }
    if !is_sha256(&manifest.worker_sha256)
        || hash_file(&worker)? != manifest.worker_sha256.to_ascii_lowercase()
    {
        return Err(readiness_error("OCR inference worker integrity failed"));
    }
    Ok(VerifiedPaddleOcrRuntime {
        interpreter,
        worker,
        protocol_version: manifest.protocol_version,
        engine_version: manifest.engine_version,
    })
}

fn admitted_file(root: &Path, path: &Path) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(path) else {
        return false;
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return false;
    }
    path.canonicalize()
        .is_ok_and(|value| value.starts_with(root))
}

fn safe_version(version: &str) -> bool {
    !version.is_empty()
        && version.len() <= 64
        && version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn hash_file(path: &Path) -> Result<String, ExtensionApplicationError> {
    let mut file = std::fs::File::open(path)
        .map_err(|_| readiness_error("OCR inference worker is unreadable"))?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| readiness_error("OCR inference worker is unreadable"))?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    let bytes = digest.finalize();
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    Ok(encoded)
}

fn readiness_error(message: &str) -> ExtensionApplicationError {
    ExtensionApplicationError::Runtime(message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspection_reads_only_fixed_owned_files_and_rejects_manifest_extensions() {
        let root = tempfile::tempdir().expect("root");
        let interpreter = venv_python(root.path());
        std::fs::create_dir_all(interpreter.parent().expect("interpreter parent"))
            .expect("interpreter directory");
        std::fs::write(&interpreter, b"python").expect("interpreter");
        std::fs::write(root.path().join(INFERENCE_WORKER), b"worker").expect("worker");
        let renderer_name = if cfg!(windows) {
            "pdfium-render.exe"
        } else {
            "pdfium-render"
        };
        std::fs::write(root.path().join(renderer_name), b"renderer").expect("renderer");
        let renderer_hash = hash_file(&root.path().join(renderer_name)).expect("renderer hash");
        std::fs::write(
            root.path().join(".vanehub-pdfium-render.json"),
            format!(
                r#"{{"protocolVersion":"vanehub.pdfium.render.v1","rendererVersion":"1.0.0","binarySha256":"{renderer_hash}"}}"#
            ),
        )
        .expect("renderer manifest");
        std::fs::write(
            root.path().join(INFERENCE_MANIFEST),
            format!(
                r#"{{"protocolVersion":"vanehub.paddleocr.inference.v1","engineVersion":"3.2.0","workerSha256":"{}"}}"#,
                hash_file(&root.path().join(INFERENCE_WORKER)).expect("hash")
            ),
        )
        .expect("manifest");
        let inspection = ManagedPaddleOcrReadinessInspector
            .inspect(&root.path().to_string_lossy())
            .expect("inspection");
        assert!(inspection.interpreter_ready);
        assert!(inspection.worker_ready);

        std::fs::write(
            root.path().join(INFERENCE_MANIFEST),
            format!(
                r#"{{"protocolVersion":"vanehub.paddleocr.inference.v1","engineVersion":"3.2.0","workerSha256":"{}","endpoint":"https://remote.invalid"}}"#,
                hash_file(&root.path().join(INFERENCE_WORKER)).expect("hash")
            ),
        )
        .expect("extended manifest");
        assert!(ManagedPaddleOcrReadinessInspector
            .inspect(&root.path().to_string_lossy())
            .is_err());
    }
}
