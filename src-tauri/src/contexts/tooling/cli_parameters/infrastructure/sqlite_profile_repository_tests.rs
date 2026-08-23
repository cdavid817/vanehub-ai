use super::SqliteCliParameterProfileRepository;
use crate::contexts::tooling::cli_parameters::application::error::CliParameterApplicationError;
use crate::contexts::tooling::cli_parameters::application::models::ReplaceCliParameterProfile;
use crate::contexts::tooling::cli_parameters::application::ports::CliParameterProfileRepository;
use crate::contexts::tooling::cli_parameters::domain::selection::{
    CliParameterSelection, CliParameterSelectionMap,
};
use crate::platform::database::NativeDatabase;
use rusqlite::params;
use tempfile::TempDir;

const CATALOG_VERSION: &str = "2.0.0";

fn repository() -> (TempDir, SqliteCliParameterProfileRepository) {
    let directory = tempfile::tempdir().expect("tempdir");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    (
        directory,
        SqliteCliParameterProfileRepository::new(database),
    )
}

fn selections(entries: &[(&str, CliParameterSelection)]) -> CliParameterSelectionMap {
    entries
        .iter()
        .map(|(id, selection)| ((*id).to_string(), selection.clone()))
        .collect()
}

fn replace(
    agent_id: &str,
    expected_revision: i64,
    entries: &[(&str, CliParameterSelection)],
) -> ReplaceCliParameterProfile {
    ReplaceCliParameterProfile {
        agent_id: agent_id.to_string(),
        expected_revision,
        catalog_version: CATALOG_VERSION.to_string(),
        selections: selections(entries),
    }
}

#[test]
fn a_fresh_profile_starts_at_revision_zero_with_no_rows() {
    let (_directory, repository) = repository();
    let profile = repository.load("claude-code").expect("load");
    assert_eq!(profile.revision, 0);
    assert_eq!(profile.selection_schema_version, 1);
    assert!(profile.rows.is_empty());
    assert!(profile.updated_at.is_none());
}

#[test]
fn a_save_increments_the_revision_exactly_once_and_omits_inherited_rows() {
    let (_directory, repository) = repository();
    let persisted = repository
        .replace_if_revision(replace(
            "claude-code",
            0,
            &[
                ("model", CliParameterSelection::text("sonnet")),
                ("safeMode", CliParameterSelection::Inherit),
            ],
        ))
        .expect("save");
    assert_eq!(persisted.revision, 1);

    let profile = repository.load("claude-code").expect("load");
    assert_eq!(profile.revision, 1);
    assert_eq!(profile.selection_schema_version, 2);
    assert_eq!(profile.catalog_version, CATALOG_VERSION);
    assert_eq!(profile.rows.len(), 1);
    assert_eq!(profile.rows[0].parameter_id, "model");
    assert_eq!(
        profile.rows[0].value_json,
        r#"{"state":"value","value":"sonnet"}"#
    );
}

#[test]
fn a_stale_revision_is_rejected_and_leaves_the_committed_profile_intact() {
    let (_directory, repository) = repository();
    repository
        .replace_if_revision(replace(
            "codex-cli",
            0,
            &[("search", CliParameterSelection::boolean(true))],
        ))
        .expect("first save");

    let error = repository
        .replace_if_revision(replace(
            "codex-cli",
            0,
            &[("oss", CliParameterSelection::boolean(true))],
        ))
        .expect_err("stale save must be rejected");
    assert!(matches!(
        error,
        CliParameterApplicationError::RevisionConflict {
            expected_revision: 0,
            actual_revision: 1,
            ..
        }
    ));

    let profile = repository.load("codex-cli").expect("load");
    assert_eq!(profile.revision, 1);
    assert_eq!(profile.rows.len(), 1);
    assert_eq!(profile.rows[0].parameter_id, "search");
}

#[test]
fn reset_clears_the_rows_and_increments_the_revision_once() {
    let (_directory, repository) = repository();
    repository
        .replace_if_revision(replace(
            "gemini-cli",
            0,
            &[("debug", CliParameterSelection::boolean(true))],
        ))
        .expect("save");
    let persisted = repository
        .reset_if_revision("gemini-cli", 1, CATALOG_VERSION)
        .expect("reset");
    assert_eq!(persisted.revision, 2);

    let profile = repository.load("gemini-cli").expect("load");
    assert!(profile.rows.is_empty());
    assert_eq!(profile.revision, 2);

    assert!(repository
        .reset_if_revision("gemini-cli", 1, CATALOG_VERSION)
        .is_err());
}

#[test]
fn profiles_stay_isolated_across_every_managed_agent() {
    let (_directory, repository) = repository();
    for (index, agent_id) in [
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "opencode",
        "antigravity-cli",
    ]
    .into_iter()
    .enumerate()
    {
        repository
            .replace_if_revision(replace(
                agent_id,
                0,
                &[(
                    "model",
                    CliParameterSelection::text(format!("model-{index}")),
                )],
            ))
            .expect("save");
    }
    for (index, agent_id) in [
        "claude-code",
        "codex-cli",
        "gemini-cli",
        "opencode",
        "antigravity-cli",
    ]
    .into_iter()
    .enumerate()
    {
        let profile = repository.load(agent_id).expect("load");
        assert_eq!(profile.revision, 1);
        assert_eq!(profile.rows.len(), 1);
        assert!(profile.rows[0]
            .value_json
            .contains(&format!("model-{index}")));
    }
}

#[test]
fn a_saved_profile_survives_a_restart() {
    let directory = tempfile::tempdir().expect("tempdir");
    {
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        let repository = SqliteCliParameterProfileRepository::new(database);
        repository
            .replace_if_revision(replace(
                "opencode",
                0,
                &[("model", CliParameterSelection::text("anthropic/claude"))],
            ))
            .expect("save");
    }
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let repository = SqliteCliParameterProfileRepository::new(database);
    let profile = repository.load("opencode").expect("load");
    assert_eq!(profile.revision, 1);
    assert_eq!(profile.selection_schema_version, 2);
    assert!(profile.rows[0].value_json.contains("anthropic/claude"));
}

#[test]
fn legacy_rows_and_malformed_json_survive_the_migration_untouched() {
    let (_directory, repository) = repository();
    {
        let database_connection = repository.raw_connection_for_tests();
        database_connection
            .execute(
                "INSERT INTO cli_parameter_settings (agent_id, parameter_id, enabled, value_json, updated_at)
                 VALUES ('claude-code', 'model', 1, '\"sonnet\"', '2026-01-01T00:00:00Z'),
                        ('claude-code', 'broken', 1, 'not-json', '2026-01-01T00:00:00Z')",
                params![],
            )
            .expect("legacy rows");
    }
    let profile = repository.load("claude-code").expect("load");
    assert_eq!(profile.rows.len(), 2);
    assert!(profile
        .rows
        .iter()
        .any(|row| row.parameter_id == "broken" && row.value_json == "not-json"));
    // The metadata backfill inserted by migration 81 keeps the profile at the legacy schema
    // version until the first successful save rewrites it.
    assert_eq!(profile.selection_schema_version, 1);
    assert_eq!(profile.revision, 0);
}

#[test]
fn applying_the_schema_again_is_a_no_op() {
    let (_directory, repository) = repository();
    let connection = repository.raw_connection_for_tests();
    super::apply_schema(&connection).expect("first reapply");
    super::apply_schema(&connection).expect("second reapply");
    let count: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'cli_parameter_profiles'",
            [],
            |row| row.get(0),
        )
        .expect("query");
    assert_eq!(count, 1);
}
