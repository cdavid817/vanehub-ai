use crate::contexts::code_execution::application::CodeExecutionLimits;
use crate::contexts::tooling::extensions::application::{
    OcrAdmissionLimits, OcrExecutionError, RenderedOcrPage,
};
use crate::platform::filesystem::create_new_file;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub(super) const PROTOCOL_VERSION: &str = "vanehub.pdfium.render.v1";
pub(super) const RENDER_DURATION_MS: u64 = 30_000;
const MANIFEST_FILE: &str = ".vanehub-pdfium-render.json";
const BINARY_FILE: &str = if cfg!(windows) {
    "pdfium-render.exe"
} else {
    "pdfium-render"
};
const MAX_MANIFEST_BYTES: u64 = 4 * 1024;
const MAX_BINARY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_RESULT_BYTES: u64 = 128 * 1024;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RendererManifest {
    pub(super) protocol_version: String,
    pub(super) renderer_version: String,
    pub(super) binary_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RendererRequest<'a> {
    pub(super) protocol_version: &'static str,
    pub(super) action: &'static str,
    pub(super) source_path: &'a Path,
    pub(super) page_numbers: &'a [u32],
    pub(super) output_directory: Option<&'a Path>,
    pub(super) limits: Option<OcrAdmissionLimitsWire>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct OcrAdmissionLimitsWire {
    pub(super) image_width: u32,
    pub(super) image_height: u32,
    pub(super) image_pixels: u64,
    pub(super) rendered_page_bytes: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RendererResult {
    pub(super) protocol_version: String,
    pub(super) renderer_version: String,
    pub(super) action: String,
    pub(super) page_count: Option<u32>,
    pub(super) pages: Vec<RendererPage>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct RendererPage {
    pub(super) page_number: u32,
    pub(super) file_name: String,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) size_bytes: u64,
}

pub(super) fn verified_runtime(
    install_path: &Path,
) -> Result<(PathBuf, RendererManifest), OcrExecutionError> {
    let root = install_path
        .canonicalize()
        .map_err(|_| OcrExecutionError::RenderFailure)?;
    let manifest: RendererManifest = read_bounded_json(&root.join(MANIFEST_FILE))?;
    if manifest.protocol_version != PROTOCOL_VERSION
        || !safe_version(&manifest.renderer_version)
        || !is_sha256(&manifest.binary_sha256)
    {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    let binary = root.join(BINARY_FILE);
    let metadata = admitted_regular_file(&root, &binary, MAX_BINARY_BYTES)?;
    if metadata.len() == 0 || hash_file(&binary)? != manifest.binary_sha256.to_ascii_lowercase() {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    Ok((binary, manifest))
}

pub(super) fn validate_render_result(
    result: RendererResult,
    requested: &[u32],
    output: &Path,
    limits: OcrAdmissionLimits,
) -> Result<Vec<RenderedOcrPage>, OcrExecutionError> {
    if result.action != "render"
        || result.page_count.is_some()
        || result.pages.len() != requested.len()
    {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    result
        .pages
        .into_iter()
        .zip(requested)
        .map(|(page, expected)| validate_page(page, *expected, output, limits))
        .collect()
}

fn validate_page(
    page: RendererPage,
    expected: u32,
    output: &Path,
    limits: OcrAdmissionLimits,
) -> Result<RenderedOcrPage, OcrExecutionError> {
    let expected_name = format!("page-{expected}.png");
    if page.page_number != expected || page.file_name != expected_name {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    let path = output.join(expected_name);
    let metadata = admitted_regular_file(output, &path, limits.rendered_page_bytes)?;
    let bytes = std::fs::read(&path).map_err(|_| OcrExecutionError::RenderFailure)?;
    let dimensions = png_dimensions(&bytes).ok_or(OcrExecutionError::ProtocolViolation)?;
    let pixels = u64::from(dimensions.0).saturating_mul(u64::from(dimensions.1));
    if metadata.len() != page.size_bytes
        || dimensions != (page.width, page.height)
        || page.width == 0
        || page.height == 0
        || page.width > limits.image_width
        || page.height > limits.image_height
        || pixels > limits.image_pixels
    {
        return Err(OcrExecutionError::LimitExceeded);
    }
    Ok(RenderedOcrPage {
        page_number: page.page_number,
        path,
        width: page.width,
        height: page.height,
        size_bytes: page.size_bytes,
    })
}

pub(super) fn renderer_process_limits() -> CodeExecutionLimits {
    CodeExecutionLimits {
        wall_time_ms: RENDER_DURATION_MS,
        cpu_time_ms: RENDER_DURATION_MS,
        memory_bytes: 512 * 1024 * 1024,
        process_count: 1,
        stdout_bytes: 64 * 1024,
        stderr_bytes: 64 * 1024,
        filesystem_bytes: 32 * 1024 * 1024 * 32,
        file_count: 32,
        event_count: 64,
    }
}

pub(super) fn admitted_regular_file(
    root: &Path,
    path: &Path,
    maximum: u64,
) -> Result<std::fs::Metadata, OcrExecutionError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| OcrExecutionError::RenderFailure)?;
    let canonical_root = root
        .canonicalize()
        .map_err(|_| OcrExecutionError::RenderFailure)?;
    let canonical = path
        .canonicalize()
        .map_err(|_| OcrExecutionError::RenderFailure)?;
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > maximum
        || !canonical.starts_with(canonical_root)
    {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    Ok(metadata)
}

pub(super) fn ensure_directory(path: &Path) -> Result<(), OcrExecutionError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(OcrExecutionError::RenderFailure),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            std::fs::create_dir(path).map_err(|_| OcrExecutionError::RenderFailure)
        }
        Err(_) => Err(OcrExecutionError::RenderFailure),
    }
}

pub(super) fn write_new_json(path: &Path, value: &impl Serialize) -> Result<(), OcrExecutionError> {
    let bytes = serde_json::to_vec(value).map_err(|_| OcrExecutionError::InvalidRequest)?;
    let mut file = create_new_file(path).map_err(|_| OcrExecutionError::RenderFailure)?;
    file.write_all(&bytes)
        .map_err(|_| OcrExecutionError::RenderFailure)
}

pub(super) fn read_bounded_json<T: for<'de> Deserialize<'de>>(
    path: &Path,
) -> Result<T, OcrExecutionError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| OcrExecutionError::RenderFailure)?;
    let maximum = if path.file_name().is_some_and(|name| name == MANIFEST_FILE) {
        MAX_MANIFEST_BYTES
    } else {
        MAX_RESULT_BYTES
    };
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > maximum {
        return Err(OcrExecutionError::ProtocolViolation);
    }
    let bytes = std::fs::read(path).map_err(|_| OcrExecutionError::RenderFailure)?;
    serde_json::from_slice(&bytes).map_err(|_| OcrExecutionError::ProtocolViolation)
}

