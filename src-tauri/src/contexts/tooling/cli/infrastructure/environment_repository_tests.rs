// Included through `#[path]` from environment_repository.rs.
//
// Every test builds its own database in its own `TempDir`. Nothing here opens, copies, or writes
// the shared `%APPDATA%\ai.vanehub.app\vanehub.sqlite` -- all worktrees share that one file, and a
// test that touched it would corrupt whatever another session was doing.
use super::*;

use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::bulk::CliBulkActionItem;
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogStatus, CliCatalogUnavailableReason,
};
use crate::contexts::tooling::cli::domain::ids::CliInstallationId;
use crate::contexts::tooling::cli::domain::installation::{CliEnvironmentOrigin, CliInstallation};
use crate::contexts::tooling::cli::domain::plan::{CliCommandPreview, CliFallbackPolicy};
use crate::contexts::tooling::cli::domain::source::{CliSourceConfidence, CliSourceKind};
use crate::contexts::tooling::cli::domain::status::{
    CliDiscoveryStatus, CliExecutableStatus, CliFreshness, CliUpdateStatus,
};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

fn stamp(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).expect("timestamp")
}

fn tool() -> CliToolId {
    CliToolId::new("claude-code").expect("tool id")
}

fn npm() -> CliSourceId {
    CliSourceId::new("npm").expect("source id")
}

/// A repository over a database in its own temporary directory.
///
/// `NativeDatabase::new` runs the full migration set, so these tests exercise the real 81-83
/// migrations rather than a hand-applied schema. The `TempDir` is returned alongside the
/// repository because dropping it deletes the directory out from under the open pool.
fn repository() -> (SqliteCliEnvironmentRepository, tempfile::TempDir) {
    let directory = tempfile::tempdir().expect("temp directory");
    let database =
        NativeDatabase::new(directory.path().to_path_buf()).expect("database in a temp directory");
    (SqliteCliEnvironmentRepository::new(database), directory)
}

fn snapshot_for(agent_id: &str) -> CliEnvironmentSnapshot {
    let mut snapshot = CliEnvironmentSnapshot::never_scanned(
        CliToolId::new(agent_id).expect("tool id"),
        "fingerprint-a".to_string(),
    );
    snapshot.installations = vec![CliInstallation {
        id: CliInstallationId::new("a").expect("installation id"),
        executable_path: "/path/claude".to_string(),
        canonical_path: None,
        alias_paths: Vec::new(),
        target_missing: false,
        reported_version: Some(NormalizedCliVersion::parse("1.2.0")),
        source_id: Some(npm()),
        source_kind: CliSourceKind::Npm,
        source_confidence: CliSourceConfidence::Inferred,
        path_priority: Some(0),
        environment_origin: CliEnvironmentOrigin::Path,
        executable_status: CliExecutableStatus::Healthy,
    }];
    snapshot.discovery = CliDiscoveryStatus::FoundOne;
    snapshot.checked_at = Some(stamp(1_000));
    snapshot.recompute_derived(false, false)
}

fn plan_for(plan_id: &str, state: CliActionPlanState) -> CliActionPlan {
    let created_at = stamp(1_000);
    CliActionPlan {
        id: CliActionPlanId::new(plan_id).expect("plan id"),
        revision: 1,
        agent_id: tool(),
        action: CliActionKind::Upgrade,
        source_id: npm(),
        installation_id: None,
        current_version: Some("1.2.0".to_string()),
        target_version: Some("1.3.0".to_string()),
        channel: Some("stable".to_string()),
        command_preview: CliCommandPreview::new("npm", vec!["install".to_string()]),
        preconditions: Vec::new(),
        warnings: Vec::new(),
        requires_elevation: false,
        requires_network: true,
        fallback_policy: CliFallbackPolicy::None,
        environment_fingerprint: "fingerprint-a".to_string(),
        state,
        created_at,
        expires_at: CliActionPlan::default_expiry(created_at),
    }
}

#[test]
fn a_snapshot_round_trips_through_storage() {
    let (repository, _directory) = repository();
    let original = snapshot_for("claude-code");

    repository.save_snapshot_atomic(&original).expect("save");
    let loaded = repository.load_snapshot(&tool()).expect("load");

    assert_eq!(loaded.as_ref(), Some(&original));
}

#[test]
fn saving_a_snapshot_twice_replaces_it_rather_than_duplicating() {
    let (repository, _directory) = repository();
    let mut snapshot = snapshot_for("claude-code");
    repository.save_snapshot_atomic(&snapshot).expect("first");

    snapshot.last_operation_id = Some("op-2".to_string());
    repository.save_snapshot_atomic(&snapshot).expect("second");

    let all = repository.list_snapshots().expect("list");
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].last_operation_id.as_deref(), Some("op-2"));
}

