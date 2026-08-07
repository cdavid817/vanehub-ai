use crate::contexts::workspaces::application::{
    ShellEvent, ShellLaunch, ShellLog, WorkspaceApplicationError as AppError, WorkspaceLogLevel,
    WorkspaceShellEventPort, WorkspaceShellLogPort, WorkspaceShellRuntimePort,
};
use crate::contexts::workspaces::domain::{reset_directory_command, ShellHost, TerminalDimensions};
use crate::platform::filesystem::normalize_windows_extended_length_path;
use crate::platform::text::take_decodable_utf8;
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;

/// The blocking halves of a shell, shared out of the registry so PTY writes and resizes
/// never run while the registry lock is held. A shell whose child stopped draining its
/// pipe would otherwise stall input, resize and shutdown for *every* other shell.
struct ShellIo {
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
}

struct ManagedShell {
    session_id: String,
    root: PathBuf,
    io: Arc<ShellIo>,
    child: Box<dyn Child + Send + Sync>,
}

/// Larger reads coalesce bursty PTY output into fewer IPC events without adding latency:
/// a read still returns as soon as any bytes are available, so interactive echo is
/// unaffected, while a flood of build output emits far fewer events than a 4 KiB buffer.
/// Matches the agent terminal's read width.
const SHELL_READ_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(crate) struct PortablePtyShellRuntime {
    shells: Arc<Mutex<HashMap<String, ManagedShell>>>,
    events: Arc<dyn WorkspaceShellEventPort>,
    logging: Arc<dyn WorkspaceShellLogPort>,
}

impl PortablePtyShellRuntime {
    pub(crate) fn new(
        events: Arc<dyn WorkspaceShellEventPort>,
        logging: Arc<dyn WorkspaceShellLogPort>,
    ) -> Self {
        Self {
            shells: Arc::new(Mutex::new(HashMap::new())),
            events,
            logging,
        }
    }
}

fn terminal_size(dimensions: TerminalDimensions) -> PtySize {
    PtySize {
        rows: dimensions.rows(),
        cols: dimensions.cols(),
        pixel_width: 0,
        pixel_height: 0,
    }
}

fn default_shell() -> String {
    if cfg!(target_os = "windows") {
        std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
    } else {
        std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
    }
}

fn shell_root_path(root: &str) -> PathBuf {
    PathBuf::from(normalize_windows_extended_length_path(root))
}

fn write_shell_log(
    logging: &dyn WorkspaceShellLogPort,
    level: WorkspaceLogLevel,
    session_id: &str,
    shell_id: &str,
    message: &str,
) {
    logging.write(ShellLog {
        level,
        session_id: session_id.to_string(),
        shell_id: shell_id.to_string(),
        message: message.to_string(),
    });
}

fn terminate_child(
    child: &mut dyn Child,
    logging: &dyn WorkspaceShellLogPort,
    session_id: &str,
    shell_id: &str,
    shutdown: bool,
) {
    if child.kill().is_err() {
        let message = if shutdown {
            "Shell process termination failed during shutdown."
        } else {
            "Shell process termination failed."
        };
        write_shell_log(
            logging,
            WorkspaceLogLevel::Warn,
            session_id,
            shell_id,
            message,
        );
    }
    if child.wait().is_err() {
        let message = if shutdown {
            "Shell process wait failed during shutdown."
        } else {
            "Shell process wait failed."
        };
        write_shell_log(
            logging,
            WorkspaceLogLevel::Warn,
            session_id,
            shell_id,
            message,
        );
    }
}

impl PortablePtyShellRuntime {
    fn insert(&self, shell_id: String, shell: ManagedShell) -> Result<(), AppError> {
        self.shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .insert(shell_id, shell);
        Ok(())
    }

    /// Resolves a shell to its owning session and shared I/O handles, releasing the
    /// registry lock before the caller performs any blocking PTY operation.
    fn checkout(&self, shell_id: &str) -> Result<(String, Arc<ShellIo>), AppError> {
        let shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        Ok((shell.session_id.clone(), shell.io.clone()))
    }

    /// Resolves a shell's working root alongside its I/O handles, for the directory reset
    /// command that has to be rendered from the root before anything is written.
    fn checkout_with_root(
        &self,
        shell_id: &str,
    ) -> Result<(String, PathBuf, Arc<ShellIo>), AppError> {
        let shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        Ok((
            shell.session_id.clone(),
            shell.root.clone(),
            shell.io.clone(),
        ))
    }

