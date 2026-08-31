use super::invocation_budget::SkillToolInvocationBudget;
use crate::contexts::tooling::skill_tools::application::SkillToolApplicationError;
use crate::contexts::tooling::skill_tools::domain::{SkillProcessPermissions, SkillToolLimits};
use crate::platform::filesystem::BoundedFilesystem;
use crate::platform::process::{ProcessAdapter, ProcessCancellation, ProcessError, ProcessRequest};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub(crate) struct SkillToolProcessRequest {
    pub(crate) executable: String,
    pub(crate) arguments: Vec<String>,
    pub(crate) current_directory: String,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) cancellation: Option<ProcessCancellation>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct SkillToolProcessOutput {
    pub(crate) success: bool,
    pub(crate) status: String,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) truncated: bool,
}

pub(crate) struct SkillToolProcessGateway {
    boundary: BoundedFilesystem,
    permissions: SkillProcessPermissions,
    executables: BTreeMap<String, PathBuf>,
    limits: SkillToolLimits,
    budget: SkillToolInvocationBudget,
}

impl SkillToolProcessGateway {
    pub(crate) fn new(
        workspace_root: &Path,
        permissions: SkillProcessPermissions,
        limits: SkillToolLimits,
    ) -> Result<Self, SkillToolApplicationError> {
        Self::with_budget(
            workspace_root,
            permissions,
            limits,
            SkillToolInvocationBudget::new(limits),
        )
    }

    pub(crate) fn with_budget(
        workspace_root: &Path,
        permissions: SkillProcessPermissions,
        limits: SkillToolLimits,
        budget: SkillToolInvocationBudget,
    ) -> Result<Self, SkillToolApplicationError> {
        let executables = permissions
            .commands
            .iter()
            .map(|command| {
                resolve_executable(&command.executable)
                    .map(|path| (command.executable.clone(), path))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self {
            boundary: BoundedFilesystem::new(workspace_root).map_err(|_| denied("process.cwd"))?,
            permissions,
            executables,
            limits,
            budget,
        })
    }

    pub(crate) fn execute(
        &mut self,
        request: SkillToolProcessRequest,
    ) -> Result<SkillToolProcessOutput, SkillToolApplicationError> {
        let declaration = self
            .permissions
            .commands
            .iter()
            .find(|command| {
                command.executable == request.executable && command.arguments == request.arguments
            })
            .ok_or_else(|| denied("process.command"))?;
        if request
            .environment
            .keys()
            .any(|key| !declaration.environment.contains(key))
        {
            return Err(denied("process.environment"));
        }
        let current_directory = canonical_directory(&self.boundary, &request.current_directory)?;
        let executable = self
            .executables
            .get(&request.executable)
            .ok_or_else(|| denied("process.executable"))?
            .clone();
        let _permit = self.budget.enter_child()?;
        let output_limit = usize::try_from(self.limits.output_bytes / 2).unwrap_or(usize::MAX);
        let mut process = ProcessRequest::new(executable.into_os_string())
            .args(request.arguments)
            .current_dir(current_directory)
            .env_clear()
            .timeout(self.budget.remaining_time()?)
            .output_limit(output_limit);
        for (key, value) in request.environment {
            process = process.env(key, value);
        }
        if let Some(cancellation) = request.cancellation {
            process = process.cancellation(cancellation);
        }
        let output = ProcessAdapter.execute(&process).map_err(process_error)?;
        self.budget
            .consume_output((output.stdout_bytes.len() + output.stderr_bytes.len()) as u64)?;
        Ok(SkillToolProcessOutput {
            success: output.success(),
            status: output.status_label(),
            stdout: output.stdout,
            stderr: output.stderr,
            truncated: output.output_truncated,
        })
    }
}

fn resolve_executable(identity: &str) -> Result<PathBuf, SkillToolApplicationError> {
    let search_path = std::env::var_os("PATH").ok_or_else(|| denied("process.executable"))?;
    let extensions: &[&str] = if cfg!(windows) { &["", ".exe"] } else { &[""] };
    std::env::split_paths(&search_path)
        .flat_map(|directory| {
            extensions
                .iter()
                .map(move |extension| directory.join(format!("{identity}{extension}")))
        })
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| candidate.canonicalize().ok())
        .ok_or_else(|| denied("process.executable"))
}

fn canonical_directory(
    boundary: &BoundedFilesystem,
    relative: &str,
) -> Result<PathBuf, SkillToolApplicationError> {
    let path = boundary
        .resolve_existing(relative)
        .map_err(|_| denied("process.cwd"))?;
    path.is_dir()
        .then_some(path)
        .ok_or_else(|| denied("process.cwd"))
}

fn process_error(error: ProcessError) -> SkillToolApplicationError {
    match error {
        ProcessError::TimedOut { .. } | ProcessError::Cancelled { .. } => {
            SkillToolApplicationError::ResourceLimit("process.wall-time".to_string())
        }
        _ => SkillToolApplicationError::Filesystem("process execution failed".to_string()),
    }
}

