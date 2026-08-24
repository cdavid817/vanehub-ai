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

/// The part of a file that ships in the binary.
///
/// Everything from the first `#[cfg(test)]` on is compiled out, so a rule about what the product
/// does must not read it. The alternative — exempting whole files — would also exempt the
/// production code sitting above the test module in the same file.
fn production_of(relative: &str) -> String {
    let code = code_of(relative);
    code.split("#[cfg(test)]")
        .next()
        .unwrap_or_default()
        .to_string()
}

/// The export path holds no handle to the index, anywhere down its chain.
///
/// The command-level check below catches the obvious version. This catches the version that arrives
/// later: an export that still looks file-backed at the command, and reaches the index two calls
/// down where nobody reviewing the command would see it.
#[test]
fn the_export_implementation_never_reaches_the_log_index() {
    let code = code_of("contexts/workspaces/infrastructure/session_queries.rs");
    let forbidden = [
        "SqliteLogIndexRepository",
        "SessionLogIndexRepository",
        "unified_log_query_index",
        "IndexedSessionLogRecord",
        "operations::log_api",
    ];

    let mut violations = Vec::new();
    for name in forbidden {
        if code.contains(name) {
            violations.push(format!(
                "[ARCH-LOGINDEX-005] session_queries.rs: names `{name}`. Repair: an export reads \
                 the redacted files, which are the durable record the index only projects"
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Platform logging owns the durable write, and the operations index never joins it.
///
/// The index is rebuildable precisely because it is downstream of the append and never part of it.
/// One write from the index side and the projection becomes a second source of truth — which would
/// make "delete it and rebuild" lossy, and that property is what every other decision here rests
/// on.
#[test]
fn the_log_index_never_writes_to_the_durable_log() {
    let mut violations = Vec::new();
    for relative in [
        "contexts/operations/infrastructure/log_index_repository.rs",
        "contexts/operations/infrastructure/log_index_repair_store.rs",
        "contexts/operations/infrastructure/log_source_reader.rs",
        "contexts/operations/application/log_repair.rs",
    ] {
        // Production only. A fixture writing a log file is how a log reader is tested at all, and
        // that code is compiled out of the binary this rule is about.
        let code = production_of(relative);
        for name in [
            "OpenOptions",
            "File::create",
            "fs::write",
            "append_log",
            "record_appended",
        ] {
            if code.contains(name) {
                violations.push(format!(
                    "[ARCH-LOGINDEX-006] {relative}: names `{name}`. Repair: the index is a \
                     read-only projection of the log; the durable write belongs to platform logging"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// The interactive query and the export stay two implementations of two different questions.
///
/// They read different stores on purpose, so the failure mode is not one calling the other — it is
/// one quietly becoming the other's implementation, at which point the store the export reads is
/// decided by whichever call path happened to be reused.
#[test]
fn the_interactive_query_and_the_export_share_no_implementation() {
    let query = code_of("contexts/operations/application/log_query_service.rs");

    // The query service knows how to prepare an export — which files, which window — and must not
    // know how to produce one. `export_preparation` returns names and boundaries, never records.
    assert!(
        query.contains("export_preparation"),
        "nothing tells an export which files it may read"
    );
    for forbidden in [
        "all_filtered_log_entries",
        "filter_log_entries",
        "log_files(",
    ] {
        assert!(
            !query.contains(forbidden),
            "the query service names `{forbidden}`, which belongs to the export path"
        );
    }
    // And the export's own scope predicate is one function, so a preview and an export cannot
    // disagree about which records are in scope.
    let export = code_of("contexts/workspaces/infrastructure/session_queries.rs");
    assert_eq!(
        export.matches("fn log_entry_matches").count(),
        1,
        "the export scope predicate exists more than once"
    );
}

/// The export destination comes from the native picker and from nothing else.
///
/// A frontend-supplied path would make an export an arbitrary filesystem write with the
/// application's privileges, addressed by a string that crossed the IPC boundary. The picker is
/// what makes the destination something the user chose rather than something a caller named.
#[test]
fn the_export_destination_comes_only_from_the_native_picker() {
    let code = production_of("contexts/workspaces/infrastructure/session_queries.rs");

    let export = code
        .split("pub(crate) fn export_session_logs")
        .nth(1)
        .expect("the export exists")
        .split("\n}")
        .next()
        .unwrap_or_default();
    assert!(
        export.contains("blocking_save_file"),
        "the export does not use the native destination picker"
    );
    // The query carries filters and a session id, never a destination — so there is no path on the
    // wire for a caller to supply in the first place.
    let dto = production_of("commands/workspaces/dto.rs");
    let query = dto
        .split("pub(crate) struct SessionLogQuery")
        .nth(1)
        .expect("the query DTO exists")
        .split("\n}")
        .next()
        .unwrap_or_default();
    for forbidden in ["path", "destination", "target", "file_name"] {
        assert!(
            !query.contains(forbidden),
            "the log query DTO carries `{forbidden}`, which would name an export destination"
        );
    }
}

/// The export writes through a temporary file rather than onto the destination directly.
///
/// A write that failed partway through the destination leaves a truncated log under a name that
/// promises the whole thing — and the user has no way to tell, because a log file is expected to
/// end wherever it ends.
#[test]
fn the_export_writes_through_a_temporary_file() {
    let code = production_of("contexts/workspaces/infrastructure/session_queries.rs");

    assert!(
        code.contains("NamedTempFile"),
        "the export writes directly to its destination"
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
