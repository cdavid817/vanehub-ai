use super::{
    SkillConfigCleanupState, SkillConfigurationSave, SkillConfigurationWrite,
    SqliteSkillConfigurationRepository, StoredSkillConfiguration,
};
use crate::contexts::tooling::skills::domain::{
    SkillConfigDrift, SkillConfigRevision, SkillConfigScope, SkillConfigValue,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

fn repository() -> (TempDirectory, SqliteSkillConfigurationRepository) {
    let directory = TempDirectory::new("skill-configuration-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("open test database");
    (directory, SqliteSkillConfigurationRepository::new(database))
}

fn text(value: &str) -> SkillConfigValue {
    SkillConfigValue::Text(value.to_string())
}

fn save_request(
    expected: Option<SkillConfigRevision>,
    values: Vec<(String, SkillConfigValue)>,
) -> SkillConfigurationSave {
    SkillConfigurationSave {
        skill_id: "configured-skill".to_string(),
        scope: SkillConfigScope::User,
        workspace_identity: String::new(),
        schema_hash: "hash-1".to_string(),
        base_revision: "rev-1".to_string(),
        expected_revision: expected,
        values,
        secret_keys: Vec::new(),
        validation_state: SkillConfigDrift::Compatible,
    }
}

fn saved(write: SkillConfigurationWrite) -> StoredSkillConfiguration {
    match write {
        SkillConfigurationWrite::Saved(record) => record,
        SkillConfigurationWrite::Stale(current) => {
            panic!("expected a save, got stale with {current:?}")
        }
    }
}

#[test]
fn a_first_save_expects_no_record_and_lands_at_revision_one() {
    let (_directory, repository) = repository();

    let record = saved(
        repository
            .save(&save_request(
                None,
                vec![("endpoint".to_string(), text("a"))],
            ))
            .expect("save"),
    );

    assert_eq!(record.stored_revision, SkillConfigRevision::new(1));
    assert_eq!(record.values, vec![("endpoint".to_string(), text("a"))]);
    assert_eq!(record.cleanup_state, SkillConfigCleanupState::None);
    assert_eq!(record.orphaned_at, None);
}

#[test]
fn a_save_that_expects_no_record_when_one_exists_is_stale() {
    let (_directory, repository) = repository();
    saved(
        repository
            .save(&save_request(None, Vec::new()))
            .expect("first"),
    );

    let write = repository
        .save(&save_request(
            None,
            vec![("endpoint".to_string(), text("b"))],
        ))
        .expect("second");

    match write {
        SkillConfigurationWrite::Stale(Some(current)) => {
            assert_eq!(current.stored_revision, SkillConfigRevision::new(1));
            assert!(current.values.is_empty(), "the prior record was modified");
        }
        other => panic!("expected stale, got {other:?}"),
    }
}

#[test]
fn a_stale_revision_is_rejected_and_leaves_the_prior_record_complete() {
    let (_directory, repository) = repository();
    let first = saved(
        repository
            .save(&save_request(
                None,
                vec![
                    ("endpoint".to_string(), text("original")),
                    ("label".to_string(), text("kept")),
                ],
            ))
            .expect("first"),
    );
    let second = saved(
        repository
            .save(&save_request(
                Some(first.stored_revision),
                vec![("endpoint".to_string(), text("updated"))],
            ))
            .expect("second"),
    );

    // A writer still holding revision 1 tries to write over revision 2.
    let write = repository
        .save(&save_request(
            Some(first.stored_revision),
            vec![("endpoint".to_string(), text("clobbered"))],
        ))
        .expect("third");

    match write {
        SkillConfigurationWrite::Stale(Some(current)) => {
            assert_eq!(current.stored_revision, second.stored_revision);
            assert_eq!(
                current.values,
                vec![("endpoint".to_string(), text("updated"))]
            );
        }
        other => panic!("expected stale, got {other:?}"),
    }
    let reloaded = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record");
    assert_eq!(reloaded, second);
}

#[test]
fn every_supported_value_type_round_trips_without_losing_its_type() {
    let (_directory, repository) = repository();

    let record = saved(
        repository
            .save(&save_request(
                None,
                vec![
                    ("text".to_string(), text("value")),
                    ("count".to_string(), SkillConfigValue::Integer(3)),
                    ("ratio".to_string(), SkillConfigValue::Number(3.0)),
                    ("flag".to_string(), SkillConfigValue::Boolean(false)),
                    (
                        "tags".to_string(),
                        SkillConfigValue::List(vec![text("a"), text("b")]),
                    ),
                ],
            ))
            .expect("save"),
    );

    let by_key = |key: &str| {
        record
            .values
            .iter()
            .find(|(stored, _)| stored == key)
            .expect("value")
            .1
            .clone()
    };
    // 3 and 3.0 are one JSON number, so an untagged encoding would collapse these two.
    assert_eq!(by_key("count"), SkillConfigValue::Integer(3));
    assert_eq!(by_key("ratio"), SkillConfigValue::Number(3.0));
    assert_eq!(by_key("flag"), SkillConfigValue::Boolean(false));
    assert_eq!(
        by_key("tags"),
        SkillConfigValue::List(vec![text("a"), text("b")])
    );
}

#[test]
fn resetting_a_property_keeps_the_rest_of_the_scope() {
    let (_directory, repository) = repository();
    let first = saved(
        repository
            .save(&save_request(
                None,
                vec![
                    ("endpoint".to_string(), text("kept")),
                    ("label".to_string(), text("removed")),
                ],
            ))
            .expect("save"),
    );

    let record = saved(
        repository
            .reset_property(
                "configured-skill",
                SkillConfigScope::User,
                "",
                "label",
                first.stored_revision,
            )
            .expect("reset property"),
    );

    assert_eq!(record.values, vec![("endpoint".to_string(), text("kept"))]);
    assert_eq!(record.stored_revision, SkillConfigRevision::new(2));
}

#[test]
fn resetting_a_scope_removes_the_record_rather_than_emptying_it() {
    let (_directory, repository) = repository();
    saved(
        repository
            .save(&save_request(
                None,
                vec![("endpoint".to_string(), text("a"))],
            ))
            .expect("save"),
    );

    assert!(repository
        .reset_scope("configured-skill", SkillConfigScope::User, "")
        .expect("reset scope"));

    // Absence, not an empty record: the effective value has to fall through to the next scope.
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
    assert!(!repository
        .reset_scope("configured-skill", SkillConfigScope::User, "")
        .expect("second reset"));
}

#[test]
fn user_and_project_scopes_are_stored_and_loaded_independently() {
    let (_directory, repository) = repository();
    saved(
        repository
            .save(&save_request(
                None,
                vec![("endpoint".to_string(), text("user"))],
            ))
            .expect("user save"),
    );
    saved(
        repository
            .save(&SkillConfigurationSave {
                scope: SkillConfigScope::Project,
                workspace_identity: "/workspace/one".to_string(),
                values: vec![("endpoint".to_string(), text("project"))],
                ..save_request(None, Vec::new())
            })
            .expect("project save"),
    );

    let both = repository
        .load_all_scopes("configured-skill", "/workspace/one")
        .expect("load all");

    assert_eq!(both.len(), 2);
    assert_eq!(both[0].scope, SkillConfigScope::User);
    assert_eq!(both[0].values, vec![("endpoint".to_string(), text("user"))]);
    assert_eq!(both[1].scope, SkillConfigScope::Project);
    assert_eq!(
        both[1].values,
        vec![("endpoint".to_string(), text("project"))]
    );

    // A different workspace sees only the User record.
    let other = repository
        .load_all_scopes("configured-skill", "/workspace/two")
        .expect("load other workspace");
    assert_eq!(other.len(), 1);
    assert_eq!(other[0].scope, SkillConfigScope::User);
}

#[test]
fn orphaned_records_are_retained_and_only_explicit_cleanup_removes_them() {
    let (_directory, repository) = repository();
    saved(
        repository
            .save(&save_request(
                None,
                vec![("endpoint".to_string(), text("a"))],
            ))
            .expect("save"),
    );

    assert_eq!(
        repository
            .mark_orphaned("configured-skill")
            .expect("orphan"),
        1
    );
    let orphaned = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record still present");
    assert!(orphaned.orphaned_at.is_some());
    assert_eq!(orphaned.values, vec![("endpoint".to_string(), text("a"))]);

    // Marking again does not overwrite the original orphan timestamp.
    assert_eq!(
        repository
            .mark_orphaned("configured-skill")
            .expect("orphan twice"),
        0
    );

    repository
        .set_cleanup_state("configured-skill", SkillConfigCleanupState::Failed)
        .expect("cleanup state");
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load")
            .expect("record")
            .cleanup_state,
        SkillConfigCleanupState::Failed
    );

    assert_eq!(repository.purge("configured-skill").expect("purge"), 1);
    assert_eq!(
        repository
            .load("configured-skill", SkillConfigScope::User, "")
            .expect("load"),
        None
    );
}

#[test]
fn saving_again_clears_orphan_and_cleanup_state() {
    let (_directory, repository) = repository();
    let first = saved(
        repository
            .save(&save_request(None, Vec::new()))
            .expect("save"),
    );
    repository
        .mark_orphaned("configured-skill")
        .expect("orphan");
    repository
        .set_cleanup_state("configured-skill", SkillConfigCleanupState::Pending)
        .expect("cleanup state");

    let record = saved(
        repository
            .save(&save_request(
                Some(first.stored_revision),
                vec![("endpoint".to_string(), text("returned"))],
            ))
            .expect("save after return"),
    );

    assert_eq!(record.orphaned_at, None);
    assert_eq!(record.cleanup_state, SkillConfigCleanupState::None);
}

#[test]
fn a_corrupt_value_document_reads_as_no_values_while_the_record_survives() {
    let (directory, repository) = repository();
    let first = saved(
        repository
            .save(&save_request(
                None,
                vec![("endpoint".to_string(), text("a"))],
            ))
            .expect("save"),
    );

    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen database");
    database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE skill_configuration_records SET values_json = 'not json'",
            [],
        )
        .expect("corrupt the document");

    let record = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record");

    // The revision witness survives, so a save can still replace the record rather than the
    // Skill becoming unrecoverable.
    assert!(record.values.is_empty());
    assert_eq!(record.stored_revision, first.stored_revision);
    saved(
        repository
            .save(&save_request(
                Some(record.stored_revision),
                vec![("endpoint".to_string(), text("repaired"))],
            ))
            .expect("repair"),
    );
}

