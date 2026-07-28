use super::session_capture::{
    capture_codex_baseline, capture_opencode_baseline, discover_codex_session,
    discover_opencode_session, ProviderSessionDiscovery,
};
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::path::{Path, PathBuf};

fn temp_directory(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("vanehub-{label}-{}", uuid::Uuid::new_v4().simple()));
    fs::create_dir_all(&path).expect("create temp directory");
    path
}

fn write_codex_rollout(root: &Path, name: &str, first_line: &str) {
    let directory = root.join("2026").join("07").join("26");
    fs::create_dir_all(&directory).expect("create rollout directory");
    fs::write(directory.join(name), format!("{first_line}\n")).expect("write rollout");
}

fn codex_meta(id: &str, cwd: &Path) -> String {
    json!({
        "type": "session_meta",
        "payload": {
            "id": id,
            "cwd": cwd,
            "source": "cli"
        }
    })
    .to_string()
}

#[test]
fn codex_capture_finds_only_a_unique_new_matching_rollout() {
    let root = temp_directory("codex-capture");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    write_codex_rollout(
        &root,
        "rollout-existing.jsonl",
        &codex_meta("existing", &project),
    );
    let baseline = capture_codex_baseline(root.clone(), project.clone()).expect("capture baseline");

    assert_eq!(
        discover_codex_session(&baseline).expect("pending discovery"),
        ProviderSessionDiscovery::Pending
    );
    write_codex_rollout(&root, "rollout-malformed.jsonl", "{not-json");
    write_codex_rollout(
        &root,
        "rollout-wrong-cwd.jsonl",
        &codex_meta("wrong", &root.join("other")),
    );
    write_codex_rollout(
        &root,
        "rollout-new.jsonl",
        &codex_meta("runtime-codex", &project),
    );

    assert_eq!(
        discover_codex_session(&baseline).expect("unique discovery"),
        ProviderSessionDiscovery::Found("runtime-codex".to_string())
    );
    fs::remove_dir_all(root).expect("remove temp directory");
}

#[test]
fn codex_capture_rejects_ambiguous_matching_rollouts() {
    let root = temp_directory("codex-ambiguous");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    let baseline = capture_codex_baseline(root.clone(), project.clone()).expect("capture baseline");
    write_codex_rollout(
        &root,
        "rollout-one.jsonl",
        &codex_meta("runtime-one", &project),
    );
    write_codex_rollout(
        &root,
        "rollout-two.jsonl",
        &codex_meta("runtime-two", &project),
    );

    assert_eq!(
        discover_codex_session(&baseline).expect("ambiguous discovery"),
        ProviderSessionDiscovery::Ambiguous(2)
    );
    fs::remove_dir_all(root).expect("remove temp directory");
}

fn create_opencode_database(path: &Path) -> Connection {
    let connection = Connection::open(path).expect("open sqlite database");
    connection
        .execute_batch(
            "CREATE TABLE session (
                id TEXT PRIMARY KEY,
                directory TEXT NOT NULL,
                time_created INTEGER NOT NULL
            );",
        )
        .expect("create session table");
    connection
}

fn insert_opencode_session(
    connection: &Connection,
    id: &str,
    directory: &Path,
    created_at_ms: i64,
) {
    connection
        .execute(
            "INSERT INTO session (id, directory, time_created) VALUES (?1, ?2, ?3)",
            rusqlite::params![id, directory.to_string_lossy(), created_at_ms],
        )
        .expect("insert session");
}

#[test]
fn opencode_capture_finds_only_a_unique_new_matching_row() {
    let root = temp_directory("opencode-capture");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    let database_path = root.join("opencode.db");
    let connection = create_opencode_database(&database_path);
    insert_opencode_session(&connection, "ses_existing", &project, 1);
    let baseline =
        capture_opencode_baseline(database_path, project.clone()).expect("capture baseline");

    assert_eq!(
        discover_opencode_session(&baseline).expect("pending discovery"),
        ProviderSessionDiscovery::Pending
    );
    insert_opencode_session(
        &connection,
        "ses_wrong",
        &root.join("other"),
        baseline.started_at_ms,
    );
    insert_opencode_session(&connection, "ses_new", &project, baseline.started_at_ms);

    assert_eq!(
        discover_opencode_session(&baseline).expect("unique discovery"),
        ProviderSessionDiscovery::Found("ses_new".to_string())
    );
    drop(connection);
    fs::remove_dir_all(root).expect("remove temp directory");
}

#[test]
fn opencode_capture_rejects_stale_and_ambiguous_rows() {
    let root = temp_directory("opencode-ambiguous");
    let project = root.join("project");
    fs::create_dir_all(&project).expect("create project");
    let database_path = root.join("opencode.db");
    let connection = create_opencode_database(&database_path);
    let baseline =
        capture_opencode_baseline(database_path, project.clone()).expect("capture baseline");
    insert_opencode_session(
        &connection,
        "ses_stale",
        &project,
        baseline.started_at_ms - 10_000,
    );
    assert_eq!(
        discover_opencode_session(&baseline).expect("stale discovery"),
        ProviderSessionDiscovery::Pending
    );
    insert_opencode_session(&connection, "ses_one", &project, baseline.started_at_ms);
    insert_opencode_session(&connection, "ses_two", &project, baseline.started_at_ms);

    assert_eq!(
        discover_opencode_session(&baseline).expect("ambiguous discovery"),
        ProviderSessionDiscovery::Ambiguous(2)
    );
    drop(connection);
    fs::remove_dir_all(root).expect("remove temp directory");
}
