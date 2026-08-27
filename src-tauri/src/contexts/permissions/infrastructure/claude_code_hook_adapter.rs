//! Constructs and installs/removes VaneHub's `PreToolUse` entries in Claude Code's global
//! `settings.json`, via `cli_config`'s independent hook-projection operation
//! (`add-claude-code-permission-callback` design.md D7). `cli_config` itself stays agnostic to
//! what these entries contain — this adapter is the one place that knows the matcher list and
//! the wrapper binary's path.

use crate::contexts::permissions::application::{ClaudeCodeHookPort, PermissionsApplicationError};
use crate::contexts::tooling::cli_config::api::ClaudeCodeHookProjectionPort;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Every built-in tool `claude-code-permission-hook`'s mapping table
/// (`hook_bridge_mapping::map_tool_to_action`) covers, pipe-separated per Claude Code's matcher
/// syntax — kept in sync with that table by hand, since the two live in different bounded
/// contexts and the mapping itself is deliberately not shared code.
const BUILT_IN_TOOLS_MATCHER: &str = "Bash|Edit|Write|Read|Glob|Grep";

/// Matches every MCP tool call (`mcp__<server>__<tool>`) via Claude Code's documented regex
/// matcher support — confirmed against official docs this session, not guessed.
const MCP_TOOLS_MATCHER: &str = "mcp__.*";

/// Above the wrapper's own internal HTTP-call timeout (320s, design.md D8), so the wrapper
/// always resolves the decision itself before this hook-level timeout would ever fire; well
/// below Claude Code's own 600s ceiling.
const HOOK_TIMEOUT_SECONDS: u64 = 330;

// design.md D8's ordering constraint, enforced at compile time rather than by a runtime test:
// wrapper's own timeout (320s) < this hook-level timeout < Claude Code's 600s ceiling.
const _: () = assert!(HOOK_TIMEOUT_SECONDS > 320);
const _: () = assert!(HOOK_TIMEOUT_SECONDS < 600);

pub(crate) struct ClaudeCodeHookAdapter {
    projection: Arc<dyn ClaudeCodeHookProjectionPort>,
    wrapper_path: PathBuf,
}

impl ClaudeCodeHookAdapter {
    pub(crate) fn new(
        projection: Arc<dyn ClaudeCodeHookProjectionPort>,
        wrapper_path: PathBuf,
    ) -> Self {
        Self {
            projection,
            wrapper_path,
        }
    }

    fn entries(&self) -> Vec<Value> {
        hook_entries(&shell_command_path(&self.wrapper_path))
    }
}

/// Claude Code executes hook commands through a POSIX-compatible shell even on Windows. Raw
/// Windows paths therefore lose every backslash as an escape and paths containing spaces split
/// into multiple arguments. Forward slashes are accepted by Windows executables and quoting the
/// result keeps the path a single shell token on every supported desktop platform.
fn shell_command_path(path: &Path) -> String {
    let normalized = path.to_string_lossy().replace('\\', "/");
    format!("'{}'", normalized.replace('\'', "'\\''"))
}

impl ClaudeCodeHookPort for ClaudeCodeHookAdapter {
    fn install(&self) -> Result<(), PermissionsApplicationError> {
        // These entries land in Claude Code's *global* settings, where they outlive this process
        // and run on every tool call. Pointing one at a binary that is not on disk would make each
        // of those calls invoke a command that cannot start, in a file VaneHub does not own.
        // Refusing leaves that file untouched and reports why.
        if !self.wrapper_path.is_file() {
            return Err(PermissionsApplicationError::infrastructure(
                "cli_config",
                format!(
                    "the permission hook binary is missing at {}, so Claude Code hook management \
                     cannot be enabled",
                    self.wrapper_path.display()
                ),
            ));
        }
        self.projection
            .set_permission_hook_entries(&self.entries())
            .map_err(|error| {
                PermissionsApplicationError::infrastructure("cli_config", error.to_string())
            })
    }

    fn remove(&self) -> Result<(), PermissionsApplicationError> {
        self.projection
            .set_permission_hook_entries(&[])
            .map_err(|error| {
                PermissionsApplicationError::infrastructure("cli_config", error.to_string())
            })
    }
}

