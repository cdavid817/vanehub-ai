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

/// The fields a stale-completion record carries, and nothing else.
///
/// Built as a value so a test can read it. The alternative — asserting on a written log file —
/// would test the log store, and the property that matters here is what this code *offers* it.
fn stale_context(
    shell_id: &str,
    attempted_generation: u64,
    current_generation: u64,
) -> BTreeMap<String, String> {
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
    ])
}

fn orphaned_context(shell_id: &str, attempted_generation: u64) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("shell_id".to_string(), shell_id.to_string()),
        (
            "attempted_generation".to_string(),
            attempted_generation.to_string(),
        ),
    ])
}

fn rollback_context(shell_id: &str, generation: u64, reason: &str) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("shell_id".to_string(), shell_id.to_string()),
        ("generation".to_string(), generation.to_string()),
        // A reason code from the fixed lifecycle vocabulary, never a platform error string: those
        // quote paths and command lines.
        ("reason".to_string(), reason.to_string()),
    ])
}

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
            stale_context(shell_id, attempted_generation, current_generation),
        );
    }

    fn startup_rollback_unconfirmed(&self, shell_id: &str, generation: u64, reason: &str) {
        let _ = write_message_raw(
            &fallback_log_dir(),
            LogLevel::Warn,
            "session-shell",
            "startup rollback could not confirm the child had ended",
            rollback_context(shell_id, generation, reason),
        );
    }

    fn orphaned_reaper_completion(&self, shell_id: &str, attempted_generation: u64) {
        let _ = write_message_raw(
            &fallback_log_dir(),
            LogLevel::Warn,
            "session-shell",
            "reaper completion named a shell with no entry",
            orphaned_context(shell_id, attempted_generation),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::domain::shell_reason_code;

    /// Bounded, and bounded by name rather than by count alone.
    ///
    /// A record whose field set can grow is one somebody adds a path to. Naming the three keys means
    /// a fourth has to be added here too, which is where a reviewer sees it.
    #[test]
    fn a_stale_record_carries_three_named_fields_and_no_others() {
        let context = stale_context("shell-1", 7, 8);

        assert_eq!(
            context.keys().cloned().collect::<Vec<_>>(),
            vec![
                "attempted_generation".to_string(),
                "current_generation".to_string(),
                "shell_id".to_string()
            ]
        );
        assert_eq!(context["shell_id"], "shell-1");
        assert_eq!(context["attempted_generation"], "7");
        assert_eq!(context["current_generation"], "8");
    }

    #[test]
    fn an_orphaned_record_carries_two_named_fields_and_no_others() {
        let context = orphaned_context("shell-1", 7);

        assert_eq!(
            context.keys().cloned().collect::<Vec<_>>(),
            vec!["attempted_generation".to_string(), "shell_id".to_string()]
        );
    }

    /// The values are the identifiers they were given, and the identifiers cannot be anything else.
    ///
    /// A shell id is minted by this process and a generation is a counter, so neither can carry a
    /// command, terminal output, a credential, a hostname, or a path. What this asserts is that
    /// nothing *derives* a value from something that could — the record is the arguments, unchanged.
    #[test]
    fn every_value_is_one_of_the_identifiers_it_was_handed() {
        let context = stale_context("shell-1", 7, 8);

        for value in context.values() {
            assert!(
                ["shell-1", "7", "8"].contains(&value.as_str()),
                "unexpected value {value}"
            );
        }
    }

    /// A shell id shaped like a path still travels as the id it is, because that is what it is.
    ///
    /// The guard against a path reaching a log is not a filter here — it is that the only string
    /// this record accepts is a `ShellId`, which the id port mints and nothing else constructs.
    /// Recorded as a test so the next person to widen the signature reads why they should not.
    #[test]
    fn a_rollback_record_carries_three_named_fields_and_a_reason_code() {
        let context = rollback_context("shell-1", 3, shell_reason_code::STARTUP_CLEANUP_PENDING);

        assert_eq!(
            context.keys().cloned().collect::<Vec<_>>(),
            vec![
                "generation".to_string(),
                "reason".to_string(),
                "shell_id".to_string()
            ]
        );
        // A code from the fixed vocabulary, never a platform error string: those quote paths and
        // command lines, which is exactly what must not reach a log from a Shell.
        assert_eq!(context["reason"], "shell_startup_cleanup_pending");
    }

    #[test]
    fn the_record_takes_an_identifier_rather_than_free_text() {
        let context = stale_context("shell-42", 1, 2);

        assert_eq!(context.len(), 3);
        assert_eq!(context["shell_id"], "shell-42");
    }
}