#[test]
fn listing_returns_only_stored_snapshots_in_a_stable_order() {
    let (repository, _directory) = repository();
    repository
        .save_snapshot_atomic(&snapshot_for("opencode"))
        .expect("save");
    repository
        .save_snapshot_atomic(&snapshot_for("claude-code"))
        .expect("save");

    let all = repository.list_snapshots().expect("list");

    assert_eq!(
        all.iter()
            .map(|snapshot| snapshot.agent_id.as_str())
            .collect::<Vec<_>>(),
        vec!["claude-code", "opencode"]
    );
}

#[test]
fn a_missing_snapshot_is_none_rather_than_an_error() {
    let (repository, _directory) = repository();
    assert_eq!(repository.load_snapshot(&tool()).expect("load"), None);
}

#[test]
fn malformed_stored_json_is_a_typed_storage_error_not_a_panic() {
    let (repository, _directory) = repository();
    repository
        .save_snapshot_atomic(&snapshot_for("claude-code"))
        .expect("save");
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE cli_environment_snapshots SET snapshot_json = '{not json'",
                [],
            )
            .expect("corrupt the row");
    }

    let error = repository.load_snapshot(&tool()).expect_err("refused");

    assert_eq!(error.category(), "storage");
    // The row is named so the failure is diagnosable without dumping a document that may hold
    // filesystem paths.
    assert!(error.to_string().contains("claude-code"));
}

#[test]
fn a_snapshot_document_from_a_newer_build_is_reported_as_storage_trouble() {
    let (repository, _directory) = repository();
    repository
        .save_snapshot_atomic(&snapshot_for("claude-code"))
        .expect("save");
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE cli_environment_snapshots
                 SET snapshot_json = json_set(snapshot_json, '$.documentVersion', 99)",
                [],
            )
            .expect("bump document version");
    }

    let error = repository.load_snapshot(&tool()).expect_err("refused");
    assert_eq!(error.category(), "storage");
}

#[test]
fn catalogs_are_keyed_by_source_so_one_cannot_overwrite_another() {
    let (repository, _directory) = repository();
    let make = |source: &str, latest: &str| CliVersionCatalog {
        agent_id: tool(),
        source_id: CliSourceId::new(source).expect("source"),
        channel: Some("stable".to_string()),
        versions: vec![NormalizedCliVersion::parse(latest)],
        latest: Some(NormalizedCliVersion::parse(latest)),
        fetched_at: stamp(1_000),
        expires_at: stamp(1_900),
        status: CliCatalogStatus::Available,
    };

    repository.save_catalog(&make("npm", "1.3.0")).expect("npm");
    repository
        .save_catalog(&make("winget", "1.1.0"))
        .expect("winget");

    let npm_catalog = repository
        .load_catalog(&tool(), &npm(), Some("stable"))
        .expect("load")
        .expect("present");
    let winget_catalog = repository
        .load_catalog(
            &tool(),
            &CliSourceId::new("winget").expect("source"),
            Some("stable"),
        )
        .expect("load")
        .expect("present");

    assert_eq!(
        npm_catalog
            .latest
            .as_ref()
            .map(NormalizedCliVersion::as_str),
        Some("1.3.0")
    );
    assert_eq!(
        winget_catalog
            .latest
            .as_ref()
            .map(NormalizedCliVersion::as_str),
        Some("1.1.0")
    );
}

#[test]
fn an_unavailable_catalog_persists_its_reason() {
    let (repository, _directory) = repository();
    let catalog = CliVersionCatalog::unavailable(
        tool(),
        npm(),
        Some("stable".to_string()),
        CliCatalogUnavailableReason::QueryFailed,
        stamp(1_000),
        stamp(1_900),
    );

    repository.save_catalog(&catalog).expect("save");
    let loaded = repository
        .load_catalog(&tool(), &npm(), Some("stable"))
        .expect("load")
        .expect("present");

    assert_eq!(loaded.status, catalog.status);
}

#[test]
fn a_plan_round_trips_and_starts_as_a_draft() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);

    repository.create_action_plan(&plan).expect("create");
    let loaded = repository
        .load_action_plan(&plan.id)
        .expect("load")
        .expect("present");

    assert_eq!(loaded, plan);
    assert_eq!(loaded.state, CliActionPlanState::Draft);
}

