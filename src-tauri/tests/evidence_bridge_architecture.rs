//! Boundary rules for the producer-to-journal evidence bridge.
//!
//! Kept out of `architecture.rs` because these read four specific files rather than walking the
//! tree the way that suite's shared helpers do, and because the rule they encode belongs to one
//! capability: a producer reports its own work without the journal becoming its dependency.

use std::fs;
use std::path::{Path, PathBuf};

/// Every producer's evidence port stays in the producer's own vocabulary.
///
/// The port lets a shell operation, a run, or a usage record report itself without the evidence
/// aggregate becoming a dependency of the context doing the work. That holds only while the port
/// file names none of the journal's types: one `SafeEvidencePayload` in a workspaces signal and
/// translation has moved into the producer, where the next person to add a field has the whole
/// payload enum in scope.
#[test]
fn producer_evidence_ports_never_name_the_evidence_aggregate() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let ports = [
        "contexts/agent_runtime/application/evidence.rs",
        "contexts/workspaces/application/evidence.rs",
        "contexts/operations/application/evidence.rs",
        "contexts/sessions/application/evidence.rs",
    ];
    let forbidden = [
        "execution_observability",
        "ExecutionEvidenceEvent",
        "SafeEvidencePayload",
        "RecordEvidenceInput",
        "EvidenceRepositoryPort",
        "ExecutionEvidenceApi",
        "EvidenceCorrelation",
    ];

    let mut violations = Vec::new();
    for relative in ports {
        let source = fs::read_to_string(source_root.join(relative)).unwrap_or_else(|_| {
            panic!("every producer context declares an evidence port: {relative}")
        });
        // Comments are stripped first. These files explain the rule they follow, and a scan that
        // read the prose would flag the sentence saying the type is absent.
        let code = strip_comments(&source);
        for name in forbidden {
            if code.contains(name) {
                violations.push(format!(
                    "[ARCH-EVIDENCE-001] {relative}: names `{name}`. Repair: keep the producer's \
                     own DTO and translate it in a bootstrap adapter"
                ));
            }
        }
        assert!(
            source.contains("try_publish"),
            "{relative} must expose a non-blocking try_publish"
        );
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Translation happens in bootstrap and nowhere else.
///
/// A producer that built a `RecordEvidenceInput` itself would map to the journal's shape by
/// another route: the port would still look clean while the context had already taken the
/// dependency the port exists to prevent.
#[test]
fn only_bootstrap_builds_evidence_recorder_input() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in rust_sources(&source_root) {
        let relative = relative_path(&source_root, &path);
        if relative.starts_with("bootstrap/")
            || relative.starts_with("contexts/execution_observability/")
            || relative.ends_with("_tests.rs")
        {
            continue;
        }
        if strip_comments(&fs::read_to_string(&path).expect("read native Rust source"))
            .contains("RecordEvidenceInput")
        {
            violations.push(format!(
                "[ARCH-EVIDENCE-002] {relative}: builds a recorder input outside bootstrap. \
                 Repair: publish the producer's own signal and translate it in bootstrap"
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// No command may write evidence.
///
/// The journal records what the runtime observed about its own work. A command that appended to it
/// would let a client assert what happened, and an assertion is not an observation.
#[test]
fn no_tauri_command_writes_evidence() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut violations = Vec::new();
    for path in rust_sources(&source_root.join("commands")) {
        let relative = relative_path(&source_root, &path);
        if relative.ends_with("_tests.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read command source");
        for marker in ["RecordEvidenceInput", "record_dropped_events"] {
            if source.contains(marker) {
                violations.push(format!(
                    "[ARCH-EVIDENCE-003] {relative}: reaches the evidence recorder (`{marker}`). \
                     Repair: evidence is written by in-process producers, never by a client"
                ));
            }
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// The bridge must stay bounded and non-blocking. Both properties live in one file, and both are
/// invisible from a producer's side, so a change that swapped `try_send` for `send` would pass
/// every producer test while turning an evidence write into a latency spike.
#[test]
fn the_bridge_uses_a_bounded_channel_and_a_non_blocking_send() {
    let bridge = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bootstrap/evidence_bridge.rs"),
    )
    .expect("read the evidence bridge");

    assert!(
        bridge.contains("sync_channel(EVIDENCE_QUEUE_CAPACITY)"),
        "the queue must be bounded by an explicit capacity"
    );
    assert!(
        bridge.contains("self.sender.try_send("),
        "the send must be non-blocking"
    );
    assert!(
        !bridge.contains(".sender.send("),
        "a blocking send would park the producer until the journal drains"
    );
}

/// Drops line comments so a rule cannot be tripped by the sentence that documents it. Block
/// comments are left alone: this codebase does not use them, and half-parsing Rust to find them
/// would be a worse failure than missing one.
fn strip_comments(source: &str) -> String {
    source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = fs::read_dir(root) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(rust_sources(&path));
        } else if path.extension().is_some_and(|value| value == "rs") {
            found.push(path);
        }
    }
    found
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .expect("relative source path")
        .to_string_lossy()
        .replace('\\', "/")
}
