use crate::contexts::code_execution::application::{
    SandboxBackendCapabilities, SandboxBackendError, SandboxLaunchRequest, SandboxProcess,
    SandboxProcessBackend,
};

#[derive(Debug, Default)]
pub(crate) struct PlatformSandboxBackend;

impl SandboxProcessBackend for PlatformSandboxBackend {
    fn capabilities(&self) -> SandboxBackendCapabilities {
        platform_capabilities()
    }

    fn launch(
        &self,
        request: SandboxLaunchRequest,
    ) -> Result<Box<dyn SandboxProcess>, SandboxBackendError> {
        validate_launch(&request)?;
        if !self.capabilities().ready() {
            return Err(SandboxBackendError::IsolationUnavailable);
        }
        #[cfg(windows)]
        {
            super::windows_appcontainer::launch(request)
        }
        #[cfg(not(windows))]
        {
            let _ = request;
            Err(SandboxBackendError::IsolationUnavailable)
        }
    }
}

fn validate_launch(request: &SandboxLaunchRequest) -> Result<(), SandboxBackendError> {
    if !request.executable.is_absolute()
        || !request.working_directory.is_absolute()
        || request.arguments.len() > 32
        || request
            .arguments
            .iter()
            .any(|value| value.is_empty() || value.contains('\0'))
        || request.environment.len() > 16
        || request.environment.iter().any(|(key, value)| {
            key.is_empty()
                || key.len() > 64
                || !key
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte == b'_')
                || value.len() > 4096
                || value.contains('\0')
        })
    {
        return Err(SandboxBackendError::InvalidLaunch);
    }
    Ok(())
}

#[cfg(windows)]
fn platform_capabilities() -> SandboxBackendCapabilities {
    super::windows_appcontainer::capabilities()
}

