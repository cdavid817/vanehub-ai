//! Migration 90 against a real database.
//!
//! Every concurrency test opens **two independent connections** from the pool. A single connection
//! serialises by construction, so a CAS test that shares one proves nothing.

use super::{
    apply_connector_schema, SqliteConnectorBindingRepository, SqliteConnectorDefinitionRepository,
    SqliteConnectorInstanceRepository, SqliteConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::application::{
    ConnectorBindingRepository, ConnectorDefinitionRepository, ConnectorInstanceRepository,
    ConnectorSubjectRepository,
};
use crate::contexts::tooling::connectors::domain::{
    BindingId, ConnectorDefinitionDigest, ConnectorDefinitionOutcome, ConnectorDefinitionRevision,
    ConnectorGlobalId, ConnectorInstanceError, ConnectorSnapshotRef, ConnectorSubject,
    ConnectorTarget, CredentialHandle, DisplayLabel, InstanceEdit, InstanceId, OwnerExtensionId,
    PublicConfiguration, TargetKind, ABSENT_REVISION,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::{params, Connection};
use std::collections::BTreeSet;
use std::sync::Arc;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const CONNECTOR: &str = "ext::acme.mailer::smtp";
const AT: &str = "2026-08-23T00:00:00Z";

struct Fixture {
    _directory: TempDirectory,
    database: Arc<NativeDatabase>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let database = Arc::new(
        NativeDatabase::new(directory.path().to_path_buf()).expect("database should open"),
    );
    let fixture = Fixture {
        _directory: directory,
        database,
    };
    SqliteConnectorSubjectRepository::new(fixture.database.clone())
        .ensure(&subject())
        .expect("subject");
    fixture
}

fn connector() -> ConnectorGlobalId {
    ConnectorGlobalId::parse(CONNECTOR).expect("connector")
}

fn subject() -> ConnectorSubject {
    ConnectorSubject {
        connector: connector(),
        owner_extension: OwnerExtensionId::parse("acme.mailer").expect("owner"),
        first_seen_at: AT.to_string(),
    }
}

fn instances(fixture: &Fixture) -> SqliteConnectorInstanceRepository {
    SqliteConnectorInstanceRepository::new(fixture.database.clone())
}

fn bindings(fixture: &Fixture) -> SqliteConnectorBindingRepository {
    SqliteConnectorBindingRepository::new(fixture.database.clone())
}

fn definitions(fixture: &Fixture) -> SqliteConnectorDefinitionRepository {
    SqliteConnectorDefinitionRepository::new(fixture.database.clone())
}

fn revision(snapshot: &str, digest: &str) -> ConnectorDefinitionRevision {
    ConnectorDefinitionRevision {
        snapshot: ConnectorSnapshotRef::parse(snapshot).expect("snapshot"),
        connector: connector(),
        digest: ConnectorDefinitionDigest::parse(digest).expect("digest"),
        recorded_at: AT.to_string(),
    }
}

fn configuration() -> PublicConfiguration {
    PublicConfiguration::of(&[("base_url", "https://example.test")]).expect("config")
}

fn create(fixture: &Fixture, id: &str, label: &str) -> Result<(), ConnectorInstanceError> {
    instances(fixture)
        .save(&InstanceEdit {
            instance: &InstanceId::parse(id).expect("instance"),
            connector: &connector(),
            label: &DisplayLabel::parse(label).expect("label"),
            desired_enabled: true,
            configuration: &configuration(),
            expected_revision: ABSENT_REVISION,
            at: AT,
        })
        .map(|_| ())
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

#[test]
fn migration_90_creates_every_table_the_subdomain_owns() {
    let fixture = fixture("connectors-migration");
    let connection = fixture.database.connection().expect("connection");

    for table in [
        "connector_subjects",
        "connector_definition_revisions",
        "connector_instances",
        "connector_bindings",
    ] {
        let found: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(found, 1, "{table} is missing");
    }
}

#[test]
fn migration_90_is_a_no_op_on_a_database_that_already_has_it() {
    let directory = TempDirectory::new("connectors-idempotent");
    let path = directory.path().join("repeat.sqlite");
    let connection = Connection::open(&path).expect("open");
    migrate(&connection).expect("first migrate");

    apply_connector_schema(&connection).expect("re-apply");
    apply_connector_schema(&connection).expect("re-apply again");
}

#[test]
fn an_instance_row_has_nowhere_to_put_a_secret_or_a_live_connection_state() {
    // The secret-column contract, asserted against the schema rather than against a habit. A
    // column added later that could hold a secret -- or a `connected` flag that every crash would
    // leave lying -- fails here, which is the only place it would be noticed in time.
    let fixture = fixture("connectors-columns");
    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('connector_instances')")
        .expect("prepare");
    let columns: BTreeSet<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(|column| column.expect("column"))
        .collect();

    let permitted: BTreeSet<String> = [
        "instance_id",
        "connector_global_id",
        "display_label",
        "label_key",
        "desired_enabled",
        "public_configuration",
        "credential_handle",
        "revision",
        "updated_at",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect();

    assert_eq!(
        columns, permitted,
        "an instance records what was asked for and a handle, never a secret and never live state"
    );
}

#[test]
fn a_secret_shaped_configuration_key_cannot_reach_the_column() {
    // The column is TEXT and SQLite would take a token. What stops it is that the only way to
    // build a configuration is through `PublicConfiguration`.
    assert!(PublicConfiguration::of(&[("api_key", "sk-live-1234")]).is_err());
    assert!(PublicConfiguration::of(&[("token", "ghp_1234")]).is_err());
}

// ---------------------------------------------------------------------------
// Subjects and definitions
// ---------------------------------------------------------------------------

#[test]
fn re_seeding_a_subject_moves_neither_its_first_sighting_nor_its_owner() {
    // Rewriting the owner would erase which package an operator has to uninstall, and would do it
    // on every launch.
    let fixture = fixture("connectors-subject-seed");
    let subjects = SqliteConnectorSubjectRepository::new(fixture.database.clone());

    subjects
        .ensure(&ConnectorSubject {
            owner_extension: OwnerExtensionId::parse("someone.else").expect("owner"),
            first_seen_at: "2026-09-01T00:00:00Z".to_string(),
            ..subject()
        })
        .expect("re-seed");

    let held = subjects.get(&connector()).expect("get").expect("present");
    assert_eq!(held.first_seen_at, AT);
    assert_eq!(held.owner_extension.as_str(), "acme.mailer");
    assert_eq!(subjects.all().expect("all").len(), 1);
}

#[test]
fn a_definition_cannot_reference_a_subject_that_does_not_exist() {
    let fixture = fixture("connectors-definition-fk");
    let orphan = ConnectorDefinitionRevision {
        connector: ConnectorGlobalId::parse("ext::nobody.nothing::never").expect("connector"),
        ..revision("snap-a", FIRST)
    };

    assert_eq!(
        definitions(&fixture).record(&orphan).expect_err("orphan"),
        "unknown_connector_subject"
    );
}

#[test]
fn re_recording_the_same_definition_is_idempotent_and_a_different_one_is_refused() {
    let fixture = fixture("connectors-definitions");
    let recorded = revision("snap-a", FIRST);

    assert_eq!(
        definitions(&fixture).record(&recorded).expect("record"),
        ConnectorDefinitionOutcome::Recorded
    );
    assert_eq!(
        definitions(&fixture).record(&recorded).expect("re-record"),
        ConnectorDefinitionOutcome::AlreadyRecorded
    );

    let outcome = definitions(&fixture)
        .record(&revision("snap-a", SECOND))
        .expect("conflicting record");

    assert!(!outcome.admits_connect(), "{outcome:?}");
    assert_eq!(outcome.code(), "connector_definition_content_conflict");
    assert_eq!(
        definitions(&fixture)
            .recorded(
                &connector(),
                &ConnectorSnapshotRef::parse("snap-a").expect("snapshot")
            )
            .expect("recorded")
            .map(|revision| revision.digest),
        Some(ConnectorDefinitionDigest::parse(FIRST).expect("digest")),
        "a rebuild cannot change what an already-installed snapshot means"
    );
}

#[test]
fn two_snapshots_each_hold_their_own_revision_of_one_subject() {
    let fixture = fixture("connectors-two-snapshots");
    definitions(&fixture)
        .record(&revision("snap-a", FIRST))
        .expect("first");
    definitions(&fixture)
        .record(&revision("snap-b", SECOND))
        .expect("second");

    assert_eq!(
        definitions(&fixture).revisions(&connector()).expect("all").len(),
        2,
        "an upgrade records beside the old revision so a rollback still has something to connect with"
    );
}

#[test]
fn evidence_is_not_removed_by_deleting_what_points_at_it() {
    // RESTRICT everywhere. Deleting a subject that still has an instance must fail and force
    // whoever is doing it to say what happens to the credential attached to it.
    let fixture = fixture("connectors-restrict");
    definitions(&fixture)
        .record(&revision("snap-a", FIRST))
        .expect("record");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &InstanceId::parse("instance-1").expect("instance"),
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");

    let connection = fixture.database.connection().expect("connection");
    for statement in [
        "DELETE FROM connector_subjects WHERE connector_global_id = ?1",
        "DELETE FROM connector_instances WHERE connector_global_id = ?1",
    ] {
        let error = connection
            .execute(statement, params![CONNECTOR])
            .expect_err("the reference must hold");
        assert!(
            error.to_string().contains("FOREIGN KEY"),
            "expected a foreign-key refusal, got {error}"
        );
    }
}

// ---------------------------------------------------------------------------
// Instances
// ---------------------------------------------------------------------------

#[test]
fn a_label_that_normalises_onto_an_existing_one_is_refused() {
    // `Acme Prod` and `acme  prod` in one list is how a credential gets attached to the wrong
    // instance.
    let fixture = fixture("connectors-label-collision");
    create(&fixture, "instance-1", "Acme Prod").expect("create");

    let error = create(&fixture, "instance-2", "acme   PROD").expect_err("collision");

    assert_eq!(error.code(), "duplicate_connector_label");
    let ConnectorInstanceError::DuplicateLabel { existing } = error else {
        panic!("expected a duplicate label");
    };
    assert_eq!(
        existing.as_str(),
        "instance-1",
        "an operator told the name is taken needs to know by what"
    );
}

#[test]
fn the_label_is_stored_as_typed_and_the_key_is_derived() {
    let fixture = fixture("connectors-label-storage");
    create(&fixture, "instance-1", "Acme  Prod").expect("create");

    let (label, key): (String, String) = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT display_label, label_key FROM connector_instances WHERE instance_id = ?1",
            params!["instance-1"],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("read");

    assert_eq!(label, "Acme  Prod", "the user's own words survive");
    assert_eq!(key, "acme prod");
}

#[test]
fn renaming_an_instance_keeps_its_binding_and_its_credential() {
    // Identity is `instance_id`, not the label. If it were the label, a rename would orphan every
    // binding and strand the secret in the credential store.
    let fixture = fixture("connectors-rename");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");
    let handle = CredentialHandle::parse("cred-1").expect("handle");
    instances(&fixture)
        .attach_credential(&instance, Some(&handle), 1, AT)
        .expect("attach");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");

    let renamed = instances(&fixture)
        .save(&InstanceEdit {
            instance: &instance,
            connector: &connector(),
            label: &DisplayLabel::parse("Acme Production").expect("label"),
            desired_enabled: true,
            configuration: &configuration(),
            expected_revision: 2,
            at: AT,
        })
        .expect("rename");

    assert_eq!(renamed.display_label.as_str(), "Acme Production");
    assert_eq!(
        renamed.credential,
        Some(handle),
        "an ordinary settings edit does not detach a credential by omitting it"
    );
    assert_eq!(
        bindings(&fixture)
            .bindings(&instance)
            .expect("bindings")
            .len(),
        1
    );
}

#[test]
fn an_instance_edit_from_a_stale_revision_is_refused() {
    let fixture = fixture("connectors-instance-stale");
    create(&fixture, "instance-1", "Acme Prod").expect("create");

    let error = create(&fixture, "instance-1", "Acme Prod").expect_err("stale");

    assert_eq!(error.code(), "connector_instance_stale_revision");
}

#[test]
fn an_instance_cannot_reference_a_subject_that_does_not_exist() {
    let fixture = fixture("connectors-instance-fk");

    let error = instances(&fixture)
        .save(&InstanceEdit {
            instance: &InstanceId::parse("instance-1").expect("instance"),
            connector: &ConnectorGlobalId::parse("ext::nobody.nothing::never").expect("connector"),
            label: &DisplayLabel::parse("Nowhere").expect("label"),
            desired_enabled: true,
            configuration: &configuration(),
            expected_revision: ABSENT_REVISION,
            at: AT,
        })
        .expect_err("no such subject");

    assert_eq!(error, ConnectorInstanceError::UnknownSubject);
}

#[test]
fn a_credential_handle_round_trips_and_can_be_detached_deliberately() {
    let fixture = fixture("connectors-credential");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");
    let handle = CredentialHandle::parse("cred-1").expect("handle");

    let attached = instances(&fixture)
        .attach_credential(&instance, Some(&handle), 1, AT)
        .expect("attach");
    assert_eq!(attached.credential, Some(handle));

    let detached = instances(&fixture)
        .attach_credential(&instance, None, 2, AT)
        .expect("detach");
    assert_eq!(detached.credential, None);
}

#[test]
fn attaching_a_credential_to_an_instance_that_does_not_exist_is_refused() {
    // Otherwise a secret would sit in the credential store with nothing pointing at it.
    let fixture = fixture("connectors-credential-orphan");

    let error = instances(&fixture)
        .attach_credential(
            &InstanceId::parse("instance-missing").expect("instance"),
            Some(&CredentialHandle::parse("cred-1").expect("handle")),
            ABSENT_REVISION,
            AT,
        )
        .expect_err("no such instance");

    assert_eq!(error, ConnectorInstanceError::UnknownSubject);
}

#[test]
fn two_connections_editing_one_instance_leave_one_winner() {
    let fixture = fixture("connectors-instance-cas");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let first = Arc::new(SqliteConnectorInstanceRepository::new(
        fixture.database.clone(),
    ));
    let second = Arc::new(SqliteConnectorInstanceRepository::new(
        fixture.database.clone(),
    ));

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || {
        one.save(&InstanceEdit {
            instance: &InstanceId::parse("instance-1").expect("instance"),
            connector: &connector(),
            label: &DisplayLabel::parse("Left").expect("label"),
            desired_enabled: true,
            configuration: &configuration(),
            expected_revision: 1,
            at: AT,
        })
    });
    let right = std::thread::spawn(move || {
        two.save(&InstanceEdit {
            instance: &InstanceId::parse("instance-1").expect("instance"),
            connector: &connector(),
            label: &DisplayLabel::parse("Right").expect("label"),
            desired_enabled: false,
            configuration: &configuration(),
            expected_revision: 1,
            at: AT,
        })
    });

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one may edit: {outcomes:?}"
    );
    let loser = outcomes
        .iter()
        .find_map(|outcome| outcome.as_ref().err())
        .expect("one must lose");
    assert_eq!(loser.code(), "connector_instance_stale_revision");
}

// ---------------------------------------------------------------------------
// Bindings
// ---------------------------------------------------------------------------

#[test]
fn one_instance_holds_exactly_one_global_binding() {
    // The NULL-uniqueness trap: with a nullable target column SQLite would treat every NULL as
    // distinct and admit unlimited global bindings, each invisible to the others.
    let fixture = fixture("connectors-one-global");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");

    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-2").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            false,
            1,
            AT,
        )
        .expect("rebind");

    let held = bindings(&fixture).bindings(&instance).expect("bindings");
    assert_eq!(held.len(), 1, "expected one global binding, got {held:?}");
    assert_eq!(
        held[0].binding.as_str(),
        "binding-1",
        "the identity of 'this instance at this target' is the pair, so the original id stands"
    );
}

#[test]
fn a_scoped_binding_does_not_speak_for_the_global_one() {
    let fixture = fixture("connectors-targets");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");
    let project = ConnectorTarget::scoped(TargetKind::Project, "d:/work/repo").expect("project");

    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("global");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-2").expect("binding"),
            &instance,
            &project,
            false,
            ABSENT_REVISION,
            AT,
        )
        .expect("project");

    assert_eq!(
        bindings(&fixture)
            .bindings(&instance)
            .expect("bindings")
            .len(),
        2
    );
    assert!(
        bindings(&fixture)
            .binding(&instance, &ConnectorTarget::global())
            .expect("global")
            .expect("present")
            .enabled,
        "the project override must not have moved the global binding"
    );
}

