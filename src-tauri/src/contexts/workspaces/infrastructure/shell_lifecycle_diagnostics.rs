//! Where a Shell lifecycle no-op goes so it is not silent.
//!
//! One line through unified logging, with identifiers and counts only. The events it records are
//! correct no-ops — a Reaper completion for a generation that has been replaced must not release
//! capacity — and a no-op leaves nothing behind. If the stale path fires for a reason nobody
//! predicted, this is what will say so.
//!
//! Nothing here carries a command, terminal output, a host, or a path. The Shell being named is one
//! whose contents are exactly what must not travel; what an operator needs is which Shell, which
//! generation was attempted, and which one is current.

use crate::contexts::workspaces::application::ShellLifecycleDiagnosticsPort;
use crate::platform::logging::{fallback_log_dir, write_message_raw, LogLevel};
use std::collections::BTreeMap;

pub(crate) struct UnifiedLogShellDiagnostics;

impl ShellLifecycleDiagnosticsPort for UnifiedLogShellDiagnostics {
    fn stale_reaper_completion(
        &self,
        shell_id: &str,
        attempted_generation: u64,
        current_generation: u64,
    ) {
        // A write failure is swallowed. This is a diagnostic about a path that is already doing
        // nothing; propagating from here would turn "we could not record a no-op" into a failure of
        // the sweep that has other Shells to attend to.
        let _ = write_message_raw(
            &fallback_log_dir(),
            LogLevel::Warn,
            "session-shell",
            "reaper completion named a generation that is no longer current",
            BTreeMap::from([
                ("shell_id".to_string(), shell_id.to_string()),
                (
                    "attempted_generation".to_string(),
                    attempted_generation.to_string(),
                ),
                (
                    "current_generation".to_string(),
                    current_generation.to_string(),
                ),
            ]),
        );
    }

    fn orphaned_reaper_completion(&self, shell_id: &str, attempted_generation: u64) {
        let _ = write_message_raw(
            &fallback_log_dir(),
            LogLevel::Warn,
            "session-shell",
            "reaper completion named a shell with no entry",
            BTreeMap::from([
                ("shell_id".to_string(), shell_id.to_string()),
                (
                    "attempted_generation".to_string(),
                    attempted_generation.to_string(),
                ),
            ]),
        );
    }
}
