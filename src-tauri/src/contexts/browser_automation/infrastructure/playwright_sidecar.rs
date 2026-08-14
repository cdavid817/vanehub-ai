use crate::contexts::browser_automation::application::{
    BrowserContextPolicy, BrowserOwnership, BrowserSession, BrowserSessionError,
    BrowserSessionFactory, BrowserSidecarError, BrowserSidecarFactory, BrowserSidecarLimits,
    BrowserSidecarRequest, BrowserSidecarResponse, BrowserSidecarSupervisor,
    BrowserSidecarTransport, BROWSER_SIDECAR_PROTOCOL_VERSION,
};
use crate::platform::process::{BlockingStderrDrain, ManagedChild};
use std::collections::BTreeMap;
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

const STDERR_LIMIT: usize = 64 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct PlaywrightSidecarFactory {
    node_executable: String,
    script_path: PathBuf,
}

impl PlaywrightSidecarFactory {
    pub(crate) fn new(node_executable: String, script_path: PathBuf) -> Self {
        Self {
            node_executable,
            script_path,
        }
    }
}

impl BrowserSidecarFactory for PlaywrightSidecarFactory {
    fn spawn(
        &self,
        limits: BrowserSidecarLimits,
    ) -> Result<Box<dyn BrowserSidecarTransport>, BrowserSidecarError> {
        PlaywrightSidecarProcess::spawn(
            &self.node_executable,
            &self.script_path,
            limits.max_message_bytes,
        )
        .map(|process| Box::new(process) as Box<dyn BrowserSidecarTransport>)
    }
}

impl BrowserSessionFactory for PlaywrightSidecarFactory {
    fn create_isolated(
        &self,
        ownership: &BrowserOwnership,
        policy: BrowserContextPolicy,
    ) -> Result<Box<dyn BrowserSession>, BrowserSessionError> {
        let mut supervisor =
            BrowserSidecarSupervisor::new(BrowserSidecarLimits::default(), Arc::new(self.clone()))
                .map_err(BrowserSessionError::ProtocolFailure)?;
        supervisor
            .start()
            .map_err(BrowserSessionError::ProtocolFailure)?;
        let response = supervisor
            .request(
                "context.create",
                serde_json::json!({
                    "owner": {
                        "session_id": ownership.session_id,
                        "generation_id": ownership.generation_id
                    },
                    "policy": {
                        "incognito": true,
                        "persistent": false,
                        "import_cookies": false,
                        "extensions": false,
                        "http_credentials": false,
                        "max_lifetime_ms": policy.max_lifetime.as_millis(),
                        "max_pages": policy.max_pages,
                        "max_download_bytes": policy.max_download_bytes,
                        "max_event_count": policy.max_event_count
                    }
                }),
            )
            .map_err(BrowserSessionError::ProtocolFailure)?;
        let context_id = response
            .result
            .as_ref()
            .and_then(|value| value.get("context_id"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty() && value.len() <= 128)
            .ok_or(BrowserSessionError::UnsafeContext)?
            .to_string();
        Ok(Box::new(OwnedPlaywrightSession {
            supervisor,
            context_id,
        }))
    }
}

struct OwnedPlaywrightSession {
    supervisor: BrowserSidecarSupervisor,
    context_id: String,
}

impl BrowserSession for OwnedPlaywrightSession {
    fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        self.supervisor.request(
            method,
            serde_json::json!({"context_id": self.context_id, "input": params}),
        )
    }

    fn close(&mut self) -> Result<(), BrowserSidecarError> {
        let _ = self.supervisor.request(
            "context.close",
            serde_json::json!({"context_id": self.context_id}),
        );
        self.supervisor.shutdown()
    }
}

pub(crate) struct PlaywrightSidecarProcess {
    child: ManagedChild,
    stdin: std::process::ChildStdin,
    inbound: Receiver<Result<Vec<u8>, BrowserSidecarError>>,
    stderr: Option<BlockingStderrDrain>,
    max_message_bytes: usize,
}

impl PlaywrightSidecarProcess {
    pub(crate) fn spawn(
        node_executable: &str,
        script_path: &std::path::Path,
        max_message_bytes: usize,
    ) -> Result<Self, BrowserSidecarError> {
        Self::spawn_with_environment(
            node_executable,
            script_path,
            max_message_bytes,
            &BTreeMap::new(),
        )
    }

