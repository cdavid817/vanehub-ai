//! The `--uninstall` escape hatch (`add-permission-hook-recovery`'s "Wrapper provides a
//! standalone uninstall escape hatch"): removes VaneHub's own `PreToolUse` entries from Claude
//! Code's global settings so a machine whose VaneHub instance is gone can be released with one
//! command instead of hand-edited JSON. Runs entirely offline — it must work precisely when
//! VaneHub cannot.
//!
//! The settings path (`~/.claude/settings.json`), the ownership marker, and the removal
//! predicate are duplicated by hand from `contexts/tooling/cli_config`'s `primary_path`,
//! `PERMISSION_HOOK_MARKER`, and `is_vanehub_owned` — the same keep-in-sync contract as the
//! discovery-file duplication documented in `main.rs`, and for the same reason: this binary
//! deliberately never links `vanehub_ai_lib`.

use std::io;
use std::path::{Path, PathBuf};

/// Must match `live_config.rs`'s `PERMISSION_HOOK_MARKER`, which in turn names this very
/// binary — an entry is VaneHub-owned iff any of its hook commands mentions it.
const PERMISSION_HOOK_MARKER: &str = "vanehub-permission-hook";

pub(crate) fn run() -> Result<String, String> {
    uninstall_from(&settings_path()?)
}

/// Must match `live_config.rs`'s `primary_path("claude-code")`.
fn settings_path() -> Result<PathBuf, String> {
    dirs::home_dir()
        .map(|home| home.join(".claude").join("settings.json"))
        .ok_or_else(|| "could not resolve the home directory".to_string())
}

fn uninstall_from(path: &Path) -> Result<String, String> {
    let raw = match std::fs::read_to_string(path) {
        Ok(raw) => raw,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(format!(
                "nothing to remove: {} does not exist",
                path.display()
            ));
        }
        Err(error) => return Err(format!("could not read {}: {error}", path.display())),
    };
    let mut document: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
        format!(
            "{} is not valid JSON ({error}); refusing to modify it",
            path.display()
        )
    })?;

    let removed = remove_owned_entries(&mut document)
        .map_err(|reason| format!("{}: {reason}; refusing to modify it", path.display()))?;
    if removed == 0 {
        return Ok(format!(
            "nothing to remove: no VaneHub-owned PreToolUse entries in {}",
            path.display()
        ));
    }

    let bytes = serde_json::to_vec_pretty(&document)
        .map_err(|error| format!("could not re-serialize the settings document: {error}"))?;
    write_replacing(path, &bytes)?;
    Ok(format!(
        "removed {removed} VaneHub-owned PreToolUse hook entr{} from {}",
        if removed == 1 { "y" } else { "ies" },
        path.display()
    ))
}

/// `Ok(0)` when the hooks structure simply isn't there; `Err` when it exists with an unexpected
/// shape — guessing at a malformed document risks destroying settings this tool doesn't own.
fn remove_owned_entries(document: &mut serde_json::Value) -> Result<usize, &'static str> {
    let Some(root) = document.as_object_mut() else {
        return Err("root is not a JSON object");
    };
    let Some(hooks) = root.get_mut("hooks") else {
        return Ok(0);
    };
    let Some(hooks) = hooks.as_object_mut() else {
        return Err("hooks is not a JSON object");
    };
    let Some(pre_tool_use) = hooks.get_mut("PreToolUse") else {
        return Ok(0);
    };
    let Some(pre_tool_use) = pre_tool_use.as_array_mut() else {
        return Err("hooks.PreToolUse is not a JSON array");
    };
    let before = pre_tool_use.len();
    pre_tool_use.retain(|entry| !is_vanehub_owned(entry));
    Ok(before - pre_tool_use.len())
}

/// Must match `live_config.rs`'s `is_vanehub_owned`.
fn is_vanehub_owned(entry: &serde_json::Value) -> bool {
    entry
        .get("hooks")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|hooks| {
            hooks.iter().any(|hook| {
                hook.get("command")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(|command| command.contains(PERMISSION_HOOK_MARKER))
            })
        })
}