#[test]
fn a_binding_cannot_reference_an_instance_that_does_not_exist() {
    let fixture = fixture("connectors-binding-fk");

    let error = bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &InstanceId::parse("instance-missing").expect("instance"),
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect_err("no such instance");

    assert_eq!(error.code(), "unknown_connector_instance");
}

#[test]
fn a_typed_target_the_database_does_not_know_is_refused() {
    let fixture = fixture("connectors-typed-target");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let connection = fixture.database.connection().expect("connection");

    for (kind, key) in [("workspace", "x"), ("global", "not-empty"), ("project", "")] {
        let error = connection
            .execute(
                "INSERT INTO connector_bindings \
                     (binding_id, instance_id, target_kind, target_key, enabled, revision, \
                      updated_at) \
                 VALUES ('binding-x', 'instance-1', ?1, ?2, 1, 1, ?3)",
                params![kind, key, AT],
            )
            .expect_err("the typed target check must refuse it");
        assert!(
            error.to_string().contains("CHECK"),
            "expected a CHECK refusal for {kind}/{key}, got {error}"
        );
    }
}

#[test]
fn two_connections_moving_one_binding_leave_one_winner() {
    let fixture = fixture("connectors-binding-cas");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");
    let first = Arc::new(SqliteConnectorBindingRepository::new(
        fixture.database.clone(),
    ));
    let second = Arc::new(SqliteConnectorBindingRepository::new(
        fixture.database.clone(),
    ));

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || {
        one.set(
            &BindingId::parse("binding-1").expect("binding"),
            &InstanceId::parse("instance-1").expect("instance"),
            &ConnectorTarget::global(),
            false,
            1,
            AT,
        )
    });
    let right = std::thread::spawn(move || {
        two.set(
            &BindingId::parse("binding-1").expect("binding"),
            &InstanceId::parse("instance-1").expect("instance"),
            &ConnectorTarget::global(),
            true,
            1,
            AT,
        )
    });

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one may move the binding: {outcomes:?}"
    );
    assert_eq!(
        outcomes
            .iter()
            .find_map(|outcome| outcome.as_ref().err())
            .expect("one must lose")
            .code(),
        "connector_binding_stale_revision"
    );
}