fn denied(capability: &str) -> SkillToolApplicationError {
    SkillToolApplicationError::HostDenied(capability.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skill_tools::domain::{
        SkillProcessCommand, SkillToolLimits, DEFAULT_SKILL_TOOL_LIMITS,
    };
    use crate::test_support::TempDirectory;

    /// The default limits with a wall-time nobody here is testing.
    ///
    /// These tests spawn a real `rustc` and assert things that have nothing to do with how long it
    /// takes: that an argument stays one literal token, that a denied environment variable is
    /// refused, that the child ceiling holds. The shipped ten-second ceiling is the *product's*
    /// bound on a skill's whole invocation, and on a loaded Windows runner — where this step runs
    /// straight after `cargo build`, with as many of these tests in parallel as there are cores —
    /// one `rustc` startup has exceeded it. Widening costs no discriminating power: the assertions
    /// below are about what the child did, not about when.
    ///
    /// The one test that *is* about a ceiling names the ceiling it means.
    fn unhurried_limits() -> SkillToolLimits {
        SkillToolLimits {
            wall_time_milliseconds: 120_000,
            ..DEFAULT_SKILL_TOOL_LIMITS
        }
    }

    fn permissions(arguments: &[&str], environment: &[&str]) -> SkillProcessPermissions {
        SkillProcessPermissions {
            commands: vec![SkillProcessCommand {
                executable: "rustc".to_string(),
                arguments: arguments.iter().map(|value| (*value).to_string()).collect(),
                environment: environment
                    .iter()
                    .map(|value| (*value).to_string())
                    .collect(),
            }],
        }
    }

    fn request(arguments: &[&str]) -> SkillToolProcessRequest {
        SkillToolProcessRequest {
            executable: "rustc".to_string(),
            arguments: arguments.iter().map(|value| (*value).to_string()).collect(),
            current_directory: ".".to_string(),
            environment: BTreeMap::new(),
            cancellation: None,
        }
    }

    #[test]
    fn exact_structured_command_executes_without_a_shell() {
        let workspace = TempDirectory::new("skill-process-workspace");
        let mut gateway = SkillToolProcessGateway::new(
            workspace.path(),
            permissions(&["--version"], &[]),
            unhurried_limits(),
        )
        .expect("gateway");

        let output = gateway
            .execute(request(&["--version"]))
            .expect("execute rustc");
        assert!(output.success);
        assert!(!output.truncated);
    }

    #[test]
    fn argument_environment_cwd_and_child_bounds_fail_closed() {
        let workspace = TempDirectory::new("skill-process-adversarial");
        let mut limits = unhurried_limits();
        limits.child_processes = 1;
        let mut gateway = SkillToolProcessGateway::new(
            workspace.path(),
            permissions(&["--version"], &["LANG"]),
            limits,
        )
        .expect("gateway");

        assert!(gateway.execute(request(&["--version;whoami"])).is_err());
        let mut bad_environment = request(&["--version"]);
        bad_environment
            .environment
            .insert("SECRET".to_string(), "value".to_string());
        assert!(gateway.execute(bad_environment).is_err());
        let mut bad_cwd = request(&["--version"]);
        bad_cwd.current_directory = "../outside".to_string();
        assert!(gateway.execute(bad_cwd).is_err());
        gateway
            .execute(request(&["--version"]))
            .expect("first child");
        // Named, not wildcarded. `ResourceLimit` is also what a wall-time timeout produces, so a
        // wildcard here passes when the second `rustc` merely ran long — which is the assertion
        // succeeding for the opposite of the reason it claims.
        // Named, not wildcarded. `ResourceLimit` is also what a wall-time timeout produces, so a
        // wildcard here passes when the second `rustc` merely ran long — the assertion succeeding
        // for the opposite of the reason it claims. `aggregate` is as specific as the production
        // code allows: six ceilings share that one code, and only the timeout is excluded by it.
        assert!(matches!(
            gateway.execute(request(&["--version"])),
            Err(SkillToolApplicationError::ResourceLimit(ref code)) if code == "aggregate"
        ));
    }

    #[test]
    fn admitted_shell_metacharacters_remain_one_literal_argument() {
        let workspace = TempDirectory::new("skill-process-literal-argument");
        let marker = workspace.path().join("must-not-exist");
        let argument = format!("--version;touch={}", marker.to_string_lossy());
        let mut gateway = SkillToolProcessGateway::new(
            workspace.path(),
            permissions(&[&argument], &[]),
            unhurried_limits(),
        )
        .expect("gateway");

        let output = gateway.execute(request(&[&argument])).expect("direct exec");
        assert!(!output.success);
        assert!(!marker.exists());
    }
}