#[test]
fn admission_moves_the_plan_to_executing_atomically() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");

    let admitted = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_100))
        .expect("admitted");

    assert_eq!(admitted.state, CliActionPlanState::Executing);
    // The stored row moved too, not just the returned value.
    let stored = repository
        .load_action_plan(&plan.id)
        .expect("load")
        .expect("present");
    assert_eq!(stored.state, CliActionPlanState::Executing);
}

#[test]
fn a_plan_can_only_be_admitted_once() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");
    repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_100))
        .expect("first");

    let error = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_200))
        .expect_err("second is refused");

    assert_eq!(error.category(), "plan-consumed");
}

#[test]
fn admission_refuses_an_expired_stale_or_superseded_plan_without_changing_it() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");

    // Ten minutes and one second later.
    let expired = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_000 + 601))
        .expect_err("expired");
    assert_eq!(expired.category(), "plan-expired");

    let stale = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-CHANGED", stamp(1_100))
        .expect_err("stale");
    assert_eq!(stale.category(), "plan-stale");

    let superseded = repository
        .begin_action_plan_execution(&plan.id, 9, "fingerprint-a", stamp(1_100))
        .expect_err("superseded");
    assert_eq!(superseded.category(), "plan-revision-mismatch");

    // A refused admission leaves the plan available for a valid one.
    let stored = repository
        .load_action_plan(&plan.id)
        .expect("load")
        .expect("present");
    assert_eq!(stored.state, CliActionPlanState::Draft);
}

#[test]
fn admitting_a_plan_that_does_not_exist_is_reported_as_not_found() {
    let (repository, _directory) = repository();
    let missing = CliActionPlanId::new("nope").expect("plan id");

    let error = repository
        .begin_action_plan_execution(&missing, 1, "fingerprint-a", stamp(1_100))
        .expect_err("refused");

    assert_eq!(error.category(), "plan-not-found");
}

#[test]
fn finishing_a_plan_records_its_terminal_state() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");

    repository
        .finish_action_plan(&plan.id, CliActionPlanState::Completed, stamp(1_500))
        .expect("finish");

    let stored = repository
        .load_action_plan(&plan.id)
        .expect("load")
        .expect("present");
    assert_eq!(stored.state, CliActionPlanState::Completed);
}

#[test]
fn only_draft_plans_for_the_requested_tool_are_listed() {
    let (repository, _directory) = repository();
    repository
        .create_action_plan(&plan_for("draft-1", CliActionPlanState::Draft))
        .expect("create");
    let mut other_tool = plan_for("draft-2", CliActionPlanState::Draft);
    other_tool.agent_id = CliToolId::new("codex-cli").expect("tool id");
    repository.create_action_plan(&other_tool).expect("create");
    repository
        .create_action_plan(&plan_for("done-1", CliActionPlanState::Completed))
        .expect("create");

    let drafts = repository.list_draft_plans(&tool()).expect("list");

    assert_eq!(drafts.len(), 1);
    assert_eq!(drafts[0].id.as_str(), "draft-1");
}

#[test]
fn a_bulk_plan_and_its_item_plans_are_inserted_together() {
    let (repository, _directory) = repository();
    let item = plan_for("item-1", CliActionPlanState::Draft);
    let bulk = CliBulkActionPlan {
        id: CliBulkPlanId::new("bulk-1").expect("bulk id"),
        revision: 1,
        items: vec![CliBulkActionItem {
            agent_id: tool(),
            plan_id: item.id.clone(),
            source_id: npm(),
            current_version: Some("1.2.0".to_string()),
            target_version: Some("1.3.0".to_string()),
            requires_elevation: false,
            requires_network: true,
            state: CliActionPlanState::Draft,
            skipped_reason: None,
        }],
        skipped: Vec::new(),
        environment_fingerprint: "fingerprint-a".to_string(),
        created_at: stamp(1_000),
        expires_at: stamp(1_600),
    };

    repository
        .create_bulk_plan_atomic(&bulk, std::slice::from_ref(&item))
        .expect("create");

    assert_eq!(
        repository.load_bulk_plan(&bulk.id).expect("load"),
        Some(bulk)
    );
    assert!(repository
        .load_action_plan(&item.id)
        .expect("load")
        .is_some());
}

fn bulk_for(id: &str) -> CliBulkActionPlan {
    CliBulkActionPlan {
        id: CliBulkPlanId::new(id).expect("bulk id"),
        revision: 1,
        items: Vec::new(),
        skipped: Vec::new(),
        environment_fingerprint: "fingerprint-a".to_string(),
        created_at: stamp(1_000),
        expires_at: stamp(1_600),
    }
}

