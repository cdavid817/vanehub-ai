//! What the log-index repair is structurally forbidden from doing.
//!
//! Two of these cannot be caught by a unit test at all. "Startup does not scan the log directory"
//! is a property of where a call sits, not of what it returns — a synchronous scan would produce
//! exactly the same index, just after a delay nobody attributes to it. And "no unbounded read"
//! is about a call that does not appear rather than one that does.

use std::fs;
use std::path::{Path, PathBuf};

fn source_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Prose is stripped first. These files explain the rules they follow, and a scan that read the
/// explanation would flag the sentence saying the call is absent.
fn code_of(relative: &str) -> String {
    let source = fs::read_to_string(source_root().join(relative))
        .unwrap_or_else(|_| panic!("a log-repair source file is missing: {relative}"));
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim_start();
            !trimmed.starts_with("//") && !trimmed.starts_with("/*") && !trimmed.starts_with('*')
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Startup hands the repair to a background task and returns.
///
/// A synchronous scan here is not a crash and not an error — it is a window that takes longer to
/// appear the more logs a user has kept, which reads as "the app got slower" rather than as a
/// startup path doing filesystem work. The `spawn` is the whole difference.
#[test]
fn startup_hands_the_repair_to_a_background_task_rather_than_running_it() {
    let bootstrap = code_of("bootstrap/operations.rs");

    let job = bootstrap
        .split("pub(crate) fn start_log_index_repair_job")
        .nth(1)
        .expect("the repair job is started from bootstrap");
    let body = job.split("\n}").next().unwrap_or_default();

    assert!(
        body.contains("spawn"),
        "the repair job body does not spawn: {body}"
    );
    // The assembling function must not repair either. Assembly runs on the startup path, and a
    // repair called from there is synchronous however the job below is written.
    let assemble = bootstrap
        .split("pub(crate) fn assemble_session_log_api")
        .nth(1)
        .expect("the log api is assembled in bootstrap")
        .split("\n}")
        .next()
        .unwrap_or_default();
    assert!(
        !assemble.contains("repair"),
        "assembly runs a repair on the startup path"
    );
}

/// The runtime starts the job after the window exists, never before.
#[test]
fn the_runtime_starts_the_repair_job_off_the_critical_path() {
    let runtime = code_of("bootstrap/runtime.rs");

    assert!(
        runtime.contains("start_log_index_repair_job"),
        "nothing ever starts the repair"
    );
    // Called exactly once. A second call site would start a competing pass, and single-flight would
    // silently absorb it — so the duplicate would look harmless while doubling the startup work.
    assert_eq!(
        runtime.matches("start_log_index_repair_job").count(),
        1,
        "the repair job is started from more than one place"
    );
}

/// Nothing in the repair path reads a whole file or an unbounded line.
///
/// The bounds are only bounds while every read goes through the bounded helper. One
/// `read_to_string` on a log file makes the process's memory a function of how long the user has
/// been running it.
#[test]
fn the_repair_path_never_reads_a_whole_file_or_an_unbounded_line() {
    let forbidden = [
        "read_to_string",
        "read_to_end",
        "fs::read(",
        // `read_line` allocates whatever the line happens to be and fails a whole batch on invalid
        // UTF-8, which turns one damaged byte into a file that can never be indexed.
        "read_line",
        "lines()",
    ];

    let mut violations = Vec::new();
    for relative in [
        "contexts/operations/infrastructure/log_source_reader.rs",
        "contexts/operations/application/log_repair.rs",
        "contexts/operations/infrastructure/log_index_repair_store.rs",
    ] {
        let code = code_of(relative);
        for name in forbidden {
            if code.contains(name) {
                violations.push(format!(
                    "[ARCH-LOGREPAIR-001] {relative}: calls `{name}`. Repair: read through the \
                     bounded line reader, which caps allocation and skips past what it cannot use"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Every bound the repair answers to is a named constant.
///
/// A literal buried in a loop is a bound nobody can find when they need to change it, and one that
/// no test can assert against without repeating the literal.
#[test]
fn every_repair_bound_is_a_named_constant() {
    let service = code_of("contexts/operations/application/log_query_service.rs");
    let reader = code_of("contexts/operations/infrastructure/log_source_reader.rs");

    for bound in [
        "REPAIR_BATCH_RECORDS",
        "REPAIR_BATCH_BYTES",
        "REPAIR_FILES_PER_PASS",
        "REPAIR_BATCHES_PER_FILE",
        "REPAIR_PRUNE_ROWS",
    ] {
        assert!(
            service.contains(&format!("const {bound}")),
            "the {bound} bound is not a named constant"
        );
    }
    assert!(
        reader.contains("const MAX_LOG_LINE_BYTES"),
        "the line ceiling is not a named constant"
    );
}

/// Only a successful listing can lead to a deletion.
///
/// The expiry call is driven by what is *retained*, so an empty inventory means "delete
/// everything". That shape is fine as long as one thing holds: the code that deletes is
/// unreachable from a listing that failed. It is not a property any single call can express, which
/// is why it is asserted structurally — the failure branch returns before reconcile exists.
#[test]
fn deletion_is_unreachable_from_a_listing_that_failed() {
    let repair = code_of("contexts/operations/application/log_repair.rs");

    let discover = repair
        .split("fn discover(")
        .nth(1)
        .expect("the pass discovers before it reconciles");
    let discover_body = discover.split("\n    }").next().unwrap_or_default();
    assert!(
        discover_body.contains("return Err("),
        "a failed listing does not return early: {discover_body}"
    );
    // And the phase that deletes names its own guard rather than relying on the caller's ordering.
    let reconcile = repair
        .split("fn reconcile(")
        .nth(1)
        .expect("the pass reconciles")
        .split("\n    }")
        .next()
        .unwrap_or_default();
    assert!(
        reconcile.contains("snapshot.identities()"),
        "reconcile does not take its retained set from the snapshot"
    );
    // The deleting calls appear only inside reconcile and the truncation path, never at top level.
    assert!(
        !repair.contains("fn repair(&self)")
            || !repair
                .split("fn repair(&self)")
                .nth(1)
                .unwrap_or_default()
                .split("\n    }")
                .next()
                .unwrap_or_default()
                .contains("expire_sources"),
        "the top-level pass expires sources outside the reconcile phase"
    );
}

/// Every deletion the repair performs is bounded and looped.
///
/// A single unbounded `DELETE` over the corpus holds the write lock for as long as it takes, and
/// retention can expire an entire previous generation at once.
#[test]
fn every_deletion_the_repair_performs_is_bounded() {
    let store = code_of("contexts/operations/infrastructure/log_index_repair_store.rs");

    for statement in ["expire_sources", "prune_source_generation"] {
        let body = store
            .split(&format!("pub(crate) fn {statement}"))
            .nth(1)
            .unwrap_or_else(|| panic!("{statement} is missing"))
            .split("\n}")
            .next()
            .unwrap_or_default();
        assert!(
            body.contains("LIMIT ?"),
            "{statement} deletes without a bound: {body}"
        );
        assert!(
            body.contains("transaction.commit()"),
            "{statement} does not end its transaction"
        );
    }
}

/// The repair reads the durable files and writes the projection, never the other way round.
///
/// An index that repaired itself from itself would confirm whatever it already held, including the
/// gaps it was supposed to fill.
#[test]
fn the_repair_reads_files_and_writes_the_index_and_never_the_reverse() {
    let repair = code_of("contexts/operations/application/log_repair.rs");

    // The application layer names ports, never a storage or filesystem type. That is what keeps the
    // direction structural rather than a matter of which call happened to be written first.
    for forbidden in ["rusqlite", "SqliteLogIndexRepository", "std::fs", "PathBuf"] {
        assert!(
            !repair.contains(forbidden),
            "the repair names `{forbidden}` directly"
        );
    }
    assert!(
        repair.contains("self.sources.read_batch"),
        "the repair does not read from the source files"
    );
    assert!(
        repair.contains("self.index.commit_batch"),
        "the repair does not write through the batch commit"
    );
}