fn hook_entries(command: &str) -> Vec<Value> {
    [BUILT_IN_TOOLS_MATCHER, MCP_TOOLS_MATCHER]
        .into_iter()
        .map(|matcher| {
            json!({
                "matcher": matcher,
                "hooks": [{
                    "type": "command",
                    "command": command,
                    "timeout": HOOK_TIMEOUT_SECONDS,
                }]
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli_config::api::CliConfigError;
    use crate::test_support::TempDirectory;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeProjection {
        calls: Mutex<Vec<Vec<Value>>>,
        fail_next: Mutex<bool>,
    }

    impl ClaudeCodeHookProjectionPort for FakeProjection {
        fn set_permission_hook_entries(&self, entries: &[Value]) -> Result<(), CliConfigError> {
            if std::mem::take(&mut *self.fail_next.lock().unwrap()) {
                return Err(CliConfigError::Repository);
            }
            self.calls.lock().unwrap().push(entries.to_vec());
            Ok(())
        }
    }

    #[test]
    fn install_sends_one_entry_per_matcher_pointing_at_the_wrapper_path() {
        let directory = TempDirectory::new("claude-code-hook-install");
        let wrapper = directory.write("vanehub-permission-hook", "");
        let projection = Arc::new(FakeProjection::default());
        let adapter = ClaudeCodeHookAdapter::new(projection.clone(), wrapper.clone());

        adapter.install().expect("install should succeed");

        let calls = projection.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        let entries = &calls[0];
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0]["matcher"], "Bash|Edit|Write|Read|Glob|Grep");
        assert_eq!(entries[1]["matcher"], "mcp__.*");
        for entry in entries {
            assert_eq!(entry["hooks"][0]["command"], shell_command_path(&wrapper));
            assert_eq!(entry["hooks"][0]["type"], "command");
            assert_eq!(entry["hooks"][0]["timeout"], 330);
        }
    }

    #[test]
    fn windows_wrapper_path_is_bash_safe() {
        let command = shell_command_path(Path::new(
            r"D:\c00606997\Documents\code OpenSource\vanehub-ai\target\debug\vanehub-permission-hook.exe",
        ));

        assert_eq!(
            command,
            "'D:/c00606997/Documents/code OpenSource/vanehub-ai/target/debug/vanehub-permission-hook.exe'"
        );
        assert!(!command.contains('\\'));
    }

    #[test]
    fn wrapper_path_with_apostrophe_remains_one_shell_token() {
        assert_eq!(
            shell_command_path(Path::new("/tmp/user's tools/vanehub-permission-hook")),
            "'/tmp/user'\\''s tools/vanehub-permission-hook'"
        );
    }

    #[test]
    fn install_refuses_when_the_wrapper_binary_is_absent_and_writes_nothing() {
        let directory = TempDirectory::new("claude-code-hook-absent");
        let projection = Arc::new(FakeProjection::default());
        let adapter = ClaudeCodeHookAdapter::new(
            projection.clone(),
            directory.path().join("vanehub-permission-hook"),
        );

        let result = adapter.install();

        assert!(matches!(
            result,
            Err(PermissionsApplicationError::Infrastructure {
                category: "cli_config",
                ..
            })
        ));
        assert!(
            projection.calls.lock().unwrap().is_empty(),
            "the user's global Claude Code settings must be left untouched"
        );
    }

    #[test]
    fn remove_still_works_when_the_wrapper_binary_is_absent() {
        let directory = TempDirectory::new("claude-code-hook-cleanup");
        let projection = Arc::new(FakeProjection::default());
        let adapter = ClaudeCodeHookAdapter::new(
            projection.clone(),
            directory.path().join("vanehub-permission-hook"),
        );

        adapter
            .remove()
            .expect("a previously installed hook must stay removable");

        assert_eq!(projection.calls.lock().unwrap().len(), 1);
    }

    #[test]
    fn remove_sends_an_empty_entry_list() {
        let projection = Arc::new(FakeProjection::default());
        let adapter =
            ClaudeCodeHookAdapter::new(projection.clone(), PathBuf::from("/opt/vanehub-hook"));

        adapter.remove().expect("remove should succeed");

        let calls = projection.calls.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].is_empty());
    }

    #[test]
    fn projection_failure_surfaces_as_an_infrastructure_error() {
        let directory = TempDirectory::new("claude-code-hook-projection-failure");
        let wrapper = directory.write("vanehub-permission-hook", "");
        let projection = Arc::new(FakeProjection::default());
        *projection.fail_next.lock().unwrap() = true;
        let adapter = ClaudeCodeHookAdapter::new(projection, wrapper);

        let result = adapter.install();

        assert!(matches!(
            result,
            Err(PermissionsApplicationError::Infrastructure {
                category: "cli_config",
                ..
            })
        ));
    }
}
