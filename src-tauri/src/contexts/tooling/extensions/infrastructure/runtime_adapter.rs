use super::installation_adapter::{ensure_owned_target, venv_python};
use crate::contexts::tooling::extensions::application::{
    ExtensionApplicationError, ExtensionExecutionLog, ExtensionMutationPort, ExtensionRuntimePort,
};
use crate::contexts::tooling::extensions::domain::{
    ExtensionFrameworkId, ExtensionRuntimeObservation, RuntimePlan,
};
use std::collections::{HashMap, HashSet};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const HEALTH_CONNECT_TIMEOUT: Duration = Duration::from_millis(200);
const START_HEALTH_TIMEOUT: Duration = Duration::from_secs(3);
const START_HEALTH_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone)]
pub(crate) struct OwnedExtensionRuntime {
    inner: Arc<RuntimeInner>,
}

struct RuntimeInner {
    root: PathBuf,
    active_operations: Mutex<HashSet<ExtensionFrameworkId>>,
    children: Mutex<HashMap<ExtensionFrameworkId, Child>>,
}

impl Drop for RuntimeInner {
    fn drop(&mut self) {
        if let Ok(children) = self.children.get_mut() {
            for child in children.values_mut() {
                let _ = child.kill();
                let _ = child.wait();
            }
        }
    }
}