#[test]
fn a_bulk_insert_that_fails_leaves_no_item_plans_behind() {
    let (repository, _directory) = repository();
    let bulk = bulk_for("bulk-1");
    repository
        .create_bulk_plan_atomic(&bulk, &[])
        .expect("the first batch lands");
    let orphan = plan_for("item-orphan", CliActionPlanState::Draft);

    // The same batch id again: the bulk row violates the primary key, and it is written before the
    // item plans, so the item is inside the transaction that fails.
    let error = repository
        .create_bulk_plan_atomic(&bulk, &[orphan.clone()])
        .expect_err("refused");

    assert_eq!(error.category(), "storage");
    // Rolled back completely, so nothing is left that no bulk plan references and no expiry sweep
    // would ever reach.
    assert_eq!(repository.load_action_plan(&orphan.id).expect("load"), None);
}

#[test]
fn attaching_an_already_persisted_plan_to_a_batch_updates_it_instead_of_failing() {
    let (repository, _directory) = repository();
    // Bulk preparation runs the ordinary single-action planning path first, so by the time the
    // batch is recorded every item plan is already a row. Inserting again failed on the primary
    // key and took the whole batch down with it.
    let plan = plan_for("item-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("planned");

    repository
        .create_bulk_plan_atomic(&bulk_for("bulk-1"), &[plan.clone()])
        .expect("the batch attaches the plan it planned");

    assert_eq!(
        repository.load_action_plan(&plan.id).expect("load"),
        Some(plan.clone())
    );
    // One row, not two: the plan was updated in place rather than duplicated.
    assert_eq!(
        repository
            .list_draft_plans(&plan.agent_id)
            .expect("drafts")
            .len(),
        1
    );
}

#[test]
fn expiry_marks_only_overdue_drafts_and_is_bounded() {
    let (repository, _directory) = repository();
    for index in 0..5 {
        repository
            .create_action_plan(&plan_for(
                &format!("plan-{index}"),
                CliActionPlanState::Draft,
            ))
            .expect("create");
    }
    repository
        .create_action_plan(&plan_for("executing", CliActionPlanState::Executing))
        .expect("create");

    // Nothing is overdue yet.
    assert_eq!(
        repository
            .expire_stale_plans(stamp(1_100), 64)
            .expect("sweep"),
        0
    );

    // Bounded: only two of the five overdue drafts are touched.
    let first = repository
        .expire_stale_plans(stamp(1_000 + 601), 2)
        .expect("sweep");
    assert_eq!(first, 2);
    let rest = repository
        .expire_stale_plans(stamp(1_000 + 601), 64)
        .expect("sweep");
    assert_eq!(rest, 3);
    // Idempotent, and an executing plan is never swept out from under a running mutation.
    assert_eq!(
        repository
            .expire_stale_plans(stamp(1_000 + 601), 64)
            .expect("sweep"),
        0
    );
    let executing = repository
        .load_action_plan(&CliActionPlanId::new("executing").expect("id"))
        .expect("load")
        .expect("present");
    assert_eq!(executing.state, CliActionPlanState::Executing);
}

#[test]
fn an_expired_plan_can_no_longer_be_admitted() {
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");
    repository
        .expire_stale_plans(stamp(1_000 + 601), 64)
        .expect("sweep");

    let error = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_100))
        .expect_err("refused");

    // The sweep moved it out of draft, so admission reports it as consumed rather than running it.
    assert_eq!(error.category(), "plan-consumed");
}

#[test]
fn an_interrupted_plan_left_executing_is_not_admitted_again() {
    // A crash between admission and completion leaves the row in `executing`. Re-admitting it
    // would run the same external effect twice.
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Executing);
    repository.create_action_plan(&plan).expect("create");

    let error = repository
        .begin_action_plan_execution(&plan.id, 1, "fingerprint-a", stamp(1_100))
        .expect_err("refused");

    assert_eq!(error.category(), "plan-consumed");
}

#[test]
fn the_legacy_table_is_left_intact_alongside_the_new_ones() {
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        // The legacy table is created by the real migrations this database already ran; the row is
        // what a user upgrading into this change would be carrying.
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status (agent_id, detected_path)
                 VALUES ('claude-code', '/usr/local/bin/claude');",
            )
            .expect("legacy row");
    }

    repository
        .save_snapshot_atomic(&snapshot_for("claude-code"))
        .expect("save");

    let connection = repository.connection().expect("connection");
    let legacy: i64 = connection
        .query_row("SELECT COUNT(*) FROM cli_tool_status", [], |row| row.get(0))
        .expect("legacy row survives");
    assert_eq!(legacy, 1);
}

