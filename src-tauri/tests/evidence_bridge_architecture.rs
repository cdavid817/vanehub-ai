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

/// The bridge reaches execution_observability only through its published api.
///
/// It is the one file that legitimately knows both vocabularies, which makes it the likeliest
/// place for a shortcut: reaching `infrastructure` for a repository or `domain` for a constructor
/// the api does not publish would work, compile, and quietly move the boundary. What the api
/// publishes is what the context is willing to support; everything else is a private detail that
/// can change without warning.
#[test]
fn the_evidence_bridge_imports_only_the_published_api() {
    let code = strip_comments(&read_bridge());

    let mut violations = Vec::new();
    for layer in ["application", "domain", "infrastructure"] {
        let path = format!("execution_observability::{layer}");
        if code.contains(&path) {
            violations.push(format!(
                "[ARCH-EVIDENCE-004] bootstrap/evidence_bridge.rs: reaches `{path}`. Repair: \
                 publish what it needs through execution_observability::api"
            ));
        }
    }
    assert!(
        code.contains("execution_observability::api"),
        "the bridge must reach the context through its published api"
    );
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// No production path can build a producer that records nothing.
///
/// The publisher used to be a default a builder overrode, so an assembly that forgot the builder
/// compiled, ran, and recorded nothing — and the only symptom was a panel reporting that a session
/// did no work. It is now a constructor argument, which makes that a compile error. This rule
/// guards the other half: the no-op publisher stays out of production reach, so nobody satisfies
/// the argument by passing one.
#[test]
fn production_never_constructs_a_no_op_evidence_publisher() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let no_ops = [
        "NoAgentEvidence",
        "NoWorkspaceEvidence",
        "NoOperationsEvidence",
        "NoSessionEvidence",
    ];

    let mut violations = Vec::new();
    for path in rust_sources(&source_root) {
        let relative = relative_path(&source_root, &path);
        if relative.ends_with("_tests.rs") {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read native Rust source");
        for (index, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("//") || !no_ops.iter().any(|name| trimmed.contains(name)) {
                continue;
            }
            // A declaration, an impl, or an import is fine. A construction is what matters, and
            // only outside a test.
            if trimmed.starts_with("pub(crate) struct")
                || trimmed.starts_with("impl ")
                || trimmed.starts_with("pub(crate) use")
                || trimmed.starts_with("use ")
                || guarded_by_cfg_test(&source, index)
            {
                continue;
            }
            violations.push(format!(
                "[ARCH-EVIDENCE-005] {relative}:{}: constructs a no-op evidence publisher outside \
                 a test. Repair: pass the real publisher, or move this behind cfg(test)",
                index + 1
            ));
        }
    }
    assert!(violations.is_empty(), "{}", violations.join("\n"));
}

/// Every producer's production constructor takes its publisher.
///
/// A default can be silently wrong; an argument cannot. The rule reads the signature rather than
/// the call sites, because a re-introduced default would make every call site look correct again.
#[test]
fn every_producer_constructor_requires_an_evidence_publisher() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/contexts");
    let constructors = [
        (
            "workspaces/application/shell_service.rs",
            "evidence: Arc<dyn WorkspaceEvidencePort>",
        ),
        (
            "agent_runtime/application/service.rs",
            "evidence: Arc<dyn super::AgentEvidencePort>",
        ),
        (
            "operations/application/run_service.rs",
            "evidence: Arc<dyn super::OperationsEvidencePort>",
        ),
        (
            "sessions/application/service.rs",
            "evidence: Arc<dyn super::SessionEvidencePort>",
        ),
        (
            "sessions/application/review.rs",
            "evidence: Arc<dyn super::SessionEvidencePort>",
        ),
    ];

    for (relative, parameter) in constructors {
        let source = fs::read_to_string(source_root.join(relative))
            .unwrap_or_else(|_| panic!("this producer has an application service: {relative}"));
        let code = strip_comments(&source);
        assert!(
            code.contains(parameter),
            "[ARCH-EVIDENCE-006] {relative}: the production constructor no longer takes \
             `{parameter}`. Repair: keep the publisher a required argument, so an assembly that \
             forgets it fails to compile rather than recording nothing"
        );
        // The escape hatch is allowed, but only behind cfg(test).
        assert!(
            !code.contains("new_for_test_without_evidence") || source.contains("#[cfg(test)]"),
            "[ARCH-EVIDENCE-006] {relative}: the evidence-free constructor is reachable from \
             production"
        );
    }
}