#[test]
fn an_unavailable_definition_removes_no_instance_binding_or_credential() {
    // The rule that makes a brief upgrade blip survivable: an extension that fails to activate for
    // thirty seconds must not cost the user every credential they configured for it. Readiness
    // gates new connects; it never deletes.
    let fixture = fixture("connectors-unavailable");
    definitions(&fixture)
        .record(&revision("snap-a", FIRST))
        .expect("record");
    create(&fixture, "instance-1", "Acme Prod").expect("create");
    let instance = InstanceId::parse("instance-1").expect("instance");
    instances(&fixture)
        .attach_credential(
            &instance,
            Some(&CredentialHandle::parse("cred-1").expect("handle")),
            1,
            AT,
        )
        .expect("attach");
    bindings(&fixture)
        .set(
            &BindingId::parse("binding-1").expect("binding"),
            &instance,
            &ConnectorTarget::global(),
            true,
            ABSENT_REVISION,
            AT,
        )
        .expect("bind");

    // Nothing in this subdomain deletes on absence: reconciliation is a read. Reading everything
    // back after the definition would have gone unavailable is the assertion.
    let held = instances(&fixture)
        .get(&instance)
        .expect("get")
        .expect("present");

    assert!(held.credential.is_some(), "the credential handle survives");
    assert_eq!(
        bindings(&fixture)
            .bindings(&instance)
            .expect("bindings")
            .len(),
        1
    );
    assert_eq!(
        instances(&fixture)
            .for_connector(&connector())
            .expect("instances")
            .len(),
        1
    );
}
