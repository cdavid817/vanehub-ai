use crate::contexts::agent_runtime::api::{
    AgentRuntimeApi, GuardedValidationCancellation, GuardedValidationRequest,
    GuardedValidationStatus, LoopVerificationCommand,
};
use crate::contexts::execution_observability::application::EvaluationVerifierPort;
use crate::contexts::execution_observability::domain::{EvaluationAcceptance, EvaluationCheck};
use std::fs;
use std::path::{Component, Path};

const PROFILE_TIMEOUT_SECONDS: u64 = 120;
const MAX_SCAN_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Clone)]
pub(crate) struct NativeEvaluationVerifierAdapter {
    agents: AgentRuntimeApi,
}

impl NativeEvaluationVerifierAdapter {
    pub(crate) fn new(agents: AgentRuntimeApi) -> Self {
        Self { agents }
    }
}

pub(crate) fn verify_static_acceptance(
    workspace: &Path,
    acceptance: &EvaluationAcceptance,
) -> Result<Vec<EvaluationCheck>, String> {
    let mut checks = Vec::new();
    for expected in &acceptance.expected_files {
        let path = bounded_child(workspace, expected)?;
        checks.push(EvaluationCheck {
            check_id: format!("expected-file:{expected}"),
            passed: path.is_file(),
            summary: if path.is_file() {
                "expected file exists"
            } else {
                "expected file is missing"
            }
            .into(),
        });
    }
    let files = bounded_files(workspace)?;
    for pattern in &acceptance.forbidden_patterns {
        let found = files.iter().any(|file| {
            fs::read_to_string(file)
                .unwrap_or_default()
                .contains(pattern)
        });
        checks.push(EvaluationCheck {
            check_id: format!("forbidden-pattern:{pattern}"),
            passed: !found,
            summary: if found {
                "forbidden pattern found"
            } else {
                "forbidden pattern absent"
            }
            .into(),
        });
    }
    Ok(checks)
}

fn bounded_child(root: &Path, relative: &str) -> Result<std::path::PathBuf, String> {
    let path = Path::new(relative);
    if path.is_absolute()
        || path
            .components()
            .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err("evaluation assertion path escapes workspace".into());
    }
    Ok(root.join(path))
}

fn bounded_files(root: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    let mut bytes = 0_u64;
    while let Some(path) = pending.pop() {
        for entry in fs::read_dir(path).map_err(|error| error.to_string())? {
            let entry = entry.map_err(|error| error.to_string())?;
            let kind = entry.file_type().map_err(|error| error.to_string())?;
            if kind.is_symlink() {
                return Err("evaluation verifier rejects symlinks".into());
            }
            if kind.is_dir() {
                pending.push(entry.path());
            } else if kind.is_file() {
                bytes = bytes
                    .saturating_add(entry.metadata().map_err(|error| error.to_string())?.len());
                if bytes > MAX_SCAN_BYTES || files.len() >= 1_000 {
                    return Err("evaluation verifier scan bound exceeded".into());
                }
                files.push(entry.path());
            }
        }
    }
    Ok(files)
}

impl EvaluationVerifierPort for NativeEvaluationVerifierAdapter {
    fn verify(&self, profile: &str, workspace: &str) -> Result<EvaluationCheck, String> {
        let Some(command) = profile_command(profile)? else {
            return Ok(EvaluationCheck {
                check_id: profile.to_string(),
                passed: true,
                summary: "profile is evaluated by bounded static and diff assertions".to_string(),
            });
        };
        let result = self
            .agents
            .run_guarded_validation_cancellable(
                GuardedValidationRequest {
                    worktree_root: workspace.to_string(),
                    command,
                },
                GuardedValidationCancellation::default(),
            )
            .map_err(|error| error.to_string())?;
        Ok(EvaluationCheck {
            check_id: profile.to_string(),
            passed: result.status == GuardedValidationStatus::Passed,
            summary: result
                .output_summary
                .unwrap_or_else(|| result.status.as_str().to_string()),
        })
    }
}

fn profile_command(profile: &str) -> Result<Option<LoopVerificationCommand>, String> {
    let definition = match profile {
        "npm-test" => Some(("npm", vec!["test"])),
        "cargo-test" => Some(("cargo", vec!["test"])),
        "static-files" | "diff-rules" => None,
        _ => return Err("unknown evaluation verifier profile".to_string()),
    };
    definition
        .map(|(program, args)| {
            LoopVerificationCommand::new(
                profile.to_string(),
                program.to_string(),
                args.into_iter().map(str::to_string).collect(),
                None,
                PROFILE_TIMEOUT_SECONDS,
                true,
            )
            .map_err(|error| error.to_string())
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profiles_are_closed_and_never_accept_manifest_commands() {
        assert!(profile_command("npm-test").expect("profile").is_some());
        assert!(profile_command("cargo-test").expect("profile").is_some());
        assert!(profile_command("diff-rules").expect("profile").is_none());
        assert!(profile_command("sh -c rm").is_err());
    }

    #[test]
    fn static_assertions_are_bounded_and_deterministic() {
        let root = tempfile::tempdir().expect("temp");
        fs::create_dir(root.path().join("src")).expect("dir");
        fs::write(
            root.path().join("src/fixed.ts"),
            "export const fixed = true;",
        )
        .expect("write");
        let acceptance = EvaluationAcceptance {
            verifier_profiles: vec!["static-files".into()],
            expected_files: vec!["src/fixed.ts".into()],
            forbidden_patterns: vec!["TODO_SECRET".into()],
        };
        assert!(verify_static_acceptance(root.path(), &acceptance)
            .expect("verify")
            .iter()
            .all(|check| check.passed));
        assert!(verify_static_acceptance(
            root.path(),
            &EvaluationAcceptance {
                expected_files: vec!["../escape".into()],
                ..acceptance
            }
        )
        .is_err());
    }
}