#[cfg(not(windows))]
fn platform_capabilities() -> SandboxBackendCapabilities {
    SandboxBackendCapabilities {
        restricted_identity: false,
        job_cpu_limit: false,
        job_memory_limit: false,
        job_process_limit: false,
        kill_process_tree: false,
        acl_confinement: false,
        network_denied: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::code_execution::application::CodeExecutionLimits;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    #[test]
    #[cfg(not(windows))]
    fn unavailable_platform_backend_fails_closed_without_spawning() {
        let backend = PlatformSandboxBackend;
        assert!(!backend.capabilities().ready());
        let root = std::env::current_dir().expect("cwd");
        let request = SandboxLaunchRequest {
            executable: root.join("runtime.exe"),
            arguments: vec!["source.py".to_owned()],
            working_directory: root,
            environment: windows_environment(),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        assert!(matches!(
            backend.launch(request),
            Err(SandboxBackendError::IsolationUnavailable)
        ));
    }

    #[cfg(windows)]
    #[test]
    fn windows_backend_exposes_only_the_complete_appcontainer_profile() {
        let backend = PlatformSandboxBackend;
        assert!(backend.capabilities().ready());
    }

    #[cfg(windows)]
    #[test]
    fn appcontainer_executes_inside_an_acl_granted_job() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join("inputs")).expect("inputs");
        let work = workspace.path().join("work");
        std::fs::create_dir(&work).expect("work");
        std::fs::create_dir(workspace.path().join("outputs")).expect("outputs");
        let request = SandboxLaunchRequest {
            executable: windows_directory().join("System32").join("whoami.exe"),
            arguments: Vec::new(),
            working_directory: work,
            environment: windows_environment(),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        let mut process = PlatformSandboxBackend.launch(request).expect("launch");
        let observation = process
            .wait_until(std::time::Instant::now() + std::time::Duration::from_secs(5))
            .expect("wait")
            .expect("exit");
        assert_eq!(observation.exit_code, 0);
        assert!(!observation.stdout.is_empty());
        assert!(observation.cpu_time_ms.is_some());
        assert!(observation.peak_memory_bytes.is_some());
    }

    #[cfg(windows)]
    #[test]
    fn appcontainer_without_capabilities_cannot_reach_loopback() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("listener");
        listener.set_nonblocking(true).expect("nonblocking");
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join("inputs")).expect("inputs");
        let work = workspace.path().join("work");
        std::fs::create_dir(&work).expect("work");
        std::fs::create_dir(workspace.path().join("outputs")).expect("outputs");
        let windows = windows_directory();
        let request = SandboxLaunchRequest {
            executable: windows.join("System32").join("curl.exe"),
            arguments: vec![
                "--connect-timeout".to_owned(),
                "1".to_owned(),
                "--max-time".to_owned(),
                "2".to_owned(),
                format!("http://{}/", listener.local_addr().expect("address")),
            ],
            working_directory: work,
            environment: windows_environment(),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        let mut process = PlatformSandboxBackend.launch(request).expect("launch");
        let observation = process
            .wait_until(std::time::Instant::now() + std::time::Duration::from_secs(5))
            .expect("wait")
            .expect("exit");
        assert_ne!(observation.exit_code, 0);
        assert!(matches!(
            listener.accept(),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock
        ));
    }

    #[cfg(windows)]
    #[test]
    fn appcontainer_cannot_read_a_sibling_outside_the_granted_workspace() {
        let base = tempfile::tempdir().expect("base");
        let secret = base.path().join("host-secret.txt");
        std::fs::write(&secret, "must-not-cross-boundary").expect("secret");
        let workspace = base.path().join("sandbox");
        std::fs::create_dir(&workspace).expect("workspace");
        std::fs::create_dir(workspace.join("inputs")).expect("inputs");
        let work = workspace.join("work");
        std::fs::create_dir(&work).expect("work");
        std::fs::create_dir(workspace.join("outputs")).expect("outputs");
        let windows = windows_directory();
        let request = SandboxLaunchRequest {
            executable: windows.join("System32").join("findstr.exe"),
            arguments: vec![
                "/R".to_owned(),
                ".*".to_owned(),
                secret.to_string_lossy().into_owned(),
            ],
            working_directory: work,
            environment: windows_environment(),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        let mut process = PlatformSandboxBackend.launch(request).expect("launch");
        let observation = process
            .wait_until(std::time::Instant::now() + std::time::Duration::from_secs(5))
            .expect("wait")
            .expect("exit");
        assert_ne!(observation.exit_code, 0);
        assert!(!String::from_utf8_lossy(&observation.stdout).contains("must-not-cross-boundary"));
    }

    #[cfg(windows)]
    #[test]
    fn job_termination_stops_a_running_descendant_tree() {
        let workspace = tempfile::tempdir().expect("workspace");
        std::fs::create_dir(workspace.path().join("inputs")).expect("inputs");
        let work = workspace.path().join("work");
        std::fs::create_dir(&work).expect("work");
        std::fs::create_dir(workspace.path().join("outputs")).expect("outputs");
        std::fs::write(work.join("child.cmd"), ":loop\r\ngoto loop\r\n").expect("child script");
        std::fs::write(
            work.join("loop.cmd"),
            "start \"\" /b cmd.exe /d /c child.cmd\r\n:loop\r\ngoto loop\r\n",
        )
        .expect("loop script");
        let windows = windows_directory();
        let request = SandboxLaunchRequest {
            executable: windows.join("System32").join("cmd.exe"),
            arguments: vec!["/d".to_owned(), "/c".to_owned(), "loop.cmd".to_owned()],
            working_directory: work,
            environment: windows_environment(),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        let mut process = PlatformSandboxBackend.launch(request).expect("launch");
        assert!(process
            .wait_until(std::time::Instant::now() + std::time::Duration::from_millis(100))
            .expect("wait")
            .is_none());
        process
            .terminate_tree(std::time::Duration::from_secs(2))
            .expect("terminate tree");
    }

    #[test]
    fn free_form_or_relative_launch_material_is_rejected_before_readiness() {
        let backend = PlatformSandboxBackend;
        let request = SandboxLaunchRequest {
            executable: PathBuf::from("cmd.exe"),
            arguments: vec!["/c".to_owned(), "whoami".to_owned()],
            working_directory: PathBuf::from("."),
            environment: BTreeMap::from([("Path".to_owned(), "ambient".to_owned())]),
            limits: CodeExecutionLimits::HARD_CEILING,
        };
        assert!(matches!(
            backend.launch(request),
            Err(SandboxBackendError::InvalidLaunch)
        ));
    }

    #[cfg(windows)]
    fn windows_directory() -> PathBuf {
        PathBuf::from(
            std::env::var_os("SystemRoot")
                .or_else(|| std::env::var_os("WINDIR"))
                .unwrap_or_else(|| "C:\\Windows".into()),
        )
    }

    #[cfg(windows)]
    fn windows_environment() -> BTreeMap<String, String> {
        let root = windows_directory();
        let mut environment = BTreeMap::from([
            ("SYSTEMROOT".to_owned(), root.to_string_lossy().into_owned()),
            ("WINDIR".to_owned(), root.to_string_lossy().into_owned()),
            ("SYSTEMDRIVE".to_owned(), "C:".to_owned()),
            (
                "COMSPEC".to_owned(),
                root.join("System32/cmd.exe").to_string_lossy().into_owned(),
            ),
            ("PATHEXT".to_owned(), ".COM;.EXE;.BAT;.CMD".to_owned()),
            (
                "PATH".to_owned(),
                root.join("System32").to_string_lossy().into_owned(),
            ),
        ]);
        for key in ["LOCALAPPDATA", "TEMP", "TMP"] {
            if let Ok(value) = std::env::var(key) {
                environment.insert(key.to_owned(), value);
            }
        }
        environment
    }
}
