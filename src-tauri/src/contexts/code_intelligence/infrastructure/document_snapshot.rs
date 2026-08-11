use crate::contexts::code_intelligence::domain::models::LanguageFamily;
use crate::platform::filesystem::{BoundaryError, BoundedFilesystem};
use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub(crate) const MAX_DOCUMENT_BYTES: usize = 10 * 1024 * 1024;
const BINARY_SNIFF_BYTES: usize = 8 * 1024;

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DocumentAdmissionError {
    #[error("document path must be relative")]
    AbsolutePath,
    #[error("document path traversal is not allowed")]
    Traversal,
    #[error("hidden document paths are unavailable")]
    HiddenPath,
    #[error("document resolves outside the workspace")]
    OutsideWorkspace,
    #[error("document is unavailable")]
    Unavailable,
    #[error("document target is not a file")]
    NotFile,
    #[error("document exceeds the file-size limit")]
    FileTooLarge,
    #[error("binary document content is unavailable")]
    BinaryContent,
    #[error("document content is not valid UTF-8")]
    InvalidUtf8,
    #[error("document language is unsupported")]
    UnsupportedLanguage,
    #[error("document admission limit is invalid")]
    InvalidLimit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiskDocumentSnapshot {
    canonical_path: PathBuf,
    relative_path: String,
    language: LanguageFamily,
    language_id: &'static str,
    text: String,
}

impl DiskDocumentSnapshot {
    pub(crate) fn canonical_path(&self) -> PathBuf {
        self.canonical_path.clone()
    }

    pub(crate) fn relative_path(&self) -> &str {
        &self.relative_path
    }

    pub(crate) const fn language(&self) -> LanguageFamily {
        self.language
    }

    pub(crate) const fn language_id(&self) -> &'static str {
        self.language_id
    }

    pub(crate) fn text(&self) -> &str {
        &self.text
    }
}

pub(crate) struct DocumentAdmission {
    filesystem: BoundedFilesystem,
    max_bytes: usize,
}

impl DocumentAdmission {
    pub(crate) fn new(workspace_root: &Path) -> Result<Self, DocumentAdmissionError> {
        Self::with_max_bytes(workspace_root, MAX_DOCUMENT_BYTES)
    }

    pub(crate) fn with_max_bytes(
        workspace_root: &Path,
        max_bytes: usize,
    ) -> Result<Self, DocumentAdmissionError> {
        if max_bytes == 0 {
            return Err(DocumentAdmissionError::InvalidLimit);
        }
        let filesystem = BoundedFilesystem::new(workspace_root).map_err(map_boundary_error)?;
        Ok(Self {
            filesystem,
            max_bytes,
        })
    }

    pub(crate) fn read(
        &self,
        relative_path: &str,
    ) -> Result<DiskDocumentSnapshot, DocumentAdmissionError> {
        let canonical_path = self
            .filesystem
            .resolve_existing(relative_path)
            .map_err(map_boundary_error)?;
        let metadata = canonical_path
            .metadata()
            .map_err(|_| DocumentAdmissionError::Unavailable)?;
        if !metadata.is_file() {
            return Err(DocumentAdmissionError::NotFile);
        }
        if metadata.len() > self.max_bytes as u64 {
            return Err(DocumentAdmissionError::FileTooLarge);
        }
        let (language, language_id) = identify_language(&canonical_path)?;
        let mut bytes = Vec::with_capacity((metadata.len() as usize).min(self.max_bytes));
        File::open(&canonical_path)
            .map_err(|_| DocumentAdmissionError::Unavailable)?
            .take(self.max_bytes as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| DocumentAdmissionError::Unavailable)?;
        if bytes.len() > self.max_bytes {
            return Err(DocumentAdmissionError::FileTooLarge);
        }
        if bytes.iter().take(BINARY_SNIFF_BYTES).any(|byte| *byte == 0) {
            return Err(DocumentAdmissionError::BinaryContent);
        }
        let text = String::from_utf8(bytes).map_err(|_| DocumentAdmissionError::InvalidUtf8)?;
        Ok(DiskDocumentSnapshot {
            canonical_path,
            relative_path: normalize_relative(relative_path),
            language,
            language_id,
            text,
        })
    }
}

fn identify_language(
    path: &Path,
) -> Result<(LanguageFamily, &'static str), DocumentAdmissionError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .ok_or(DocumentAdmissionError::UnsupportedLanguage)?;
    match extension.as_str() {
        "rs" => Ok((LanguageFamily::Rust, "rust")),
        "ts" => Ok((LanguageFamily::TypeScriptJavaScript, "typescript")),
        "tsx" => Ok((LanguageFamily::TypeScriptJavaScript, "typescriptreact")),
        "js" | "mjs" | "cjs" => Ok((LanguageFamily::TypeScriptJavaScript, "javascript")),
        "jsx" => Ok((LanguageFamily::TypeScriptJavaScript, "javascriptreact")),
        _ => Err(DocumentAdmissionError::UnsupportedLanguage),
    }
}

fn normalize_relative(relative: &str) -> String {
    Path::new(relative).to_string_lossy().replace('\\', "/")
}

fn map_boundary_error(error: BoundaryError) -> DocumentAdmissionError {
    match error {
        BoundaryError::Absolute => DocumentAdmissionError::AbsolutePath,
        BoundaryError::Hidden => DocumentAdmissionError::HiddenPath,
        BoundaryError::Escape => DocumentAdmissionError::Traversal,
        BoundaryError::OutsideRoot => DocumentAdmissionError::OutsideWorkspace,
        BoundaryError::NotDirectory => DocumentAdmissionError::Unavailable,
        BoundaryError::Io(_) | BoundaryError::MissingParent | BoundaryError::MissingFileName => {
            DocumentAdmissionError::Unavailable
        }
    }
}