fn hash_file(path: &Path) -> Result<String, OcrExecutionError> {
    let mut file = std::fs::File::open(path).map_err(|_| OcrExecutionError::RenderFailure)?;
    let mut digest = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|_| OcrExecutionError::RenderFailure)?;
        if count == 0 {
            break;
        }
        digest.update(&buffer[..count]);
    }
    Ok(hex_digest(&digest.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn png_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    if bytes.len() < 24 || !bytes.starts_with(b"\x89PNG\r\n\x1a\n") || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some((
        u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        u32::from_be_bytes(bytes[20..24].try_into().ok()?),
    ))
}

fn safe_version(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub(super) fn minimal_environment() -> BTreeMap<String, String> {
    let environment = BTreeMap::from([("NO_PROXY".to_owned(), "*".to_owned())]);
    #[cfg(windows)]
    {
        let mut environment = environment;
        let root = std::env::var("SystemRoot")
            .or_else(|_| std::env::var("WINDIR"))
            .unwrap_or_else(|_| "C:\\Windows".to_owned());
        environment.insert("SYSTEMROOT".to_owned(), root.clone());
        environment.insert("WINDIR".to_owned(), root.clone());
        environment.insert("PATH".to_owned(), format!("{root}\\System32"));
        environment
    }
    #[cfg(not(windows))]
    {
        environment
    }
}

pub(super) struct ControlFiles {
    request: PathBuf,
    result: PathBuf,
}

impl ControlFiles {
    pub(super) fn new(request: PathBuf, result: PathBuf) -> Self {
        Self { request, result }
    }
}

impl Drop for ControlFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.request);
        let _ = std::fs::remove_file(&self.result);
    }
}