/// Same-directory temp file plus rename, so a crash mid-write can never leave a truncated
/// settings file behind — this tool's whole reason to exist is recovering from crashes.
fn write_replacing(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file_name = path
        .file_name()
        .map(std::ffi::OsStr::to_os_string)
        .ok_or_else(|| format!("{} has no file name to replace", path.display()))?;
    file_name.push(format!(".vanehub-uninstall-{}", std::process::id()));
    let temp_path = path.with_file_name(file_name);

    std::fs::write(&temp_path, bytes)
        .map_err(|error| format!("could not write {}: {error}", temp_path.display()))?;
    std::fs::rename(&temp_path, path).map_err(|error| {
        let _ = std::fs::remove_file(&temp_path);
        format!("could not replace {}: {error}", path.display())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct TempSettings {
        directory: PathBuf,
    }

    impl TempSettings {
        fn new(name: &str) -> Self {
            let directory = std::env::temp_dir().join(format!(
                "vanehub-hook-uninstall-test-{}-{name}",
                std::process::id()
            ));
            std::fs::create_dir_all(&directory).expect("create temp directory");
            Self { directory }
        }

        fn path(&self) -> PathBuf {
            self.directory.join("settings.json")
        }
    }

    impl Drop for TempSettings {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.directory);
        }
    }

    fn owned_entry() -> serde_json::Value {
        json!({
            "matcher": "Bash|Edit|Write|Read|Glob|Grep",
            "hooks": [{"type": "command", "command": "/opt/bin/vanehub-permission-hook", "timeout": 330}]
        })
    }

    fn foreign_entry() -> serde_json::Value {
        json!({
            "matcher": "Bash",
            "hooks": [{"type": "command", "command": "/usr/local/bin/some-other-guard"}]
        })
    }

    #[test]
    fn removes_owned_entries_and_preserves_everything_else() {
        let temp = TempSettings::new("removes");
        let document = json!({
            "model": "sonnet",
            "hooks": {
                "PreToolUse": [owned_entry(), foreign_entry(), owned_entry()],
                "PostToolUse": [{"matcher": "Edit", "hooks": []}]
            }
        });
        std::fs::write(temp.path(), document.to_string()).expect("seed settings");

        let message = uninstall_from(&temp.path()).expect("uninstall should succeed");
        assert!(
            message.contains("removed 2"),
            "unexpected message: {message}"
        );

        let raw = std::fs::read_to_string(temp.path()).expect("read back");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["model"], "sonnet");
        assert_eq!(value["hooks"]["PreToolUse"], json!([foreign_entry()]));
        assert_eq!(value["hooks"]["PostToolUse"][0]["matcher"], "Edit");
    }

    #[test]
    fn missing_file_is_a_successful_no_op() {
        let temp = TempSettings::new("missing");
        let message = uninstall_from(&temp.path()).expect("should succeed");
        assert!(message.contains("nothing to remove"));
        assert!(!temp.path().exists());
    }

    #[test]
    fn settings_without_owned_entries_are_left_byte_identical() {
        let temp = TempSettings::new("no-owned");
        let original = json!({"hooks": {"PreToolUse": [foreign_entry()]}}).to_string();
        std::fs::write(temp.path(), &original).expect("seed settings");

        let message = uninstall_from(&temp.path()).expect("should succeed");
        assert!(message.contains("nothing to remove"));
        assert_eq!(
            std::fs::read_to_string(temp.path()).expect("read back"),
            original
        );
    }

    #[test]
    fn unparseable_settings_fail_without_modification() {
        let temp = TempSettings::new("garbage");
        std::fs::write(temp.path(), "{not json").expect("seed settings");

        let error = uninstall_from(&temp.path()).expect_err("should refuse");
        assert!(
            error.contains("not valid JSON"),
            "unexpected error: {error}"
        );
        assert_eq!(
            std::fs::read_to_string(temp.path()).expect("read back"),
            "{not json"
        );
    }

    #[test]
    fn malformed_hooks_shape_fails_without_modification() {
        let temp = TempSettings::new("bad-shape");
        let original = json!({"hooks": {"PreToolUse": "not an array"}}).to_string();
        std::fs::write(temp.path(), &original).expect("seed settings");

        let error = uninstall_from(&temp.path()).expect_err("should refuse");
        assert!(error.contains("PreToolUse"), "unexpected error: {error}");
        assert_eq!(
            std::fs::read_to_string(temp.path()).expect("read back"),
            original
        );
    }

    #[test]
    fn ownership_predicate_matches_live_config_semantics() {
        assert!(is_vanehub_owned(&owned_entry()));
        assert!(!is_vanehub_owned(&foreign_entry()));
        assert!(!is_vanehub_owned(&json!({"matcher": "Bash"})));
    }
}