    fn write_all(io: &ShellIo, bytes: &[u8]) -> Result<(), AppError> {
        let mut writer = io
            .writer
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        writer
            .write_all(bytes)
            .and_then(|_| writer.flush())
            .map_err(|error| AppError::Storage(error.to_string()))
    }
}

impl WorkspaceShellRuntimePort for PortablePtyShellRuntime {
    fn open_shell(&self, launch: &ShellLaunch) -> Result<(), AppError> {
        let root = shell_root_path(&launch.root);
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(terminal_size(launch.dimensions))
            .map_err(|error| {
                write_shell_log(
                    self.logging.as_ref(),
                    WorkspaceLogLevel::Error,
                    &launch.session_id,
                    &launch.shell_id,
                    "PTY creation failed.",
                );
                AppError::LaunchFailed(error.to_string())
            })?;
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(&root);
        let child = pair.slave.spawn_command(command).map_err(|error| {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Error,
                &launch.session_id,
                &launch.shell_id,
                "Shell process launch failed.",
            );
            AppError::LaunchFailed(error.to_string())
        })?;
        drop(pair.slave);
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|error| AppError::Storage(error.to_string()))?;

        let events = self.events.clone();
        let reader_shell_id = launch.shell_id.clone();
        let reader_session_id = launch.session_id.clone();
        thread::spawn(move || {
            let mut buffer = [0u8; SHELL_READ_BUFFER_BYTES];
            // Reads land on arbitrary byte boundaries, so a multi-byte UTF-8 sequence can be
            // split across two reads; carry the incomplete tail until the next read completes it.
            let mut pending: Vec<u8> = Vec::new();
            loop {
                match reader.read(&mut buffer) {
                    Ok(0) => break,
                    Ok(count) => {
                        pending.extend_from_slice(&buffer[..count]);
                        let content = take_decodable_utf8(&mut pending);
                        if content.is_empty() {
                            continue;
                        }
                        events.publish(ShellEvent::Output {
                            shell_id: reader_shell_id.clone(),
                            session_id: reader_session_id.clone(),
                            content,
                        })
                    }
                    Err(_) => break,
                }
            }
            events.publish(ShellEvent::State {
                shell_id: reader_shell_id,
                session_id: reader_session_id,
                state: "disconnected",
                error: None,
            });
        });

        self.insert(
            launch.shell_id.clone(),
            ManagedShell {
                session_id: launch.session_id.clone(),
                root,
                io: Arc::new(ShellIo {
                    master: Mutex::new(pair.master),
                    writer: Mutex::new(writer),
                }),
                child,
            },
        )
    }

    fn write_input(&self, shell_id: &str, content: &str) -> Result<(), AppError> {
        let (session_id, io) = self.checkout(shell_id)?;
        let result = Self::write_all(&io, content.as_bytes());
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell input failed.",
            );
        }
        result
    }

    fn reset_directory(&self, shell_id: &str) -> Result<(), AppError> {
        let (session_id, root, io) = self.checkout_with_root(shell_id)?;
        let host = if cfg!(target_os = "windows") {
            ShellHost::Windows
        } else {
            ShellHost::Unix
        };
        let command = reset_directory_command(&root.to_string_lossy(), host);
        let result = Self::write_all(&io, command.as_bytes());
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell directory reset failed.",
            );
        }
        result
    }

    fn resize(&self, shell_id: &str, dimensions: TerminalDimensions) -> Result<(), AppError> {
        let (session_id, io) = self.checkout(shell_id)?;
        let result = io
            .master
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))
            .and_then(|master| {
                master
                    .resize(terminal_size(dimensions))
                    .map_err(|error| AppError::Storage(error.to_string()))
            });
        if result.is_err() {
            write_shell_log(
                self.logging.as_ref(),
                WorkspaceLogLevel::Warn,
                &session_id,
                shell_id,
                "Shell resize failed.",
            );
        }
        result
    }

    fn stop(&self, shell_id: &str) -> Result<Option<String>, AppError> {
        let shell = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .remove(shell_id);
        let Some(mut shell) = shell else {
            return Ok(None);
        };
        terminate_child(
            &mut *shell.child,
            self.logging.as_ref(),
            &shell.session_id,
            shell_id,
            false,
        );
        Ok(Some(shell.session_id))
    }

    fn stop_for_session(&self, session_id: &str) -> Result<Vec<(String, String)>, AppError> {
        let shell_ids = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .iter()
            .filter(|(_, shell)| shell.session_id == session_id)
            .map(|(shell_id, _)| shell_id.clone())
            .collect::<Vec<_>>();
        let mut stopped = Vec::with_capacity(shell_ids.len());
        for shell_id in shell_ids {
            if let Some(owning_session_id) = self.stop(&shell_id)? {
                stopped.push((shell_id, owning_session_id));
            }
        }
        Ok(stopped)
    }
}