#[test]
fn a_legacy_row_is_read_as_a_stale_snapshot_when_no_authoritative_one_exists() {
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status
                    (agent_id, detected_path, current_version, last_checked_at)
                 VALUES
                    ('claude-code', '/usr/local/bin/claude', '1.2.0', '1970-01-01T00:16:40+00:00');",
            )
            .expect("legacy row");
    }

    let snapshot = repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect("load")
        .expect("legacy snapshot");

    assert_eq!(snapshot.freshness, CliFreshness::Stale);
    assert_eq!(snapshot.discovery, CliDiscoveryStatus::FoundOne);
    assert_eq!(snapshot.installations.len(), 1);
    assert_eq!(
        snapshot.installations[0].executable_path,
        "/usr/local/bin/claude"
    );
    // Never re-probed, so nothing about it is presented as verified.
    assert_eq!(
        snapshot.installations[0].executable_status,
        CliExecutableStatus::Unknown
    );
    assert_eq!(
        snapshot.installations[0].source_confidence,
        CliSourceConfidence::Unknown
    );
    // A fingerprint that can never match a computed one, so no mutation can be planned off it.
    assert_eq!(snapshot.environment_fingerprint, LEGACY_FINGERPRINT);
}

#[test]
fn an_authoritative_snapshot_wins_over_the_legacy_row() {
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status (agent_id, detected_path)
                 VALUES ('claude-code', '/legacy/claude');",
            )
            .expect("legacy row");
    }
    repository
        .save_snapshot_atomic(&snapshot_for("claude-code"))
        .expect("save");

    let snapshot = repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect("load")
        .expect("snapshot");

    assert_ne!(snapshot.environment_fingerprint, LEGACY_FINGERPRINT);
    assert!(
        snapshot
            .installations
            .iter()
            .all(|installation| installation.executable_path != "/legacy/claude"),
        "legacy path leaked into the authoritative snapshot"
    );
}

#[test]
fn a_legacy_row_never_becomes_the_new_write_model() {
    // Reading a legacy row must not promote it: the new tables stay empty until a real refresh
    // writes one, otherwise unverified data would be indistinguishable from probed data.
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status (agent_id, detected_path)
                 VALUES ('claude-code', '/usr/local/bin/claude');",
            )
            .expect("legacy row");
    }

    repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect("load");

    let connection = repository.connection().expect("connection");
    let stored: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM cli_environment_snapshots",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(stored, 0);
}

#[test]
fn a_malformed_legacy_timestamp_is_a_typed_error_not_a_panic() {
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status (agent_id, detected_path, last_checked_at)
                 VALUES ('claude-code', '/usr/local/bin/claude', 'not-a-timestamp');",
            )
            .expect("legacy row");
    }

    let error = repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect_err("typed error");

    assert_eq!(error.category(), "storage");
}

#[test]
fn malformed_legacy_json_cannot_reach_the_new_model() {
    // `available_versions` is the one JSON column the legacy table carries. The migration path
    // reads none of it, so a row corrupted by an older build is inert rather than fatal.
    let (repository, _directory) = repository();
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute_batch(
                "INSERT INTO cli_tool_status (agent_id, detected_path, available_versions)
                 VALUES ('claude-code', '/usr/local/bin/claude', '{not json');",
            )
            .expect("legacy row");
    }

    let snapshot = repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect("load")
        .expect("legacy snapshot");

    // The legacy row establishes a path and nothing else -- no source ownership, no update state.
    assert!(snapshot.sources.is_empty());
    assert_eq!(snapshot.update, CliUpdateStatus::Unknown);
    assert_eq!(snapshot.installations.len(), 1);
}

#[test]
fn an_empty_database_reports_no_snapshot_rather_than_failing() {
    let (repository, _directory) = repository();

    let snapshot = repository
        .load_snapshot(&CliToolId::new("claude-code").expect("tool id"))
        .expect("load");

    assert!(snapshot.is_none());
    assert!(repository.list_snapshots().expect("list").is_empty());
}

#[test]
fn the_state_column_outranks_the_state_inside_the_stored_document() {
    // The sweep and `finish_action_plan` move the column without rewriting the document. A read
    // that trusted the document would hand back a `draft` plan that has already been consumed.
    let (repository, _directory) = repository();
    let plan = plan_for("plan-1", CliActionPlanState::Draft);
    repository.create_action_plan(&plan).expect("create");
    {
        let connection = repository.connection().expect("connection");
        connection
            .execute(
                "UPDATE cli_action_plans SET state = 'cancelled' WHERE plan_id = 'plan-1'",
                [],
            )
            .expect("column moved");
    }

    let loaded = repository
        .load_action_plan(&plan.id)
        .expect("load")
        .expect("plan");

    assert_eq!(loaded.state, CliActionPlanState::Cancelled);
}
