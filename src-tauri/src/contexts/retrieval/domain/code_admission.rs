use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use std::collections::BTreeMap;

use super::{CodeIndexConfigurationUpdate, CodeLanguage, RetrievalError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum FileSkipReason {
    OutsideSelectedRoots,
    SensitiveFile,
    UserExcluded,
    LanguageDisabled,
    SizeLimit,
    Binary,
    Unreadable,
}

impl FileSkipReason {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::OutsideSelectedRoots => "outside_selected_roots",
            Self::SensitiveFile => "sensitive_file",
            Self::UserExcluded => "user_excluded",
            Self::LanguageDisabled => "language_disabled",
            Self::SizeLimit => "size_limit",
            Self::Binary => "binary",
            Self::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct FileSkipCounts(BTreeMap<FileSkipReason, u64>);

impl FileSkipCounts {
    pub(crate) fn record(&mut self, reason: FileSkipReason) {
        *self.0.entry(reason).or_default() += 1;
    }

    pub(crate) fn ordered(&self) -> Vec<(&'static str, u64)> {
        self.0
            .iter()
            .map(|(reason, count)| (reason.as_str(), *count))
            .collect()
    }
}

pub(crate) struct FileAdmissionPolicy {
    selected_roots: Vec<String>,
    languages: Vec<CodeLanguage>,
    exclusions: GlobSet,
    max_file_bytes: u64,
}

impl FileAdmissionPolicy {
    pub(crate) fn compile(
        configuration: &CodeIndexConfigurationUpdate,
    ) -> Result<Self, RetrievalError> {
        let validated = configuration.clone().validate()?;
        Ok(Self {
            selected_roots: validated.selected_roots,
            languages: validated.languages,
            exclusions: compile_exclusions(&validated.exclusion_patterns)?,
            max_file_bytes: validated.max_file_bytes,
        })
    }

    pub(crate) fn admit_metadata(
        &self,
        relative_path: &str,
        byte_size: u64,
    ) -> Result<CodeLanguage, FileSkipReason> {
        let normalized = relative_path.replace('\\', "/");
        if !self.is_under_selected_root(&normalized) {
            return Err(FileSkipReason::OutsideSelectedRoots);
        }
        if is_mandatory_sensitive_path(&normalized) {
            return Err(FileSkipReason::SensitiveFile);
        }
        if self.exclusions.is_match(&normalized) {
            return Err(FileSkipReason::UserExcluded);
        }
        let Some(language) = CodeLanguage::from_path(&normalized) else {
            return Err(FileSkipReason::LanguageDisabled);
        };
        if !self.languages.contains(&language) {
            return Err(FileSkipReason::LanguageDisabled);
        }
        if byte_size > self.max_file_bytes {
            return Err(FileSkipReason::SizeLimit);
        }
        Ok(language)
    }

    fn is_under_selected_root(&self, relative_path: &str) -> bool {
        self.selected_roots.iter().any(|root| {
            root.is_empty()
                || relative_path == root
                || relative_path
                    .strip_prefix(root)
                    .is_some_and(|suffix| suffix.starts_with('/'))
        })
    }
}

pub(crate) fn is_mandatory_sensitive_path(relative_path: &str) -> bool {
    let normalized = relative_path.replace('\\', "/").to_ascii_lowercase();
    let mut components = normalized.split('/').filter(|part| !part.is_empty());
    let Some(file_name) = components.next_back() else {
        return false;
    };
    if components.any(is_sensitive_directory) {
        return true;
    }
    if file_name == ".env" || file_name.starts_with(".env.") {
        return true;
    }
    if matches!(
        file_name,
        "credentials"
            | "credentials.json"
            | "application_default_credentials.json"
            | "id_rsa"
            | "id_dsa"
            | "id_ecdsa"
            | "id_ed25519"
            | ".netrc"
    ) {
        return true;
    }
    matches!(
        file_name.rsplit_once('.').map(|(_, extension)| extension),
        Some("key" | "pem" | "p12" | "pfx" | "jks" | "keystore" | "crt" | "cer" | "der")
    )
}

fn is_sensitive_directory(component: &str) -> bool {
    matches!(
        component,
        ".ssh" | ".aws" | ".azure" | ".gcp" | ".kube" | "credentials" | "secrets"
    )
}

fn compile_exclusions(patterns: &[String]) -> Result<GlobSet, RetrievalError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        add_glob(&mut builder, pattern)?;
        if !pattern.contains('/') {
            add_glob(&mut builder, &format!("**/{pattern}"))?;
        }
    }
    builder
        .build()
        .map_err(|_| RetrievalError::Validation("exclusion pattern is invalid".to_string()))
}