    fn spawn_with_environment(
        node_executable: &str,
        script_path: &std::path::Path,
        max_message_bytes: usize,
        environment: &BTreeMap<String, String>,
    ) -> Result<Self, BrowserSidecarError> {
        if !script_path.is_file() {
            return Err(BrowserSidecarError::SpawnFailed);
        }
        let args = vec![script_path.to_string_lossy().to_string()];
        let mut child = ManagedChild::spawn(node_executable, &args, environment)
            .map_err(|_| BrowserSidecarError::SpawnFailed)?;
        let stdin = child
            .take_stdin()
            .map_err(|_| BrowserSidecarError::SpawnFailed)?;
        let stdout = child
            .take_stdout()
            .map_err(|_| BrowserSidecarError::SpawnFailed)?;
        let stderr = child
            .take_stderr()
            .map_err(|_| BrowserSidecarError::SpawnFailed)?;
        let inbound = spawn_bounded_reader(stdout, max_message_bytes);
        Ok(Self {
            child,
            stdin,
            inbound,
            stderr: Some(BlockingStderrDrain::spawn(stderr, STDERR_LIMIT)),
            max_message_bytes,
        })
    }

    fn write_request(
        &mut self,
        request: &BrowserSidecarRequest,
    ) -> Result<(), BrowserSidecarError> {
        let mut encoded =
            serde_json::to_vec(request).map_err(|_| BrowserSidecarError::MalformedMessage)?;
        if encoded.len() > self.max_message_bytes {
            return Err(BrowserSidecarError::MessageTooLarge);
        }
        encoded.push(b'\n');
        self.stdin
            .write_all(&encoded)
            .and_then(|_| self.stdin.flush())
            .map_err(|_| BrowserSidecarError::ProcessExited)
    }

    fn read_response(
        &self,
        timeout: Duration,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        let line = self
            .inbound
            .recv_timeout(timeout)
            .map_err(|error| match error {
                mpsc::RecvTimeoutError::Timeout => BrowserSidecarError::Timeout,
                mpsc::RecvTimeoutError::Disconnected => BrowserSidecarError::ProcessExited,
            })??;
        serde_json::from_slice(&line).map_err(|_| BrowserSidecarError::MalformedMessage)
    }
}

impl BrowserSidecarTransport for PlaywrightSidecarProcess {
    fn request(
        &mut self,
        request: &BrowserSidecarRequest,
        limits: BrowserSidecarLimits,
    ) -> Result<BrowserSidecarResponse, BrowserSidecarError> {
        self.write_request(request)?;
        self.read_response(limits.request_timeout)
    }

    fn shutdown(&mut self, timeout: Duration) -> Result<(), BrowserSidecarError> {
        let request = BrowserSidecarRequest {
            protocol_version: BROWSER_SIDECAR_PROTOCOL_VERSION,
            request_id: "browser-shutdown".to_string(),
            method: "shutdown".to_string(),
            params: serde_json::Value::Null,
        };
        let _ = self.write_request(&request);
        let _ = self.read_response(timeout / 2);
        let deadline = Instant::now() + timeout;
        self.child
            .shutdown(deadline)
            .map_err(|_| BrowserSidecarError::ShutdownFailed)?;
        if let Some(stderr) = self.stderr.take() {
            let _ = stderr.finish(Duration::from_secs(1));
        }
        Ok(())
    }
}

fn spawn_bounded_reader(
    stdout: std::process::ChildStdout,
    max_message_bytes: usize,
) -> Receiver<Result<Vec<u8>, BrowserSidecarError>> {
    let (sender, receiver) = mpsc::sync_channel(8);
    thread::spawn(move || {
        let mut reader = BufReader::new(stdout);
        loop {
            match read_bounded_line(&mut reader, max_message_bytes) {
                Ok(Some(line)) => {
                    if sender.send(Ok(line)).is_err() {
                        break;
                    }
                }
                Ok(None) => break,
                Err(error) => {
                    let _ = sender.send(Err(error));
                    break;
                }
            }
        }
    });
    receiver
}

fn read_bounded_line(
    reader: &mut impl Read,
    max_message_bytes: usize,
) -> Result<Option<Vec<u8>>, BrowserSidecarError> {
    let mut line = Vec::with_capacity(max_message_bytes.min(8 * 1024));
    let mut byte = [0_u8; 1];
    loop {
        match reader.read(&mut byte) {
            Ok(0) if line.is_empty() => return Ok(None),
            Ok(0) => return Err(BrowserSidecarError::MalformedMessage),
            Ok(_) if byte[0] == b'\n' => return Ok(Some(line)),
            Ok(_) if line.len() >= max_message_bytes => {
                return Err(BrowserSidecarError::MessageTooLarge)
            }
            Ok(_) => line.push(byte[0]),
            Err(_) => return Err(BrowserSidecarError::ProcessExited),
        }
    }
}

#[cfg(test)]
#[path = "playwright_sidecar_tests.rs"]
mod tests;