impl OwnedExtensionRuntime {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            inner: Arc::new(RuntimeInner {
                root,
                active_operations: Mutex::new(HashSet::new()),
                children: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn target(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<PathBuf, ExtensionApplicationError> {
        let target = self.inner.root.join(framework_id.as_str());
        ensure_owned_target(&self.inner.root, &target, framework_id)?;
        Ok(target)
    }

    fn running(
        &self,
        framework_id: ExtensionFrameworkId,
    ) -> Result<bool, ExtensionApplicationError> {
        let mut children = self.inner.children.lock().map_err(runtime_lock_error)?;
        let exited = match children.get_mut(&framework_id) {
            Some(child) => child.try_wait().map_err(runtime_error)?.is_some(),
            None => return Ok(false),
        };
        if exited {
            children.remove(&framework_id);
            Ok(false)
        } else {
            Ok(true)
        }
    }
}

impl ExtensionMutationPort for OwnedExtensionRuntime {
    fn begin(&self, framework_id: ExtensionFrameworkId) -> Result<(), ExtensionApplicationError> {
        let mut active = self
            .inner
            .active_operations
            .lock()
            .map_err(runtime_lock_error)?;
        if !active.insert(framework_id) {
            return Err(ExtensionApplicationError::ConcurrentMutation(format!(
                "an extension operation is already running for {}",
                framework_id.as_str()
            )));
        }
        Ok(())
    }

    fn finish(&self, framework_id: ExtensionFrameworkId) {
        if let Ok(mut active) = self.inner.active_operations.lock() {
            active.remove(&framework_id);
        }
    }
}

impl ExtensionRuntimePort for OwnedExtensionRuntime {
    fn observe(
        &self,
        framework_id: ExtensionFrameworkId,
        port: u16,
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        if !self.running(framework_id)? {
            return Ok(ExtensionRuntimeObservation::stopped());
        }
        if loopback_is_listening(port) {
            Ok(ExtensionRuntimeObservation::healthy())
        } else {
            Ok(ExtensionRuntimeObservation {
                owned_process_running: true,
                healthy: false,
                error: Some("extension sidecar health check failed".to_string()),
            })
        }
    }

    fn start(
        &self,
        _operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        let current = self.observe(plan.framework_id, plan.port)?;
        if current.owned_process_running {
            emit(ExtensionExecutionLog::info("Framework is already running"));
            return Ok(current);
        }
        if loopback_is_listening(plan.port) {
            return Err(ExtensionApplicationError::Runtime(format!(
                "loopback port {} is already owned by another process",
                plan.port
            )));
        }

        let target = self.target(plan.framework_id)?;
        let interpreter = venv_python(&target);
        if !interpreter.is_file() {
            return Err(ExtensionApplicationError::Runtime(
                "managed framework environment is not installed".to_string(),
            ));
        }
        let args = vec![
            "-m".to_string(),
            "http.server".to_string(),
            plan.port.to_string(),
            "--bind".to_string(),
            "127.0.0.1".to_string(),
        ];
        let executable = interpreter.to_string_lossy().to_string();
        crate::platform::process::audit_command("extension.lifecycle", &executable, &args);
        let mut child = crate::platform::process::std_command(&executable)
            .map_err(runtime_error)?
            .args(&args)
            .current_dir(&target)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(runtime_error)?;
        let deadline = Instant::now() + START_HEALTH_TIMEOUT;
        while Instant::now() < deadline {
            if child.try_wait().map_err(runtime_error)?.is_some() {
                break;
            }
            if loopback_is_listening(plan.port) {
                let pid = child.id();
                self.inner
                    .children
                    .lock()
                    .map_err(runtime_lock_error)?
                    .insert(plan.framework_id, child);
                emit(ExtensionExecutionLog::info(format!(
                    "Started owned sidecar pid={pid} on 127.0.0.1:{}",
                    plan.port
                )));
                return Ok(ExtensionRuntimeObservation::healthy());
            }
            thread::sleep(START_HEALTH_INTERVAL);
        }
        let _ = child.kill();
        let _ = child.wait();
        Err(ExtensionApplicationError::Runtime(
            "extension sidecar did not become healthy".to_string(),
        ))
    }

    fn stop(
        &self,
        _operation_id: &str,
        plan: &RuntimePlan,
        emit: &mut dyn FnMut(ExtensionExecutionLog),
    ) -> Result<ExtensionRuntimeObservation, ExtensionApplicationError> {
        let child = self
            .inner
            .children
            .lock()
            .map_err(runtime_lock_error)?
            .remove(&plan.framework_id);
        let Some(mut child) = child else {
            emit(ExtensionExecutionLog::info("Framework is already stopped"));
            return Ok(ExtensionRuntimeObservation::stopped());
        };
        let pid = child.id();
        child.kill().map_err(runtime_error)?;
        let _ = child.wait();
        emit(ExtensionExecutionLog::info(format!(
            "Stopped owned sidecar pid={pid}"
        )));
        Ok(ExtensionRuntimeObservation::stopped())
    }
}

fn loopback_is_listening(port: u16) -> bool {
    TcpStream::connect_timeout(
        &SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port),
        HEALTH_CONNECT_TIMEOUT,
    )
    .is_ok()
}

fn runtime_lock_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Runtime(error.to_string())
}

fn runtime_error(error: impl std::fmt::Display) -> ExtensionApplicationError {
    ExtensionApplicationError::Runtime(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::net::TcpListener;

    #[test]
    fn mutation_guard_is_scoped_by_framework() {
        let root = TempDirectory::new("extension-runtime-lock");
        let runtime = OwnedExtensionRuntime::new(root.path().to_path_buf());
        runtime
            .begin(ExtensionFrameworkId::Paddleocr)
            .expect("first");
        assert!(runtime.begin(ExtensionFrameworkId::Paddleocr).is_err());
        assert!(runtime.begin(ExtensionFrameworkId::SherpaOnnx).is_ok());
        runtime.finish(ExtensionFrameworkId::Paddleocr);
        assert!(runtime.begin(ExtensionFrameworkId::Paddleocr).is_ok());
    }

    #[test]
    fn start_refuses_foreign_loopback_listener_without_stopping_it() {
        let root = TempDirectory::new("extension-runtime-foreign-port");
        let runtime = OwnedExtensionRuntime::new(root.path().to_path_buf());
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = listener.local_addr().expect("address").port();
        let error = runtime
            .start(
                "extension-op-1",
                &RuntimePlan {
                    framework_id: ExtensionFrameworkId::Paddleocr,
                    port,
                },
                &mut |_| {},
            )
            .expect_err("foreign listener");

        assert!(error.to_string().contains("owned by another process"));
        assert!(TcpStream::connect((Ipv4Addr::LOCALHOST, port)).is_ok());
    }

    #[test]
    fn observe_does_not_claim_an_unowned_listener_as_managed_runtime() {
        let root = TempDirectory::new("extension-runtime-observe");
        let runtime = OwnedExtensionRuntime::new(root.path().to_path_buf());
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).expect("listener");
        let port = listener.local_addr().expect("address").port();

        let observation = runtime
            .observe(ExtensionFrameworkId::Paddleocr, port)
            .expect("observation");
        assert!(!observation.owned_process_running);
        assert!(!observation.healthy);
    }
}
