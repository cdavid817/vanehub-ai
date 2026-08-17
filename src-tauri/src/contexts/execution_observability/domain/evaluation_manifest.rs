use super::{EvaluationManifest, EVALUATION_SCHEMA_VERSION};
use std::path::{Component, Path};

const MAX_PROMPT_BYTES: usize = 16 * 1024;
const MAX_TIMEOUT_SECONDS: u32 = 1_200;
const ALLOWED_VERIFIERS: &[&str] = &["npm-test", "cargo-test", "static-files", "diff-rules"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum EvaluationManifestError {
    InvalidJson,
    UnsupportedSchema,
    InvalidId,
    InvalidVersion,
    InvalidFixture,
    PromptTooLarge,
    InvalidTimeout,
    UnknownVerifier,
    UnsafePattern,
}

pub(crate) fn parse_evaluation_manifest(
    input: &str,
) -> Result<EvaluationManifest, EvaluationManifestError> {
    let manifest: EvaluationManifest =
        serde_json::from_str(input).map_err(|_| EvaluationManifestError::InvalidJson)?;
    validate_evaluation_manifest(&manifest)?;
    Ok(manifest)
}

pub(crate) fn validate_evaluation_manifest(
    manifest: &EvaluationManifest,
) -> Result<(), EvaluationManifestError> {
    if manifest.schema_version != EVALUATION_SCHEMA_VERSION {
        return Err(EvaluationManifestError::UnsupportedSchema);
    }
    if manifest.version == 0 {
        return Err(EvaluationManifestError::InvalidVersion);
    }
    if !is_stable_id(&manifest.id) {
        return Err(EvaluationManifestError::InvalidId);
    }
    if manifest.prompt.is_empty() || manifest.prompt.len() > MAX_PROMPT_BYTES {
        return Err(EvaluationManifestError::PromptTooLarge);
    }
    if manifest.timeout_seconds == 0 || manifest.timeout_seconds > MAX_TIMEOUT_SECONDS {
        return Err(EvaluationManifestError::InvalidTimeout);
    }
    if !is_relative_bounded_path(&manifest.fixture) {
        return Err(EvaluationManifestError::InvalidFixture);
    }
    if manifest
        .acceptance
        .verifier_profiles
        .iter()
        .any(|item| !ALLOWED_VERIFIERS.contains(&item.as_str()))
    {
        return Err(EvaluationManifestError::UnknownVerifier);
    }
    if manifest
        .acceptance
        .expected_files
        .iter()
        .any(|path| !is_relative_bounded_path(path))
    {
        return Err(EvaluationManifestError::InvalidFixture);
    }
    if manifest
        .acceptance
        .forbidden_patterns
        .iter()
        .any(|pattern| pattern.len() > 256 || contains_shell_syntax(pattern))
    {
        return Err(EvaluationManifestError::UnsafePattern);
    }
    Ok(())
}

fn is_stable_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 80
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn is_relative_bounded_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 240
        && !Path::new(value).is_absolute()
        && Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn contains_shell_syntax(value: &str) -> bool {
    value.contains([';', '|', '`', '\n', '\r'])
        || value.contains("$(")
        || value.contains("&&")
        || value.contains("||")
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str =
        include_str!("../../../../evaluation-fixtures/fix-null-auth-token/manifest.yaml");

    #[test]
    fn accepts_bounded_json_compatible_yaml_manifest() {
        let manifest = parse_evaluation_manifest(VALID).expect("built-in manifest must be valid");
        assert_eq!(manifest.id, "fix-null-auth-token");
    }

    #[test]
    fn rejects_schema_version_timeout_and_unknown_verifier() {
        let manifest = parse_evaluation_manifest(VALID).expect("fixture");
        for changed in [
            EvaluationManifest {
                schema_version: 2,
                ..manifest.clone()
            },
            EvaluationManifest {
                timeout_seconds: 1_201,
                ..manifest.clone()
            },
            EvaluationManifest {
                acceptance: super::super::EvaluationAcceptance {
                    verifier_profiles: vec!["sh -c".into()],
                    ..manifest.acceptance.clone()
                },
                ..manifest.clone()
            },
        ] {
            assert!(validate_evaluation_manifest(&changed).is_err());
        }
    }

    #[test]
    fn rejects_traversal_absolute_paths_and_shell_patterns() {
        let manifest = parse_evaluation_manifest(VALID).expect("fixture");
        for fixture in ["../secret", "/tmp/fixture", "nested/../../escape"] {
            assert_eq!(
                validate_evaluation_manifest(&EvaluationManifest {
                    fixture: fixture.into(),
                    ..manifest.clone()
                }),
                Err(EvaluationManifestError::InvalidFixture)
            );
        }
        assert_eq!(
            validate_evaluation_manifest(&EvaluationManifest {
                acceptance: super::super::EvaluationAcceptance {
                    forbidden_patterns: vec!["safe && rm".into()],
                    ..manifest.acceptance.clone()
                },
                ..manifest
            }),
            Err(EvaluationManifestError::UnsafePattern)
        );
    }

    #[test]
    fn rejects_zero_version_and_oversized_prompt() {
        let manifest = parse_evaluation_manifest(VALID).expect("fixture");
        assert_eq!(
            validate_evaluation_manifest(&EvaluationManifest {
                version: 0,
                ..manifest.clone()
            }),
            Err(EvaluationManifestError::InvalidVersion)
        );
        assert_eq!(
            validate_evaluation_manifest(&EvaluationManifest {
                prompt: "x".repeat(MAX_PROMPT_BYTES + 1),
                ..manifest
            }),
            Err(EvaluationManifestError::PromptTooLarge)
        );
    }
}