/// Whether the line sits inside an item marked `#[cfg(test)]`.
///
/// Walks the file tracking brace depth. When a `#[cfg(test)]` appears, the item that follows it is
/// a test region until its braces return to the depth the attribute sat at; every line inside
/// counts. Doc comments and blank lines between the attribute and the item are skipped, and the
/// attribute is recognised at any depth because it marks free functions and `impl` methods alike.
///
/// Braces inside string literals would fool this, which no evidence source has and which would
/// only ever make the rule stricter.
fn cfg_test_regions(source: &str) -> Vec<std::ops::Range<usize>> {
    let mut regions = Vec::new();
    let mut depth = 0usize;
    let mut pending: Option<usize> = None;
    let mut region_start: Option<(usize, usize)> = None;

    for (index, line) in source.lines().enumerate() {
        // Any depth: the attribute sits on a free function at the top level and on a method
        // inside an `impl`, and both need to count.
        if line.trim_start().starts_with("#[cfg(test)]") && region_start.is_none() {
            pending = Some(index);
        }
        let opens = line.matches('{').count();
        let closes = line.matches('}').count();
        if let Some(start) = pending {
            if opens > 0 {
                region_start = Some((start, depth));
                pending = None;
            }
        }
        depth = depth.saturating_add(opens).saturating_sub(closes);
        if let Some((start, base)) = region_start {
            if depth <= base {
                regions.push(start..index + 1);
                region_start = None;
            }
        }
    }
    if let Some((start, _)) = region_start {
        regions.push(start..source.lines().count());
    }
    regions
}

fn guarded_by_cfg_test(source: &str, line_index: usize) -> bool {
    cfg_test_regions(source)
        .into_iter()
        .any(|region| region.contains(&line_index))
}

/// Publishing runs on the producer's thread, so it must do no storage work there.
///
/// A `try_publish` that touched SQLite would put a database write inside whatever operation was
/// being observed, which is exactly the latency the queue exists to avoid. The cost is invisible
/// at runtime until the store is slow, so the rule reads the file instead.
#[test]
fn the_producer_facing_half_of_the_bridge_never_touches_storage() {
    let code = strip_comments(&read_bridge());
    let sender_half = code
        .split("impl EvidenceBridge {")
        .nth(1)
        .and_then(|rest| rest.split("\n}\n").next())
        .expect("the sender half is an impl block");

    for forbidden in ["evidence.record", "onnection", "recv(", ".join("] {
        assert!(
            !sender_half.contains(forbidden),
            "the producer-facing half reaches `{forbidden}`; it may only map and try_send"
        );
    }
    // Every recorder call is on the worker side. Counting call sites would break as soon as the
    // worker grew a second one, so the check is which half each call lives in: the sender half is
    // the producer's thread, and a recorder call there is a database write inside their operation.
    let worker_side = code
        .split("impl EvidenceBridge {")
        .next()
        .map(str::to_string)
        .unwrap_or_default()
        + code
            .split("\n}\n")
            .skip_while(|part| !part.contains("fn run_worker("))
            .collect::<Vec<_>>()
            .join("\n}\n")
            .as_str();
    let recorder_calls = code.matches("evidence.record(").count();
    assert!(recorder_calls >= 1, "the worker must call the recorder");
    assert_eq!(
        worker_side.matches("evidence.record(").count(),
        recorder_calls,
        "a recorder call escaped the worker side"
    );
}

/// Evidence follows the owning state change, never precedes it.
///
/// In `tool_lifecycle` the owning change is `append_tool_use`, which is fallible: publishing before
/// it would leave an observation of a tool call whose record never persisted, and a reader cannot
/// tell that from a call that really happened. The order is a property of two adjacent statements,
/// so nothing at runtime would notice if they swapped.
#[test]
fn tool_evidence_is_published_after_the_tool_use_is_appended() {
    let service = fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/contexts/agent_runtime/application/service.rs"),
    )
    .expect("read the agent runtime service");
    let code = strip_comments(&service);
    let body = code
        .split("fn tool_lifecycle(")
        .nth(1)
        .and_then(|rest| rest.split("\n    fn ").next())
        .expect("tool_lifecycle is a method on the event handler");

    let append = body
        .find("append_tool_use(")
        .expect("tool_lifecycle appends the tool use");
    let publish = body
        .find("publish_tool_evidence(")
        .expect("tool_lifecycle publishes evidence");
    assert!(
        append < publish,
        "[ARCH-EVIDENCE-006] evidence is published before append_tool_use commits. Repair: \
         publish after the owning state change, so a failed append records nothing"
    );
}

fn read_bridge() -> String {
    fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("src/bootstrap/evidence_bridge.rs"),
    )
    .expect("read the evidence bridge")
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
