use globset::Glob;
use std::path::{Component, Path};
use uuid::Uuid;

use super::error::RetrievalError;

pub(crate) const CODE_INDEX_VERSION: &str = "1";
pub(crate) const DEFAULT_MAX_FILE_BYTES: u64 = 100 * 1024;
const MAX_CONFIGURED_FILE_BYTES: u64 = 10 * 1024 * 1024;
const MAX_EXCLUSION_PATTERNS: usize = 128;
const MAX_PATTERN_LENGTH: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum CodeLanguage {
    JavaScript,
    TypeScript,
    Python,
    Rust,
    Go,
    Java,
    C,
    Cpp,
}

impl CodeLanguage {
    pub(crate) const ALL: [Self; 8] = [
        Self::JavaScript,
        Self::TypeScript,
        Self::Python,
        Self::Rust,
        Self::Go,
        Self::Java,
        Self::C,
        Self::Cpp,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Python => "python",
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Java => "java",
            Self::C => "c",
            Self::Cpp => "cpp",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|language| language.as_str() == value)
    }

    pub(crate) fn from_path(path: &str) -> Option<Self> {
        let extension = Path::new(path)
            .extension()?
            .to_string_lossy()
            .to_ascii_lowercase();
        match extension.as_str() {
            "js" | "jsx" | "mjs" | "cjs" => Some(Self::JavaScript),
            "ts" | "tsx" | "mts" | "cts" => Some(Self::TypeScript),
            "py" | "pyi" => Some(Self::Python),
            "rs" => Some(Self::Rust),
            "go" => Some(Self::Go),
            "java" => Some(Self::Java),
            "c" | "h" => Some(Self::C),
            "cc" | "cpp" | "cxx" | "hh" | "hpp" | "hxx" => Some(Self::Cpp),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeIndexConfigurationUpdate {
    pub(crate) enabled: bool,
    pub(crate) selected_roots: Vec<String>,
    pub(crate) languages: Vec<CodeLanguage>,
    pub(crate) exclusion_patterns: Vec<String>,
    pub(crate) max_file_bytes: u64,
}

impl CodeIndexConfigurationUpdate {
    pub(crate) fn validate(self) -> Result<Self, RetrievalError> {
        let Self {
            enabled,
            selected_roots,
            mut languages,
            exclusion_patterns,
            max_file_bytes,
        } = self;
        if max_file_bytes == 0 || max_file_bytes > MAX_CONFIGURED_FILE_BYTES {
            return Err(validation("file size limit is outside the supported range"));
        }
        if enabled && languages.is_empty() {
            return Err(validation(
                "at least one language is required when indexing is enabled",
            ));
        }
        if exclusion_patterns.len() > MAX_EXCLUSION_PATTERNS {
            return Err(validation("too many exclusion patterns"));
        }
        let mut selected_roots = selected_roots
            .into_iter()
            .map(normalized_config_root)
            .collect::<Result<Vec<_>, _>>()?;
        selected_roots.sort();
        selected_roots.dedup();
        if selected_roots.is_empty() {
            selected_roots.push(String::new());
        }
        for pattern in &exclusion_patterns {
            if pattern.is_empty() || pattern.len() > MAX_PATTERN_LENGTH {
                return Err(validation("exclusion pattern length is invalid"));
            }
            Glob::new(pattern).map_err(|_| validation("exclusion pattern is invalid"))?;
        }
        languages.sort_by_key(|language| language.as_str());
        languages.dedup();
        Ok(Self {
            enabled,
            selected_roots,
            languages,
            exclusion_patterns,
            max_file_bytes,
        })
    }
}

fn normalized_config_root(value: String) -> Result<String, RetrievalError> {
    if value.is_empty() || value == "." {
        return Ok(String::new());
    }
    let path = Path::new(&value);
    if path.is_absolute() {
        return Err(validation("selected root must be workspace relative"));
    }
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            _ => return Err(validation("selected root escapes the workspace")),
        }
    }
    (!parts.is_empty())
        .then(|| parts.join("/"))
        .ok_or_else(|| validation("selected root is empty"))
}