#[test]
fn a_database_created_before_this_feature_gains_the_records_on_open() {
    let (_directory, repository) = repository();

    // Opening runs every migration, so the table exists on a database that predates it and a
    // save against a previously unknown Skill works with no additional setup.
    let record = saved(
        repository
            .save(&SkillConfigurationSave {
                skill_id: "never-seen-skill".to_string(),
                ..save_request(None, vec![("endpoint".to_string(), text("a"))])
            })
            .expect("save"),
    );

    assert_eq!(record.skill_id, "never-seen-skill");
    assert_eq!(record.stored_revision, SkillConfigRevision::new(1));
}

#[test]
fn racing_writers_produce_one_winner_and_never_a_blended_record() {
    let (_directory, repository) = repository();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(8));

    let attempts = (0..8)
        .map(|index| {
            let repository = repository.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                repository.save(&save_request(
                    None,
                    vec![("endpoint".to_string(), text(&format!("writer-{index}")))],
                ))
            })
        })
        .collect::<Vec<_>>()
        .into_iter()
        .map(|handle| handle.join().expect("writer thread"))
        .collect::<Vec<_>>();

    let winners = attempts
        .iter()
        .filter(|attempt| {
            matches!(
                attempt.as_ref().expect("save result"),
                SkillConfigurationWrite::Saved(_)
            )
        })
        .count();
    assert_eq!(winners, 1, "expected exactly one writer to win the race");

    let record = repository
        .load("configured-skill", SkillConfigScope::User, "")
        .expect("load")
        .expect("record");
    assert_eq!(record.stored_revision, SkillConfigRevision::new(1));
    // Exactly one writer's payload, not a mix of several.
    assert_eq!(record.values.len(), 1);
    let stored = match &record.values[0].1 {
        SkillConfigValue::Text(value) => value.clone(),
        other => panic!("unexpected stored value {other:?}"),
    };
    assert!(
        (0..8).any(|index| stored == format!("writer-{index}")),
        "stored value came from no single writer: {stored}"
    );
}