fn add_glob(builder: &mut GlobSetBuilder, pattern: &str) -> Result<(), RetrievalError> {
    let glob = GlobBuilder::new(pattern)
        .literal_separator(true)
        .build()
        .map_err(|_| RetrievalError::Validation("exclusion pattern is invalid".to_string()))?;
    builder.add(glob);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::domain::code_index::DEFAULT_MAX_FILE_BYTES;

    fn configuration() -> CodeIndexConfigurationUpdate {
        CodeIndexConfigurationUpdate {
            enabled: true,
            selected_roots: vec![String::new()],
            languages: CodeLanguage::ALL.to_vec(),
            exclusion_patterns: vec!["*.generated.ts".to_string(), "vendor/**".to_string()],
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        }
    }

    #[test]
    fn mandatory_sensitive_patterns_are_case_normalized_and_non_overridable() {
        for path in [
            ".env",
            "config/.ENV.Local",
            "CREDENTIALS.JSON",
            "keys/server.KEY",
            "certs/server.PEM",
            ".ssh/id_rsa.pub",
            ".AWS/config",
            "secrets/app.ts",
        ] {
            assert!(is_mandatory_sensitive_path(path), "accepted {path}");
        }
        assert!(!is_mandatory_sensitive_path("src/environment.ts"));
    }

    #[test]
    fn sensitive_rules_take_precedence_over_user_exclusions() {
        let mut configuration = configuration();
        configuration.exclusion_patterns = vec!["**/*.pem".to_string()];
        let policy = FileAdmissionPolicy::compile(&configuration).expect("compile");
        assert_eq!(
            policy.admit_metadata("private/client.pem", 20),
            Err(FileSkipReason::SensitiveFile)
        );
    }

    #[test]
    fn basename_and_directory_exclusions_match_nested_paths() {
        let policy = FileAdmissionPolicy::compile(&configuration()).expect("compile");
        assert_eq!(
            policy.admit_metadata("src/api.generated.ts", 20),
            Err(FileSkipReason::UserExcluded)
        );
        assert_eq!(
            policy.admit_metadata("vendor/pkg/lib.rs", 20),
            Err(FileSkipReason::UserExcluded)
        );
    }

    #[test]
    fn selected_root_language_and_size_gates_have_stable_precedence() {
        let mut configuration = configuration();
        configuration.selected_roots = vec!["src".to_string()];
        configuration.languages = vec![CodeLanguage::Rust];
        let policy = FileAdmissionPolicy::compile(&configuration).expect("compile");
        assert_eq!(
            policy.admit_metadata("tests/main.rs", 1),
            Err(FileSkipReason::OutsideSelectedRoots)
        );
        assert_eq!(
            policy.admit_metadata("src/main.ts", 1),
            Err(FileSkipReason::LanguageDisabled)
        );
        assert_eq!(
            policy.admit_metadata("src/main.rs", DEFAULT_MAX_FILE_BYTES + 1),
            Err(FileSkipReason::SizeLimit)
        );
        assert_eq!(
            policy.admit_metadata("src/main.rs", 1),
            Ok(CodeLanguage::Rust)
        );
    }

    #[test]
    fn skip_counts_are_returned_in_enum_order_without_private_paths() {
        let mut counts = FileSkipCounts::default();
        counts.record(FileSkipReason::SizeLimit);
        counts.record(FileSkipReason::SensitiveFile);
        counts.record(FileSkipReason::SizeLimit);
        assert_eq!(
            counts.ordered(),
            vec![("sensitive_file", 1), ("size_limit", 2)]
        );
    }
}