fn validation(message: &str) -> RetrievalError {
    RetrievalError::Validation(message.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CodeIndexPhase {
    Disabled,
    Scanning,
    Parsing,
    AwaitingEmbeddingConfirmation,
    Embedding,
    Ready,
    Degraded,
    Cancelling,
    Unavailable,
}

impl CodeIndexPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::Scanning => "scanning",
            Self::Parsing => "parsing",
            Self::AwaitingEmbeddingConfirmation => "awaiting_embedding_confirmation",
            Self::Embedding => "embedding",
            Self::Ready => "ready",
            Self::Degraded => "degraded",
            Self::Cancelling => "cancelling",
            Self::Unavailable => "unavailable",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "disabled" => Some(Self::Disabled),
            "scanning" => Some(Self::Scanning),
            "parsing" => Some(Self::Parsing),
            "awaiting_embedding_confirmation" => Some(Self::AwaitingEmbeddingConfirmation),
            "embedding" => Some(Self::Embedding),
            "ready" => Some(Self::Ready),
            "degraded" => Some(Self::Degraded),
            "cancelling" => Some(Self::Cancelling),
            "unavailable" => Some(Self::Unavailable),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeWorkspace {
    pub(crate) workspace_id: String,
    pub(crate) canonical_root: String,
    pub(crate) display_name: String,
    pub(crate) enabled: bool,
    pub(crate) selected_roots: Vec<String>,
    pub(crate) languages: Vec<String>,
    pub(crate) exclusion_patterns: Vec<String>,
    pub(crate) max_file_bytes: u64,
    pub(crate) index_version: String,
    pub(crate) phase: CodeIndexPhase,
    pub(crate) generation: u64,
}

impl CodeWorkspace {
    pub(crate) fn new(canonical_root: String, display_name: String) -> Self {
        Self {
            workspace_id: Uuid::new_v4().to_string(),
            canonical_root,
            display_name,
            enabled: false,
            selected_roots: vec![String::new()],
            languages: CodeLanguage::ALL
                .iter()
                .map(|language| language.as_str().to_string())
                .collect(),
            exclusion_patterns: Vec::new(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            index_version: CODE_INDEX_VERSION.to_string(),
            phase: CodeIndexPhase::Disabled,
            generation: 0,
        }
    }

    pub(crate) fn configuration(&self) -> Result<CodeIndexConfigurationUpdate, RetrievalError> {
        let languages = self
            .languages
            .iter()
            .map(|language| {
                CodeLanguage::parse(language).ok_or_else(|| validation("unsupported code language"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        CodeIndexConfigurationUpdate {
            enabled: self.enabled,
            selected_roots: self.selected_roots.clone(),
            languages,
            exclusion_patterns: self.exclusion_patterns.clone(),
            max_file_bytes: self.max_file_bytes,
        }
        .validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn update() -> CodeIndexConfigurationUpdate {
        CodeIndexConfigurationUpdate {
            enabled: true,
            selected_roots: vec!["src\\nested".to_string(), "src/nested".to_string()],
            languages: vec![
                CodeLanguage::Rust,
                CodeLanguage::Rust,
                CodeLanguage::TypeScript,
            ],
            exclusion_patterns: vec!["vendor/**".to_string(), "*.generated.ts".to_string()],
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    #[test]
    fn configuration_validation_normalizes_and_deduplicates_values() {
        let validated = update().validate().expect("valid configuration");
        assert_eq!(validated.selected_roots, vec!["src/nested"]);
        assert_eq!(
            validated.languages,
            vec![CodeLanguage::Rust, CodeLanguage::TypeScript]
        );
    }

    #[test]
    fn configuration_validation_rejects_unsafe_roots_and_invalid_globs() {
        let mut escaping = update();
        escaping.selected_roots = vec!["../outside".to_string()];
        assert!(matches!(
            escaping.validate(),
            Err(RetrievalError::Validation(_))
        ));

        let mut invalid_glob = update();
        invalid_glob.exclusion_patterns = vec!["[invalid".to_string()];
        assert!(matches!(
            invalid_glob.validate(),
            Err(RetrievalError::Validation(_))
        ));
    }

    #[test]
    fn enabled_configuration_requires_a_language_and_a_bounded_size() {
        let mut missing_language = update();
        missing_language.languages.clear();
        assert!(missing_language.validate().is_err());

        let mut oversized = update();
        oversized.max_file_bytes = MAX_CONFIGURED_FILE_BYTES + 1;
        assert!(oversized.validate().is_err());
    }

    #[test]
    fn language_detection_covers_every_supported_parser_family() {
        for (path, expected) in [
            ("a.js", CodeLanguage::JavaScript),
            ("a.tsx", CodeLanguage::TypeScript),
            ("a.py", CodeLanguage::Python),
            ("a.rs", CodeLanguage::Rust),
            ("a.go", CodeLanguage::Go),
            ("A.JAVA", CodeLanguage::Java),
            ("a.c", CodeLanguage::C),
            ("a.cpp", CodeLanguage::Cpp),
        ] {
            assert_eq!(CodeLanguage::from_path(path), Some(expected));
        }
        assert_eq!(CodeLanguage::from_path("README.md"), None);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeFileManifest {
    pub(crate) workspace_id: String,
    pub(crate) relative_path: String,
    pub(crate) language: String,
    pub(crate) byte_size: u64,
    pub(crate) modified_ns: i64,
    pub(crate) content_hash: String,
    pub(crate) index_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeChunk {
    pub(crate) source_id: String,
    pub(crate) content: String,
    pub(crate) content_hash: String,
    pub(crate) language: String,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
    pub(crate) symbol_name: Option<String>,
    pub(crate) symbol_kind: Option<String>,
    pub(crate) ordinal: u32,
    pub(crate) chunk_key: String,
    pub(crate) redaction_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeSymbol {
    pub(crate) symbol_id: String,
    pub(crate) normalized_name: String,
    pub(crate) display_name: String,
    pub(crate) symbol_kind: String,
    pub(crate) container_name: Option<String>,
    pub(crate) start_line: u32,
    pub(crate) end_line: u32,
}
