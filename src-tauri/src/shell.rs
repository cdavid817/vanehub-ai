use super::{active_log_dir_from_conn, load_session, logging, session_tabs, AppError, RegistryStore};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Write};
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use tauri::{AppHandle, Emitter, State};
use uuid::Uuid;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateShellInput {
    session_id: String,
    rows: u16,
    cols: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResizeShellInput {
    shell_id: String,
    rows: u16,
    cols: u16,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ShellSession {
    shell_id: String,
    session_id: String,
    state: &'static str,
    capability: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
enum ShellEvent {
    #[serde(rename_all = "camelCase")]
    Output {
        shell_id: String,
        session_id: String,
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    State {
        shell_id: String,
        session_id: String,
        state: &'static str,
        error: Option<String>,
    },
}

struct ManagedShell {
    session_id: String,
    root: std::path::PathBuf,
    log_dir: std::path::PathBuf,
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
}

#[derive(Default)]
pub(crate) struct ShellManager {
    shells: Mutex<HashMap<String, ManagedShell>>,
}

fn terminal_size(rows: u16, cols: u16) -> PtySize {
    PtySize {
        rows: rows.clamp(1, 500),
        cols: cols.clamp(1, 500),
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

fn write_shell_log(
    log_dir: &Path,
    level: logging::LogLevel,
    session_id: &str,
    shell_id: &str,
    message: &str,
) {
    let mut context = BTreeMap::new();
    context.insert("sessionId".to_string(), session_id.to_string());
    context.insert("shellId".to_string(), shell_id.to_string());
    let _ = logging::write_message(log_dir, level, "session.shell", message, context);
}

fn terminate_child(
    child: &mut dyn Child,
    log_dir: &Path,
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
        write_shell_log(log_dir, logging::LogLevel::Warn, session_id, shell_id, message);
    }
    if child.wait().is_err() {
        let message = if shutdown {
            "Shell process wait failed during shutdown."
        } else {
            "Shell process wait failed."
        };
        write_shell_log(log_dir, logging::LogLevel::Warn, session_id, shell_id, message);
    }
}

impl ShellManager {
    fn insert(&self, shell_id: String, shell: ManagedShell) -> Result<(), AppError> {
        self.shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?
            .insert(shell_id, shell);
        Ok(())
    }

    fn write_input(&self, shell_id: &str, content: &str) -> Result<(), AppError> {
        let mut shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get_mut(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        let result = shell
            .writer
            .write_all(content.as_bytes())
            .and_then(|_| shell.writer.flush())
            .map_err(|error| AppError::Storage(error.to_string()));
        if result.is_err() {
            write_shell_log(&shell.log_dir, logging::LogLevel::Warn, &shell.session_id, shell_id, "Shell input failed.");
        }
        result
    }

    fn resize(&self, input: &ResizeShellInput) -> Result<(), AppError> {
        let shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get(&input.shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        let result = shell
            .master
            .resize(terminal_size(input.rows, input.cols))
            .map_err(|error| AppError::Storage(error.to_string()));
        if result.is_err() {
            write_shell_log(&shell.log_dir, logging::LogLevel::Warn, &shell.session_id, &input.shell_id, "Shell resize failed.");
        }
        result
    }

    fn reset_directory(&self, shell_id: &str) -> Result<(), AppError> {
        let mut shells = self
            .shells
            .lock()
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let shell = shells
            .get_mut(shell_id)
            .ok_or_else(|| AppError::Validation("Shell session is not connected.".to_string()))?;
        let command = if cfg!(target_os = "windows") {
            format!("cd /d \"{}\"\r\n", shell.root.display())
        } else {
            let escaped = shell.root.to_string_lossy().replace('\'', "'\"'\"'");
            format!("cd '{escaped}'\n")
        };
        let result = shell
            .writer
            .write_all(command.as_bytes())
            .and_then(|_| shell.writer.flush())
            .map_err(|error| AppError::Storage(error.to_string()));
        if result.is_err() {
            write_shell_log(&shell.log_dir, logging::LogLevel::Warn, &shell.session_id, shell_id, "Shell directory reset failed.");
        }
        result
    }

    fn kill(&self, app: &AppHandle, shell_id: &str) -> Result<(), AppError> {
        let Some(session_id) = self.stop(shell_id)? else {
            return Ok(());
        };
        let _ = app.emit(
            "shell:event",
            ShellEvent::State {
                shell_id: shell_id.to_string(),
                session_id,
                state: "disconnected",
                error: None,
            },
        );
        Ok(())
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
        terminate_child(&mut *shell.child, &shell.log_dir, &shell.session_id, shell_id, false);
        write_shell_log(&shell.log_dir, logging::LogLevel::Info, &shell.session_id, shell_id, "Shell disconnected.");
        Ok(Some(shell.session_id))
    }

    pub(crate) fn kill_for_session(
        &self,
        app: &AppHandle,
        session_id: &str,
    ) -> Result<(), AppError> {
        for (shell_id, owning_session_id) in self.stop_for_session(session_id)? {
            let _ = app.emit(
                "shell:event",
                ShellEvent::State {
                    shell_id,
                    session_id: owning_session_id,
                    state: "disconnected",
                    error: None,
                },
            );
        }
        Ok(())
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

impl Drop for ShellManager {
    fn drop(&mut self) {
        if let Ok(shells) = self.shells.get_mut() {
            for (_, mut shell) in shells.drain() {
                terminate_child(&mut *shell.child, &shell.log_dir, &shell.session_id, "shutdown", true);
            }
        }
    }
}

pub(crate) fn shell_create(
    app: AppHandle,
    state: State<'_, Mutex<RegistryStore>>,
    manager: State<'_, ShellManager>,
    input: CreateShellInput,
) -> Result<ShellSession, AppError> {
    let store = state
        .lock()
        .map_err(|error| AppError::Storage(error.to_string()))?;
    let conn = store.connection()?;
    let session = load_session(&conn, &input.session_id)?;
    let root = session_tabs::resolve_session_root(&conn, &input.session_id)?.ok_or_else(|| {
        AppError::Validation("Session workspace is unavailable.".to_string())
    })?;
    let log_dir = active_log_dir_from_conn(&conn)?;
    drop(store);

    let shell_id = Uuid::new_v4().to_string();
    let pty_system = native_pty_system();
    let pair = pty_system
        .openpty(terminal_size(input.rows, input.cols))
        .map_err(|error| {
            write_shell_log(
                &log_dir,
                logging::LogLevel::Error,
                &input.session_id,
                &shell_id,
                "PTY creation failed.",
            );
            AppError::LaunchFailed(error.to_string())
        })?;
    let mut command = CommandBuilder::new(default_shell());
    command.cwd(&root);
    let child = pair.slave.spawn_command(command).map_err(|error| {
        write_shell_log(
            &log_dir,
            logging::LogLevel::Error,
            &input.session_id,
            &shell_id,
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

    let reader_app = app.clone();
    let reader_shell_id = shell_id.clone();
    let reader_session_id = input.session_id.clone();
    thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let content = String::from_utf8_lossy(&buffer[..count]).to_string();
                    let _ = reader_app.emit(
                        "shell:event",
                        ShellEvent::Output {
                            shell_id: reader_shell_id.clone(),
                            session_id: reader_session_id.clone(),
                            content,
                        },
                    );
                }
                Err(_) => break,
            }
        }
        let _ = reader_app.emit(
            "shell:event",
            ShellEvent::State {
                shell_id: reader_shell_id,
                session_id: reader_session_id,
                state: "disconnected",
                error: None,
            },
        );
    });

    manager.insert(
        shell_id.clone(),
        ManagedShell {
            session_id: input.session_id.clone(),
            root,
            log_dir: log_dir.clone(),
            master: pair.master,
            writer,
            child,
        },
    )?;
    write_shell_log(
        &log_dir,
        logging::LogLevel::Info,
        &input.session_id,
        &shell_id,
        &format!("Shell connected for agent {}.", session.agent_id),
    );
    Ok(ShellSession {
        shell_id,
        session_id: input.session_id,
        state: "connected",
        capability: "native",
    })
}

pub(crate) fn shell_input(
    manager: State<'_, ShellManager>,
    shell_id: String,
    content: String,
) -> Result<(), AppError> {
    manager.write_input(&shell_id, &content)
}

pub(crate) fn shell_cd(
    manager: State<'_, ShellManager>,
    shell_id: String,
) -> Result<(), AppError> {
    manager.reset_directory(&shell_id)
}

pub(crate) fn shell_resize(
    manager: State<'_, ShellManager>,
    input: ResizeShellInput,
) -> Result<(), AppError> {
    manager.resize(&input)
}

pub(crate) fn shell_kill(
    app: AppHandle,
    manager: State<'_, ShellManager>,
    shell_id: String,
) -> Result<(), AppError> {
    manager.kill(&app, &shell_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use portable_pty::{ChildKiller, ExitStatus};
    use std::io;
    use std::path::PathBuf;
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

    impl ChildKiller for FailingChild {
        fn kill(&mut self) -> io::Result<()> {
            Err(io::Error::new(io::ErrorKind::Other, "secret kill detail"))
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
            Err(io::Error::new(io::ErrorKind::Other, "secret wait detail"))
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
            .openpty(terminal_size(24, 80))
            .expect("test pty");
        let mut command = CommandBuilder::new(default_shell());
        command.cwd(root);
        let child = pair.slave.spawn_command(command).expect("test shell");
        drop(pair.slave);
        let writer = pair.master.take_writer().expect("test writer");
        ManagedShell {
            session_id: session_id.to_string(),
            root: root.to_path_buf(),
            log_dir: root.to_path_buf(),
            master: pair.master,
            writer,
            child,
        }
    }

    #[test]
    fn terminal_dimensions_are_bounded() {
        assert_eq!(terminal_size(0, 0).rows, 1);
        assert_eq!(terminal_size(800, 900).cols, 500);
    }

    #[test]
    fn missing_shell_kill_is_idempotent_at_manager_level() {
        let manager = ShellManager::default();
        assert_eq!(manager.stop("missing").expect("first stop"), None);
        assert_eq!(manager.stop("missing").expect("second stop"), None);
    }

    #[test]
    fn child_shutdown_failures_write_generic_warnings() {
        let root = temp_dir("shutdown-warning");
        std::fs::create_dir_all(&root).expect("root");
        terminate_child(&mut FailingChild, &root, "session-one", "shell-one", false);
        let content = std::fs::read_to_string(root.join(logging::LOG_FILE_NAME)).expect("log");
        assert!(content.contains("Shell process termination failed."));
        assert!(content.contains("Shell process wait failed."));
        assert!(!content.contains("secret"));
        remove_test_dir(&root);
    }

    #[test]
    fn missing_shell_routes_return_validation_errors() {
        let manager = ShellManager::default();
        assert!(manager.write_input("missing", "echo test").is_err());
        assert!(manager.reset_directory("missing").is_err());
        assert!(manager
            .resize(&ResizeShellInput {
                shell_id: "missing".to_string(),
                rows: 24,
                cols: 80,
            })
            .is_err());
    }

    #[test]
    fn default_shell_and_cd_escaping_are_platform_specific() {
        assert!(!default_shell().trim().is_empty());
        let root = Path::new("folder with spaces");
        let rendered = if cfg!(target_os = "windows") {
            format!("cd /d \"{}\"\r\n", root.display())
        } else {
            let escaped = root.to_string_lossy().replace('\'', "'\"'\"'");
            format!("cd '{escaped}'\n")
        };
        assert!(rendered.contains("folder with spaces"));
    }

    #[test]
    fn manager_routes_input_resize_and_cleanup_by_shell_id() {
        let root = temp_dir("manager");
        std::fs::create_dir_all(&root).expect("root");
        let manager = ShellManager::default();
        manager
            .insert("shell-one".to_string(), managed_test_shell("session-one", &root))
            .expect("insert first");
        manager
            .insert("shell-two".to_string(), managed_test_shell("session-two", &root))
            .expect("insert second");
        assert_eq!(manager.shells.lock().expect("shell map").len(), 2);
        manager.write_input("shell-one", if cfg!(windows) { "echo test\r\n" } else { "echo test\n" }).expect("input");
        manager
            .resize(&ResizeShellInput {
                shell_id: "shell-two".to_string(),
                rows: 30,
                cols: 100,
            })
            .expect("resize");
        assert_eq!(manager.stop("shell-one").expect("stop first").as_deref(), Some("session-one"));
        assert_eq!(manager.stop("shell-one").expect("repeat stop"), None);
        assert_eq!(manager.stop("shell-two").expect("stop second").as_deref(), Some("session-two"));
        assert!(manager.shells.lock().expect("shell map").is_empty());
        remove_test_dir(&root);
    }

    #[test]
    fn manager_cleans_up_only_the_requested_session_shells() {
        let root = temp_dir("session-cleanup");
        std::fs::create_dir_all(&root).expect("root");
        let manager = ShellManager::default();
        manager
            .insert("shell-one".to_string(), managed_test_shell("session-one", &root))
            .expect("insert first");
        manager
            .insert("shell-two".to_string(), managed_test_shell("session-two", &root))
            .expect("insert second");

        let stopped = manager.stop_for_session("session-one").expect("stop session");

        assert_eq!(stopped, vec![("shell-one".to_string(), "session-one".to_string())]);
        assert!(manager.shells.lock().expect("shell map").contains_key("shell-two"));
        assert_eq!(manager.stop_for_session("session-one").expect("repeat cleanup"), Vec::new());
        manager.stop("shell-two").expect("stop remaining");
        remove_test_dir(&root);
    }

    #[test]
    fn invalid_shell_executable_fails_to_spawn() {
        let pair = native_pty_system()
            .openpty(terminal_size(24, 80))
            .expect("test pty");
        let command = CommandBuilder::new("vanehub-shell-executable-that-does-not-exist");
        assert!(pair.slave.spawn_command(command).is_err());
    }
}
