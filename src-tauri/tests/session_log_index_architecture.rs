//! Boundary rules for the migrated session-log query surface.
//!
//! Kept out of `architecture.rs` because these read a named set of files rather than walking the
//! tree, and because the rule they encode belongs to one migration: interactive log queries answer
//! from the operations-owned index, exports answer from the redacted files, and neither grows a
//! path into the other. The failure they exist to catch is not a crash — it is two implementations
//! of the same question, reached under conditions the reader cannot see.

use std::fs;
use std::path::{Path, PathBuf};

/// Files that hold a session-log command. Named rather than globbed: a new one that nobody added
/// here would also be a new one nobody reviewed against these rules.
const LOG_COMMAND_FILES: &[&str] = &[
    "commands/workspaces/list_session_logs.rs",
    "commands/workspaces/session_log_index.rs",
    "commands/workspaces/session_log_mapper.rs",
];

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Prose is stripped before scanning. These files explain the rules they follow, and a scan that
/// read the explanation would flag the sentence saying the dependency is absent.
fn code_of(relative: &str) -> String {
    let source = fs::read_to_string(source_root().join(relative))
        .unwrap_or_else(|_| panic!("a session-log command file is missing: {relative}"));
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// No query command reaches the workspaces file scanner.
///
/// The scanner is still compiled — the export path uses its sibling — so nothing stops a query
/// command from calling it, and a fallback added "just for when the index is empty" would be
/// reached exactly when the index is least trustworthy. Two implementations with different
/// filters, different bounds and different coverage, and no way for the reader to tell which one
/// answered.
#[test]
fn session_log_query_commands_never_reach_the_workspaces_file_scanner() {
    let forbidden = [
        "WorkspaceApi",
        "SessionWorkspaceQueryPort",
        "session_queries",
        "query_logs",
        "all_filtered_log_entries",
        "active_log_dir",
    ];

    let mut violations = Vec::new();
    for relative in LOG_COMMAND_FILES {
        let code = code_of(relative);
        for name in forbidden {
            if code.contains(name) {
                violations.push(format!(
                    "[ARCH-LOGINDEX-001] {relative}: names `{name}`. Repair: answer from the \
                     operations log index, and let the index report that it cannot answer"
                ));
            }
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// A command speaks to the operations API, never to its storage.
///
/// `SessionLogApi` is where the bounds live: the page ceiling, the cursor check, the repair that
/// runs once rather than concurrently. A command holding the repository directly would be outside
/// all three, and would look identical at the call site.
#[test]
fn session_log_commands_never_import_operations_infrastructure() {
    let forbidden = [
        "operations::infrastructure",
        "SqliteLogIndexRepository",
        "UnifiedLogSourceReader",
        "SessionLogQueryService",
        "NativeDatabase",
        "rusqlite",
    ];

    let mut violations = Vec::new();
    for relative in LOG_COMMAND_FILES {
        let code = code_of(relative);
        for name in forbidden {
            if code.contains(name) {
                violations.push(format!(
                    "[ARCH-LOGINDEX-002] {relative}: names `{name}`. Repair: depend on \
                     `operations::log_api::SessionLogApi`, which is where the query bounds live"
                ));
            }
        }
    }

    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// The retired file-scan query command is not registered.
///
/// Retiring an implementation and leaving its command registered produces a surface where the same
/// question has two answers and the caller picks by name. The registry is the only place that
/// decides which one a client can reach, so it is the only place worth asserting.
#[test]
fn production_registers_no_file_scanning_session_log_query_command() {
    let registry = concat!(
        include_str!("../src/commands/core_registry.rs"),
        include_str!("../src/commands/builtin_tool_registry.rs"),
        include_str!("../src/commands/supplemental_registry.rs")
    );

    for retired in [
        "scan_session_logs",
        "list_session_log_files",
        "query_session_log_files",
        "list_session_logs_from_files",
    ] {
        assert!(
            !registry.contains(retired),
            "[ARCH-LOGINDEX-003] the registry still exposes `{retired}`, a second answer to a \
             question `list_session_logs` already answers"
        );
    }

    // And the one that remains is registered exactly once: a duplicate registration is a runtime
    // panic on startup rather than a compile error, so it is worth catching here.
    assert_eq!(
        registry
            .matches("commands::workspaces::list_session_logs::list_session_logs")
            .count(),
        1,
        "[ARCH-LOGINDEX-003] `list_session_logs` is not registered exactly once"
    );
}

/// The export never reads index rows.
///
/// The redacted files are the durable record; the index is a projection that can be behind,
/// partial, or mid-repair. An export assembled from the projection would present whichever of
/// those states it happened to catch as the record itself, and it is the artifact a user keeps.
#[test]
fn the_export_command_never_sources_rows_from_the_log_index() {
    let code = code_of("commands/workspaces/export_session_logs.rs");
    let forbidden = [
        "SessionLogApi",
        "session_log_mapper",
        "IndexedSessionLog",
        "log_api",
        "unified_log_query_index",
    ];

    let mut violations = Vec::new();
    for name in forbidden {
        if code.contains(name) {
            violations.push(format!(
                "[ARCH-LOGINDEX-004] export_session_logs.rs: names `{name}`. Repair: export from \
                 the redacted files, which are the durable record the index only projects"
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));

    // The index may still *describe* an export — which files it covers, and the window they span —
    // because that answer comes from coverage rather than from rows.
    let sources = code_of("commands/workspaces/session_log_index.rs");
    assert!(
        sources.contains("get_session_log_export_sources"),
        "[ARCH-LOGINDEX-004] nothing tells an export which files it may read"
    );
}
