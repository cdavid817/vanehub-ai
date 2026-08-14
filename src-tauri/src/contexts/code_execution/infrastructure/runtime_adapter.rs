use crate::contexts::code_execution::application::{
    CodeRuntime, CodeRuntimePort, CodeServiceError, RuntimeCatalog, RuntimeVersion,
};
use crate::platform::process::{ProcessAdapter, ProcessRequest};
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Default)]
pub(crate) struct SystemCodeRuntimeAdapter;

impl CodeRuntimePort for SystemCodeRuntimeAdapter {
    fn resolve_reviewed(
        &self,
        runtime: CodeRuntime,
    ) -> Result<(PathBuf, RuntimeVersion), CodeServiceError> {
        let reviewed = RuntimeCatalog::reviewed(runtime);
        for name in reviewed.executable_names {
            if let Some(executable) = resolve_executable(name) {
                let request = ProcessRequest::new(executable.as_os_str())
                    .arg(reviewed.version_argument)
                    .timeout(Duration::from_secs(3))
                    .output_limit(4096);
                let Ok(output) = ProcessAdapter.execute(&request) else {
                    continue;
                };
                if !output.success() {
                    continue;
                }
                let version_output = if output.stdout.trim().is_empty() {
                    output.stderr.as_str()
                } else {
                    output.stdout.as_str()
                };
                if let Ok(version) = RuntimeCatalog::parse_version(runtime, version_output) {
                    return Ok((executable, version));
                }
            }
        }
        Err(CodeServiceError::RuntimeUnavailable)
    }
}

fn resolve_executable(name: &str) -> Option<PathBuf> {
    let resolver = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let request = ProcessRequest::new(resolver)
        .arg(name)
        .timeout(Duration::from_secs(3))
        .output_limit(16 * 1024);
    let output = ProcessAdapter.execute(&request).ok()?;
    if !output.success() {
        return None;
    }
    output
        .stdout
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .find_map(|path| std::fs::canonicalize(path).ok())
        .filter(|path| path.is_absolute() && path.is_file())
}