impl Drop for PortablePtyShellRuntime {
    fn drop(&mut self) {
        if Arc::strong_count(&self.shells) != 1 {
            return;
        }
        if let Ok(mut shells) = self.shells.lock() {
            for (_, mut shell) in shells.drain() {
                terminate_child(
                    &mut *shell.child,
                    self.logging.as_ref(),
                    &shell.session_id,
                    "shutdown",
                    true,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{ChildKiller, ExitStatus};
    use std::io;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    fn temp_dir(label: &str) -> PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        std::env::temp_dir().join(format!("vanehub-shell-{label}-{suffix}"))
    }

    fn remove_test_dir(path: &Path) {
        let mut last_error = None;
        for _ in 0..20 {
            match std::fs::remove_dir_all(path) {
                Ok(()) => return,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return,
                Err(error) => {
                    last_error = Some(error);
                    std::thread::sleep(Duration::from_millis(25));
                }
            }
        }
        panic!("cleanup: {}", last_error.expect("cleanup error"));
    }

    #[derive(Debug)]
    struct FailingChild;

    #[derive(Default)]
    struct CapturingEvents {
        events: Mutex<Vec<ShellEvent>>,
    }

    impl WorkspaceShellEventPort for CapturingEvents {
        fn publish(&self, event: ShellEvent) {
            self.events.lock().expect("events").push(event);
        }
    }

    #[derive(Default)]
    struct CapturingLogs {
        logs: Mutex<Vec<ShellLog>>,
    }

    impl WorkspaceShellLogPort for CapturingLogs {
        fn write(&self, log: ShellLog) {
            self.logs.lock().expect("logs").push(log);
        }
    }

    fn runtime() -> (PortablePtyShellRuntime, Arc<CapturingLogs>) {
        let logging = Arc::new(CapturingLogs::default());
        (
            PortablePtyShellRuntime::new(Arc::new(CapturingEvents::default()), logging.clone()),
            logging,
        )
    }

    impl ChildKiller for FailingChild {
        fn kill(&mut self) -> io::Result<()> {
            Err(io::Error::other("secret kill detail"))
        }

        fn clone_killer(&self) -> Box<dyn ChildKiller + Send + Sync> {
            Box::new(Self)
        }
    }

    impl Child for FailingChild {
        fn try_wait(&mut self) -> io::Result<Option<ExitStatus>> {
            Ok(None)
        }

        fn wait(&mut self) -> io::Result<ExitStatus> {
            Err(io::Error::other("secret wait detail"))
        }

        fn process_id(&self) -> Option<u32> {
            None
        }

        #[cfg(windows)]
        fn as_raw_handle(&self) -> Option<std::os::windows::io::RawHandle> {
            None
        }
    }

    fn managed_test_shell(session_id: &str, root: &Path) -> ManagedShell {
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(root);
        let child = pair.slave.spawn_command(command).expect("test shell");
        drop(pair.slave);
        let writer = pair.master.take_writer().expect("test writer");
        ManagedShell {
            session_id: session_id.to_string(),
            root: root.to_path_buf(),
            io: Arc::new(ShellIo {
                master: Mutex::new(pair.master),
                writer: Mutex::new(writer),
            }),
            child,
        }
    }

    #[test]
    fn terminal_dimensions_are_bounded() {
        assert_eq!(terminal_size(TerminalDimensions::bounded(0, 0)).rows, 1);
        assert_eq!(
            terminal_size(TerminalDimensions::bounded(800, 900)).cols,
            500
        );
    }

    #[test]
    fn missing_shell_kill_is_idempotent_at_manager_level() {
        let (manager, _) = runtime();
        assert_eq!(manager.stop("missing").expect("first stop"), None);
        assert_eq!(manager.stop("missing").expect("second stop"), None);
    }

    #[test]
    fn child_shutdown_failures_write_generic_warnings() {
        let logging = CapturingLogs::default();
        terminate_child(
            &mut FailingChild,
            &logging,
            "session-one",
            "shell-one",
            false,
        );
        let messages = logging
            .logs
            .lock()
            .expect("logs")
            .iter()
            .map(|log| log.message.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            messages,
            vec![
                "Shell process termination failed.",
                "Shell process wait failed."
            ]
        );
        assert!(!messages.join(" ").contains("secret"));
    }

    #[test]
    fn missing_shell_routes_return_validation_errors() {
        let (manager, _) = runtime();
        assert!(manager.write_input("missing", "echo test").is_err());
        assert!(manager.reset_directory("missing").is_err());
        assert!(manager
            .resize("missing", TerminalDimensions::bounded(24, 80))
            .is_err());
    }

    #[test]
    fn default_shell_and_cd_escaping_are_platform_specific() {
        assert!(!default_shell().trim().is_empty());
    }

    #[test]
    fn shell_roots_strip_windows_extended_length_prefixes_before_launch() {
        assert_eq!(
            shell_root_path(r"\\?\D:\cdavid\Documents\code\claude-code").to_string_lossy(),
            r"D:\cdavid\Documents\code\claude-code"
        );
        assert_eq!(
            shell_root_path(r"\\?\UNC\server\share\repo").to_string_lossy(),
            r"\\server\share\repo"
        );
    }

    #[test]
    fn manager_routes_input_resize_and_cleanup_by_shell_id() {
        let root = temp_dir("manager");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        manager
            .insert(
                "shell-one".to_string(),
                managed_test_shell("session-one", &root),
            )
            .expect("insert first");
        manager
            .insert(
                "shell-two".to_string(),
                managed_test_shell("session-two", &root),
            )
            .expect("insert second");
        assert_eq!(manager.shells.lock().expect("shell map").len(), 2);
        manager
            .write_input(
                "shell-one",
                if cfg!(windows) {
                    "echo test\r\n"
                } else {
                    "echo test\n"
                },
            )
            .expect("input");
        manager
            .resize("shell-two", TerminalDimensions::bounded(30, 100))
            .expect("resize");
        assert_eq!(
            manager.stop("shell-one").expect("stop first").as_deref(),
            Some("session-one")
        );
        assert_eq!(manager.stop("shell-one").expect("repeat stop"), None);
        assert_eq!(
            manager.stop("shell-two").expect("stop second").as_deref(),
            Some("session-two")
        );
        assert!(manager.shells.lock().expect("shell map").is_empty());
        remove_test_dir(&root);
    }

    #[test]
    fn a_blocked_shell_writer_does_not_stall_other_shells() {
        let root = temp_dir("writer-isolation");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        manager
            .insert(
                "shell-one".to_string(),
                managed_test_shell("session-one", &root),
            )
            .expect("insert first");
        manager
            .insert(
                "shell-two".to_string(),
                managed_test_shell("session-two", &root),
            )
            .expect("insert second");

        // Stands in for a child that stopped draining its pipe: shell-one's writer is held
        // for the whole test. The registry lock must not be part of that critical section.
        let (_, blocked) = manager.checkout("shell-one").expect("checkout first");
        let _held = blocked.writer.lock().expect("hold first writer");

        manager
            .resize("shell-two", TerminalDimensions::bounded(30, 100))
            .expect("second shell resizes while the first writer is blocked");
        manager
            .write_input("shell-two", if cfg!(windows) { "\r\n" } else { "\n" })
            .expect("second shell accepts input while the first writer is blocked");
        assert_eq!(
            manager.stop("shell-two").expect("stop second").as_deref(),
            Some("session-two")
        );

        drop(_held);
        manager.stop("shell-one").expect("stop first");
        remove_test_dir(&root);
    }

    #[test]
    fn manager_cleans_up_only_the_requested_session_shells() {
        let root = temp_dir("session-cleanup");
        std::fs::create_dir_all(&root).expect("root");
        let (manager, _) = runtime();
        manager
            .insert(
                "shell-one".to_string(),
                managed_test_shell("session-one", &root),
            )
            .expect("insert first");
        manager
            .insert(
                "shell-two".to_string(),
                managed_test_shell("session-two", &root),
            )
            .expect("insert second");

        let stopped = manager
            .stop_for_session("session-one")
            .expect("stop session");

        assert_eq!(
            stopped,
            vec![("shell-one".to_string(), "session-one".to_string())]
        );
        assert!(manager
            .shells
            .lock()
            .expect("shell map")
            .contains_key("shell-two"));
        assert_eq!(
            manager
                .stop_for_session("session-one")
                .expect("repeat cleanup"),
            Vec::new()
        );
        manager.stop("shell-two").expect("stop remaining");
        remove_test_dir(&root);
    }

    #[test]
    fn invalid_shell_executable_fails_to_spawn() {
        let pair = native_pty_system()
            .openpty(terminal_size(TerminalDimensions::bounded(24, 80)))
            .expect("test pty");
        let command = CommandBuilder::new("vanehub-shell-executable-that-does-not-exist");
        assert!(pair.slave.spawn_command(command).is_err());
    }
}
