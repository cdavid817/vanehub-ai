use std::fs;
use std::path::{Path, PathBuf};

/// Recording a decision changes a record, never the repository.
///
/// This is the guarantee the whole group rests on: a reviewer marking a hunk, clearing a mark, or
/// accepting a review must leave the Git index and the working tree exactly as they were. The
/// review surface sits next to one operation that *does* mutate — reverting a change — and the two
/// are one careless import apart.
///
/// A behavioural test can only fail for the mutation somebody thought to write a case for. This
/// reads the sources instead, so it also fails for the one nobody anticipated.
///
/// Scoped to the decision path: the workspaces revert operation legitimately runs `git apply`, and
/// a guard that swept it in would be a guard somebody has to weaken.
const DECISION_SOURCES: &[&str] = &[
    "commands/sessions/set_code_review_decision.rs",
    "commands/sessions/set_code_review_hunk_decision.rs",
    "commands/sessions/set_code_review_file_viewed.rs",
    "contexts/sessions/application/review.rs",
    "contexts/sessions/infrastructure/review_decision_repository.rs",
    "contexts/sessions/infrastructure/review_decision_schema.rs",
];

/// What touching a repository looks like in this codebase.
///
/// Matched as call and path forms rather than as words, so a comment explaining why something is
/// absent does not trip the check — a mistake this change has already made twice, and both times
/// the tempting fix was to delete the explanation.
const REPOSITORY_MUTATIONS: &[(&str, &str)] = &[
    (r#"Command::new("git""#, "runs git directly"),
    ("git_output(", "runs git through the shared helper"),
    ("revert_review_change(", "reverts a change"),
    ("render_patch(", "renders a patch to apply"),
    ("NamedTempFile", "writes a patch file"),
    ("fs::write(", "writes to the filesystem"),
    ("fs::remove_file(", "removes a file"),
];

fn native_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src")
}

fn read(relative: &str) -> String {
    fs::read_to_string(native_root().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

#[test]
fn the_patterns_match_a_path_that_really_does_mutate() {
    // Proved against the one review operation that legitimately touches the repository. Without
    // this, a typo in any pattern would leave a guard that passes because it matches nothing at
    // all — the failure a source scan hides best.
    let reverting = read("contexts/workspaces/infrastructure/session_queries.rs");
    let matched = REPOSITORY_MUTATIONS
        .iter()
        .filter(|(pattern, _)| reverting.contains(pattern))
        .count();
    assert!(
        matched >= 3,
        "the revert path should match several mutation patterns, matched {matched}"
    );
}

#[test]
fn every_decision_source_exists() {
    // A guard whose file list has drifted is a guard that scans nothing. Renames are the usual
    // cause and they are silent.
    for relative in DECISION_SOURCES {
        assert!(
            native_root().join(relative).is_file(),
            "{relative} is missing, so this guard is asserting nothing about it"
        );
    }
}

#[test]
fn no_decision_path_stages_or_rewrites_repository_content() {
    let mut offenders = Vec::new();
    for relative in DECISION_SOURCES {
        let source = read(relative);
        for (pattern, what) in REPOSITORY_MUTATIONS {
            if source.contains(pattern) {
                offenders.push(format!("{relative} {what}"));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "recording a decision must leave the index and working tree alone:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_decision_commands_do_not_reach_the_workspace_review_port() {
    // The port that can mutate. Sessions reaches workspaces for snapshots and for the hunk
    // fingerprints a witness check needs, and both are reads — but `WorkspaceReviewPort` also
    // carries the revert, so a decision command that took the whole port would be one method call
    // from staging content it has no business staging.
    for relative in DECISION_SOURCES
        .iter()
        .filter(|relative| relative.starts_with("commands/"))
    {
        let source = read(relative);
        assert!(
            !source.contains("WorkspaceApi"),
            "{relative} reaches the workspace API, which can revert"
        );
    }
}
