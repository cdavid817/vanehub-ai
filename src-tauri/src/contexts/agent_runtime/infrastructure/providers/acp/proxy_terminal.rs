//! The client terminal methods (`terminal/create`, `output`, `wait_for_exit`, `kill`, `release`)
//! the host proxies for an ACP agent.
//!
//! Every terminal is owned by one connection epoch and one session. A request that names a
//! terminal from another session or an older epoch is refused without revealing output. Output
//! is bounded; the command runs under the platform's contained child so `kill` and `release` reap
//! whatever it spawned and nothing else. Whether a command may run at all is decided before this
//! module is reached, by the permission policy.

use super::budget::{MAX_PROXY_TERMINALS, TERMINAL_OUTPUT_LIMIT_BYTES};
use crate::platform::process::ManagedChild;
use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TerminalProxyError {
    /// No terminal with that id is owned by this session and epoch.
    NotOwned,
    TooMany {
        limit: usize,
    },
    Spawn(String),
    Io(String),
}

impl TerminalProxyError {
    pub(crate) fn message(&self) -> String {
        match self {
            Self::NotOwned => "terminal is not owned by this session".to_string(),
            Self::TooMany { limit } => format!("at most {limit} proxied terminals may be open"),
            Self::Spawn(detail) => format!("terminal command could not start: {detail}"),
            Self::Io(detail) => format!("terminal operation failed: {detail}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalCreateRequest {
    pub(crate) command: String,
    pub(crate) args: Vec<String>,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) cwd: String,
    pub(crate) output_byte_limit: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalExitStatus {
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalOutput {
    pub(crate) output: String,
    pub(crate) truncated: bool,
    pub(crate) exit_status: Option<TerminalExitStatus>,
}

#[derive(Debug, Default)]
struct OutputBuffer {
    retained: Vec<u8>,
    truncated: bool,
    limit: usize,
}

impl OutputBuffer {
    fn push(&mut self, chunk: &[u8]) {
        self.retained.extend_from_slice(chunk);
        if self.retained.len() > self.limit {
            let excess = self.retained.len() - self.limit;
            self.retained.drain(..excess);
            self.truncated = true;
        }
    }
}

struct ProxiedTerminal {
    epoch: u64,
    session_id: String,
    child: Arc<Mutex<ManagedChild>>,
    output: Arc<Mutex<OutputBuffer>>,
    exit: Arc<Mutex<Option<TerminalExitStatus>>>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct TerminalOwner<'a> {
    pub(crate) epoch: u64,
    pub(crate) session_id: &'a str,
}

#[derive(Default)]
pub(crate) struct TerminalRegistry {
    terminals: Mutex<HashMap<String, ProxiedTerminal>>,
    sequence: AtomicU64,
}

impl TerminalRegistry {
    pub(crate) fn create(
        &self,
        owner: TerminalOwner<'_>,
        request: &TerminalCreateRequest,
    ) -> Result<String, TerminalProxyError> {
        {
            let terminals = lock(&self.terminals);
            let owned = terminals
                .values()
                .filter(|terminal| terminal.epoch == owner.epoch)
                .count();
            if owned >= MAX_PROXY_TERMINALS {
                return Err(TerminalProxyError::TooMany {
                    limit: MAX_PROXY_TERMINALS,
                });
            }
        }
        let mut child = ManagedChild::spawn_in(
            &request.command,
            &request.args,
            &request.environment,
            Some(Path::new(&request.cwd)),
        )
        .map_err(|error| TerminalProxyError::Spawn(error.to_string()))?;
        // The agent gets no stdin: a proxied command is not an interactive terminal.
        drop(child.take_stdin());
        let stdout = child
            .take_stdout()
            .map_err(|error| TerminalProxyError::Spawn(error.to_string()))?;
        let stderr = child
            .take_stderr()
            .map_err(|error| TerminalProxyError::Spawn(error.to_string()))?;
        let limit = request
            .output_byte_limit
            .unwrap_or(TERMINAL_OUTPUT_LIMIT_BYTES)
            .clamp(1, TERMINAL_OUTPUT_LIMIT_BYTES);
        let output = Arc::new(Mutex::new(OutputBuffer {
            limit,
            ..OutputBuffer::default()
        }));
        for stream in [Box::new(stdout) as Box<dyn Read + Send>, Box::new(stderr)] {
            let output = output.clone();
            thread::spawn(move || drain(stream, output));
        }
        let child = Arc::new(Mutex::new(child));
        let exit = Arc::new(Mutex::new(None));
        {
            let child = child.clone();
            let exit = exit.clone();
            // Poll without holding the child lock across the wait: a blocking `wait_until`
            // inside the lock kept the mutex almost permanently taken, and `kill`/`release`
            // starved on it for tens of seconds while the command ran on.
            thread::spawn(move || loop {
                let status = lock(&child).wait_until(Instant::now());
                match status {
                    Ok(Some(status)) => {
                        *lock(&exit) = Some(exit_status_of(Some(status)));
                        break;
                    }
                    Ok(None) => thread::sleep(Duration::from_millis(50)),
                    Err(_) => {
                        *lock(&exit) = Some(TerminalExitStatus {
                            exit_code: None,
                            signal: Some("unknown".to_string()),
                        });
                        break;
                    }
                }
            });
        }
        let id = format!(
            "term-{}-{}",
            owner.epoch,
            self.sequence.fetch_add(1, Ordering::Relaxed) + 1
        );
        lock(&self.terminals).insert(
            id.clone(),
            ProxiedTerminal {
                epoch: owner.epoch,
                session_id: owner.session_id.to_string(),
                child,
                output,
                exit,
            },
        );
        Ok(id)
    }

    pub(crate) fn output(
        &self,
        owner: TerminalOwner<'_>,
        terminal_id: &str,
    ) -> Result<TerminalOutput, TerminalProxyError> {
        let (output, exit) = {
            let terminals = lock(&self.terminals);
            let terminal = owned(&terminals, owner, terminal_id)?;
            (terminal.output.clone(), terminal.exit.clone())
        };
        let exit_status = lock(&exit).clone();
        let output = lock(&output);
        let result = TerminalOutput {
            output: String::from_utf8_lossy(&output.retained).to_string(),
            truncated: output.truncated,
            exit_status,
        };
        Ok(result)
    }

    /// Blocks until the command exits, `timeout` passes, or `abort` reports true. Returns `None`
    /// in the latter two cases so a driver can keep servicing other messages and ask again.
    pub(crate) fn wait_for_exit(
        &self,
        owner: TerminalOwner<'_>,
        terminal_id: &str,
        timeout: Duration,
        abort: &dyn Fn() -> bool,
    ) -> Result<Option<TerminalExitStatus>, TerminalProxyError> {
        let exit = {
            let terminals = lock(&self.terminals);
            owned(&terminals, owner, terminal_id)?.exit.clone()
        };
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(status) = lock(&exit).clone() {
                return Ok(Some(status));
            }
            if Instant::now() >= deadline || abort() {
                return Ok(None);
            }
            thread::sleep(Duration::from_millis(20));
        }
    }

    pub(crate) fn kill(
        &self,
        owner: TerminalOwner<'_>,
        terminal_id: &str,
    ) -> Result<(), TerminalProxyError> {
        let child = {
            let terminals = lock(&self.terminals);
            owned(&terminals, owner, terminal_id)?.child.clone()
        };
        let outcome = lock(&child)
            .shutdown(Instant::now() + Duration::from_secs(5))
            .map(|_| ())
            .map_err(|error| TerminalProxyError::Io(error.to_string()));
        outcome
    }

    pub(crate) fn release(
        &self,
        owner: TerminalOwner<'_>,
        terminal_id: &str,
    ) -> Result<(), TerminalProxyError> {
        let terminal = {
            let mut terminals = lock(&self.terminals);
            owned(&terminals, owner, terminal_id)?;
            terminals.remove(terminal_id)
        };
        if let Some(terminal) = terminal {
            let _ = lock(&terminal.child).shutdown(Instant::now() + Duration::from_secs(5));
        }
        Ok(())
    }

    /// Kills and forgets every terminal of one epoch. Called when the owning connection goes
    /// away, so a torn-down agent leaves no proxied command behind.
    pub(crate) fn release_epoch(&self, epoch: u64) -> usize {
        let removed: Vec<ProxiedTerminal> = {
            let mut terminals = lock(&self.terminals);
            let ids: Vec<String> = terminals
                .iter()
                .filter(|(_, terminal)| terminal.epoch == epoch)
                .map(|(id, _)| id.clone())
                .collect();
            ids.iter().filter_map(|id| terminals.remove(id)).collect()
        };
        let count = removed.len();
        for terminal in removed {
            let _ = lock(&terminal.child).shutdown(Instant::now() + Duration::from_secs(5));
        }
        count
    }

    #[cfg(test)]
    pub(crate) fn open_count(&self) -> usize {
        lock(&self.terminals).len()
    }
}

fn owned<'a>(
    terminals: &'a HashMap<String, ProxiedTerminal>,
    owner: TerminalOwner<'_>,
    terminal_id: &str,
) -> Result<&'a ProxiedTerminal, TerminalProxyError> {
    terminals
        .get(terminal_id)
        .filter(|terminal| terminal.epoch == owner.epoch && terminal.session_id == owner.session_id)
        .ok_or(TerminalProxyError::NotOwned)
}

fn exit_status_of(status: Option<std::process::ExitStatus>) -> TerminalExitStatus {
    let Some(status) = status else {
        return TerminalExitStatus {
            exit_code: None,
            signal: None,
        };
    };
    #[cfg(unix)]
    let signal = {
        use std::os::unix::process::ExitStatusExt;
        status.signal().map(|signal| signal.to_string())
    };
    #[cfg(not(unix))]
    let signal: Option<String> = None;
    TerminalExitStatus {
        exit_code: status.code(),
        signal,
    }
}

fn drain(mut stream: Box<dyn Read + Send>, output: Arc<Mutex<OutputBuffer>>) {
    let mut chunk = [0_u8; 8 * 1024];
    loop {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(read) => lock(&output).push(&chunk[..read]),
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;

    fn request(command: &str, args: &[&str], cwd: &Path) -> TerminalCreateRequest {
        TerminalCreateRequest {
            command: command.to_string(),
            args: args.iter().map(|arg| (*arg).to_string()).collect(),
            environment: BTreeMap::new(),
            cwd: cwd.to_string_lossy().to_string(),
            output_byte_limit: Some(64),
        }
    }

    #[test]
    fn terminals_are_scoped_to_their_session_and_epoch() {
        let workspace = TempDirectory::new("acp-terminal");
        let registry = TerminalRegistry::default();
        let owner = TerminalOwner {
            epoch: 3,
            session_id: "session-a",
        };
        let id = registry
            .create(
                owner,
                &request("sh", &["-c", "printf 'hello 世界'"], workspace.path()),
            )
            .expect("create");
        let status = registry
            .wait_for_exit(owner, &id, Duration::from_secs(5), &|| false)
            .expect("wait")
            .expect("exited");
        assert_eq!(status.exit_code, Some(0));
        // Output threads may finish a hair after exit; poll briefly.
        let mut output = registry.output(owner, &id).expect("output");
        for _ in 0..50 {
            if output.output.contains("世界") {
                break;
            }
            thread::sleep(Duration::from_millis(10));
            output = registry.output(owner, &id).expect("output");
        }
        assert_eq!(output.output, "hello 世界");
        assert!(!output.truncated);
        assert_eq!(output.exit_status.map(|s| s.exit_code), Some(Some(0)));

        let other_session = TerminalOwner {
            epoch: 3,
            session_id: "session-b",
        };
        let older_epoch = TerminalOwner {
            epoch: 2,
            session_id: "session-a",
        };
        for wrong in [other_session, older_epoch] {
            assert_eq!(
                registry.output(wrong, &id).expect_err("not owned"),
                TerminalProxyError::NotOwned
            );
            assert_eq!(
                registry.kill(wrong, &id).expect_err("not owned"),
                TerminalProxyError::NotOwned
            );
            assert_eq!(
                registry.release(wrong, &id).expect_err("not owned"),
                TerminalProxyError::NotOwned
            );
        }
        registry.release(owner, &id).expect("release");
        assert_eq!(
            registry.output(owner, &id).expect_err("gone"),
            TerminalProxyError::NotOwned
        );
        assert_eq!(registry.open_count(), 0);
    }

    #[test]
    fn output_is_bounded_and_kill_reaps_a_long_running_command() {
        let workspace = TempDirectory::new("acp-terminal-kill");
        let registry = TerminalRegistry::default();
        let owner = TerminalOwner {
            epoch: 9,
            session_id: "s",
        };
        let noisy = registry
            .create(
                owner,
                &request(
                    "sh",
                    &["-c", "head -c 4096 /dev/zero | tr '\\0' 'a'"],
                    workspace.path(),
                ),
            )
            .expect("create");
        registry
            .wait_for_exit(owner, &noisy, Duration::from_secs(5), &|| false)
            .expect("wait")
            .expect("exited");
        thread::sleep(Duration::from_millis(50));
        let output = registry.output(owner, &noisy).expect("output");
        assert!(output.truncated);
        assert_eq!(output.output.len(), 64);

        let started = Instant::now();
        let long = registry
            .create(owner, &request("sh", &["-c", "sleep 30"], workspace.path()))
            .expect("create");
        assert!(registry
            .wait_for_exit(owner, &long, Duration::from_millis(50), &|| false)
            .expect("wait")
            .is_none());
        // An abort request ends the wait at once even though the command runs on.
        let aborted = Instant::now();
        assert!(registry
            .wait_for_exit(owner, &long, Duration::from_secs(30), &|| true)
            .expect("wait")
            .is_none());
        assert!(aborted.elapsed() < Duration::from_secs(1));
        registry.kill(owner, &long).expect("kill");
        let status = registry
            .wait_for_exit(owner, &long, Duration::from_secs(5), &|| false)
            .expect("wait")
            .expect("reaped");
        // What the kill must prove is that a 30-second command did not run its course. The
        // recorded code after a group signal varies with the shell and with host load (a signal
        // death, 143, and once a plain 0 were all observed), so the clock is the assertion.
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "the command ran on after kill: {status:?}"
        );
        assert_ne!(status.exit_code, Some(0), "{status:?}");
        // Killed but not released: the id stays valid for output/wait, as the protocol requires.
        assert!(registry.output(owner, &long).is_ok());
        assert_eq!(registry.release_epoch(9), 2);
        assert_eq!(registry.open_count(), 0);
    }

    #[test]
    fn the_open_terminal_budget_is_enforced_per_epoch() {
        let workspace = TempDirectory::new("acp-terminal-budget");
        let registry = TerminalRegistry::default();
        let owner = TerminalOwner {
            epoch: 1,
            session_id: "s",
        };
        for _ in 0..MAX_PROXY_TERMINALS {
            registry
                .create(owner, &request("sh", &["-c", "sleep 5"], workspace.path()))
                .expect("within budget");
        }
        assert!(matches!(
            registry.create(owner, &request("sh", &["-c", "true"], workspace.path())),
            Err(TerminalProxyError::TooMany { .. })
        ));
        registry.release_epoch(1);
        assert_eq!(registry.open_count(), 0);
        assert!(matches!(
            registry.create(
                owner,
                &request("/definitely/missing/binary", &[], workspace.path())
            ),
            Err(TerminalProxyError::Spawn(_))
        ));
    }
}
