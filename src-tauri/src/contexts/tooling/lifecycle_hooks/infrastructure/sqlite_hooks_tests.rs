//! Migration 87 against a real database: references, idempotency, retention, redaction, and CAS.
//!
//! Every concurrency test here opens **two independent connections** from the pool. A single
//! connection serialises by construction, so a CAS test that shares one proves nothing about the
//! thing it claims to prove.

use super::{
    apply_lifecycle_hook_schema, SqliteHookBindingRepository, SqliteHookDefinitionRepository,
    SqliteHookExecutionRepository, SqliteHookSubjectRepository, ABSENT_BINDING_REVISION,
};
use crate::contexts::tooling::lifecycle_hooks::application::{
    HookBindingRepository, HookDefinitionRepository, HookExecutionRepository, HookSubjectRepository,
};
use crate::contexts::tooling::lifecycle_hooks::domain::{
    DefinitionDigest, DefinitionOutcome, HookBindingError, HookDefinitionRevision, HookEvent,
    HookExecutionError, HookExecutionId, HookExecutionRecord, HookExecutionRetention,
    HookExecutionStatus, HookGlobalId, HookOrigin, HookOutcomeCode, HookScope, HookScopeKind,
    HookSubject, SeedOutcome, SnapshotRef,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::sync::Arc;

const FIRST: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const SECOND: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const AT: &str = "2026-08-01T00:00:00Z";

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
    seed_subject(&fixture);
    fixture
}

fn hook() -> HookGlobalId {
    HookGlobalId::parse("ext::acme.git-guardian::pre-commit").expect("hook")
}

fn snapshot(value: &str) -> SnapshotRef {
    SnapshotRef::parse(value).expect("snapshot")
}

fn digest(value: &str) -> DefinitionDigest {
    DefinitionDigest::parse(value).expect("digest")
}

fn subject() -> HookSubject {
    HookSubject {
        hook: hook(),
        origin: HookOrigin::Extension,
        first_seen_at: AT.to_string(),
    }
}

fn seed_subject(fixture: &Fixture) {
    SqliteHookSubjectRepository::new(fixture.database.clone())
        .ensure(&subject())
        .expect("subject");
}

fn revision(snapshot_id: &str, value: &str, event: HookEvent) -> HookDefinitionRevision {
    HookDefinitionRevision {
        hook: hook(),
        snapshot: snapshot(snapshot_id),
        event,
        digest: digest(value),
        recorded_at: AT.to_string(),
    }
}

fn execution(id: &str, status: HookExecutionStatus) -> HookExecutionRecord {
    HookExecutionRecord {
        execution: HookExecutionId::parse(id).expect("execution"),
        hook: hook(),
        sequence: 0,
        status,
        outcome: status
            .is_terminal()
            .then(|| HookOutcomeCode::parse("exit_zero").expect("outcome")),
        duration_ms: Some(12),
        started_at: AT.to_string(),
        finished_at: status.is_terminal().then(|| AT.to_string()),
    }
}

#[test]
fn migration_87_creates_every_table_the_subdomain_owns() {
    let fixture = fixture("hooks-migration");
    let connection = fixture.database.connection().expect("connection");

    for table in [
        "lifecycle_hook_subjects",
        "lifecycle_hook_definition_revisions",
        "lifecycle_hook_bindings",
        "lifecycle_hook_executions",
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
fn migration_87_is_a_no_op_on_a_database_that_already_has_it() {
    // Every start runs it. A migration that is not idempotent is a migration that breaks the
    // second launch, which is the launch nobody tests by hand.
    let directory = TempDirectory::new("hooks-idempotent");
    let path = directory.path().join("repeat.sqlite");
    let connection = Connection::open(&path).expect("open");
    migrate(&connection).expect("first migrate");

    apply_lifecycle_hook_schema(&connection).expect("re-apply");
    apply_lifecycle_hook_schema(&connection).expect("re-apply again");
}

#[test]
fn an_execution_row_has_nowhere_to_put_a_prompt_a_path_or_a_message() {
    // The redaction guarantee, asserted against the schema rather than against a habit. A column
    // added later that could hold free text fails here, which is the only place it would be
    // noticed before something wrote a session's prompt into a durable row.
    let fixture = fixture("hooks-columns");
    let connection = fixture.database.connection().expect("connection");
    let mut statement = connection
        .prepare("SELECT name FROM pragma_table_info('lifecycle_hook_executions')")
        .expect("prepare");
    let columns: BTreeSet<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(|column| column.expect("column"))
        .collect();

    let permitted: BTreeSet<String> = [
        "execution_id",
        "hook_global_id",
        "sequence",
        "status",
        "terminal",
        "outcome_code",
        "duration_ms",
        "started_at",
        "finished_at",
    ]
    .iter()
    .map(|name| (*name).to_string())
    .collect();

    assert_eq!(
        columns, permitted,
        "an execution row may record that a Hook ran and how it ended, never what it saw or said"
    );
}

#[test]
fn an_outcome_code_column_cannot_be_loaded_with_a_message_through_the_repository() {
    // The column is TEXT, so SQLite would take a stderr dump. What stops it is that the only way
    // to build a record is through `HookOutcomeCode`, whose grammar refuses one.
    assert!(HookOutcomeCode::parse("Failed: C:\\Users\\alice\\hook.ps1 exited 1").is_err());
}

#[test]
fn re_seeding_a_subject_does_not_move_when_it_was_first_seen() {
    // Every start seeds the built-ins. Rewriting `first_seen_at` would erase the only record of
    // when the Hook entered this installation, and would do it on a schedule -- every launch.
    let fixture = fixture("hooks-subject-seed");
    let subjects = SqliteHookSubjectRepository::new(fixture.database.clone());

    subjects
        .ensure(&HookSubject {
            first_seen_at: "2026-09-01T00:00:00Z".to_string(),
            origin: HookOrigin::Builtin,
            ..subject()
        })
        .expect("re-seed");

    let held = subjects.get(&hook()).expect("get").expect("present");
    assert_eq!(held.first_seen_at, AT, "the first sighting stands");
    assert_eq!(
        held.origin,
        HookOrigin::Extension,
        "and a re-seed cannot relabel where the subject came from"
    );
    assert_eq!(subjects.all().expect("all").len(), 1);
}

#[test]
fn a_definition_cannot_reference_a_subject_that_does_not_exist() {
    let fixture = fixture("hooks-fk");
    let definitions = SqliteHookDefinitionRepository::new(fixture.database.clone());
    let orphan = HookDefinitionRevision {
        hook: HookGlobalId::parse("ext::nobody.nothing::never").expect("hook"),
        ..revision("snap-a", FIRST, HookEvent::PreToolUse)
    };

    let error = definitions.record(&orphan).expect_err("no such subject");

    assert_eq!(error, "unknown_hook_subject");
}

#[test]
fn a_binding_cannot_reference_a_subject_that_does_not_exist() {
    let fixture = fixture("hooks-binding-fk");
    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());

    let error = bindings
        .set(
            &HookGlobalId::parse("ext::nobody.nothing::never").expect("hook"),
            &HookScope::global(),
            true,
            ABSENT_BINDING_REVISION,
            AT,
        )
        .expect_err("no such subject");

    assert_eq!(error, HookBindingError::UnknownSubject);
}

#[test]
fn evidence_is_not_removed_by_deleting_what_points_at_it() {
    // RESTRICT everywhere, CASCADE nowhere. Deleting a subject that still has a definition, a
    // binding, or an execution must fail and force whoever is doing it to say what should happen
    // to the evidence.
    let fixture = fixture("hooks-restrict");
    SqliteHookDefinitionRepository::new(fixture.database.clone())
        .record(&revision("snap-a", FIRST, HookEvent::PreToolUse))
        .expect("record");
    SqliteHookBindingRepository::new(fixture.database.clone())
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("seed");
    SqliteHookExecutionRepository::new(fixture.database.clone())
        .append(&execution("exec-1", HookExecutionStatus::Succeeded))
        .expect("append");

    let connection = fixture.database.connection().expect("connection");
    let error = connection
        .execute(
            "DELETE FROM lifecycle_hook_subjects WHERE hook_global_id = ?1",
            [hook().as_str()],
        )
        .expect_err("the reference must hold");

    assert!(
        error.to_string().contains("FOREIGN KEY"),
        "expected a foreign-key refusal, got {error}"
    );
}

#[test]
fn re_recording_the_same_definition_is_idempotent_and_a_different_one_is_refused() {
    let fixture = fixture("hooks-definitions");
    let definitions = SqliteHookDefinitionRepository::new(fixture.database.clone());
    let recorded = revision("snap-a", FIRST, HookEvent::PreToolUse);

    assert_eq!(
        definitions.record(&recorded).expect("record"),
        DefinitionOutcome::Recorded
    );
    assert_eq!(
        definitions.record(&recorded).expect("re-record"),
        DefinitionOutcome::AlreadyRecorded,
        "reinstalling a snapshot must not break the Hook it reinstalled"
    );

    let rebuilt = revision("snap-a", SECOND, HookEvent::PreToolUse);
    let outcome = definitions.record(&rebuilt).expect("conflicting record");

    assert!(
        !outcome.admits_dispatch(),
        "a pair with two answers must not be dispatched from: {outcome:?}"
    );
    assert_eq!(
        definitions
            .recorded(&hook(), &snapshot("snap-a"))
            .expect("recorded")
            .map(|revision| revision.digest),
        Some(digest(FIRST)),
        "the stored row is untouched; a rebuild cannot change what an installed snapshot means"
    );
}

#[test]
fn two_snapshots_each_hold_their_own_revision_of_one_subject() {
    let fixture = fixture("hooks-two-snapshots");
    let definitions = SqliteHookDefinitionRepository::new(fixture.database.clone());

    definitions
        .record(&revision("snap-a", FIRST, HookEvent::PreToolUse))
        .expect("first");
    definitions
        .record(&revision("snap-b", SECOND, HookEvent::PostToolUse))
        .expect("second");

    assert_eq!(
        definitions.revisions(&hook()).expect("revisions").len(),
        2,
        "an upgrade records beside the old revision so a rollback still has something to run"
    );
}

#[test]
fn a_seed_never_overwrites_a_binding_the_user_already_has() {
    // The failure this prevents: a built-in Hook is disabled, an upgrade re-seeds its defaults,
    // and the Hook starts running again. The user finds out when it runs.
    let fixture = fixture("hooks-seed");
    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());

    assert_eq!(
        bindings
            .seed_default(&hook(), &HookScope::global(), true, AT)
            .expect("seed"),
        SeedOutcome::Seeded
    );
    let disabled = bindings
        .set(&hook(), &HookScope::global(), false, 1, AT)
        .expect("disable");
    assert!(!disabled.enabled);

    assert_eq!(
        bindings
            .seed_default(&hook(), &HookScope::global(), true, "2026-09-01T00:00:00Z")
            .expect("re-seed"),
        SeedOutcome::Preserved
    );
    let held = bindings
        .binding(&hook(), &HookScope::global())
        .expect("binding")
        .expect("present");
    assert!(!held.enabled, "the user's choice survives the re-seed");
    assert_eq!(
        held.revision, 2,
        "and the re-seed did not write, so the revision did not move"
    );
}

#[test]
fn one_hook_holds_exactly_one_global_binding() {
    // The NULL-uniqueness trap: with a nullable scope column, SQLite would treat every NULL as
    // distinct and admit unlimited global bindings, each invisible to the others.
    let fixture = fixture("hooks-one-global");
    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());

    bindings
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("seed");
    bindings
        .set(&hook(), &HookScope::global(), false, 1, AT)
        .expect("move");
    bindings
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("re-seed");

    let held = bindings.bindings(&hook()).expect("bindings");
    assert_eq!(held.len(), 1, "expected one global binding, got {held:?}");
}

#[test]
fn a_scoped_binding_does_not_speak_for_the_global_one() {
    let fixture = fixture("hooks-scopes");
    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());
    let project = HookScope::scoped(HookScopeKind::Project, "d:/work/repo").expect("project");

    bindings
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("global");
    bindings
        .seed_default(&hook(), &project, false, AT)
        .expect("project");

    assert_eq!(bindings.bindings(&hook()).expect("bindings").len(), 2);
    assert!(
        bindings
            .binding(&hook(), &HookScope::global())
            .expect("global")
            .expect("present")
            .enabled,
        "the project override must not have moved the global binding"
    );
}

#[test]
fn a_binding_move_from_a_stale_revision_is_refused_and_reports_both_numbers() {
    let fixture = fixture("hooks-stale");
    let bindings = SqliteHookBindingRepository::new(fixture.database.clone());
    bindings
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("seed");

    let error = bindings
        .set(
            &hook(),
            &HookScope::global(),
            false,
            ABSENT_BINDING_REVISION,
            AT,
        )
        .expect_err("stale");

    assert_eq!(
        error,
        HookBindingError::StaleRevision {
            expected: ABSENT_BINDING_REVISION,
            actual: 1,
        }
    );
}

#[test]
fn two_connections_moving_the_same_binding_leave_one_winner() {
    // Two independent connections. The read and the write are in one write transaction, so the
    // loser sees the winner's revision rather than the one it started from.
    let fixture = fixture("hooks-binding-cas");
    SqliteHookBindingRepository::new(fixture.database.clone())
        .seed_default(&hook(), &HookScope::global(), true, AT)
        .expect("seed");
    let first = Arc::new(SqliteHookBindingRepository::new(fixture.database.clone()));
    let second = Arc::new(SqliteHookBindingRepository::new(fixture.database.clone()));

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || one.set(&hook(), &HookScope::global(), false, 1, AT));
    let right = std::thread::spawn(move || two.set(&hook(), &HookScope::global(), true, 1, AT));

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one may move the binding: {outcomes:?}"
    );
    let loser = outcomes
        .iter()
        .find_map(|outcome| outcome.as_ref().err())
        .expect("one must lose");
    assert_eq!(
        loser.code(),
        "hook_binding_stale_revision",
        "the loser is told its revision was stale, not that storage broke"
    );
}

#[test]
fn appended_executions_carry_a_monotonic_sequence_the_caller_does_not_choose() {
    let fixture = fixture("hooks-sequence");
    let executions = SqliteHookExecutionRepository::new(fixture.database.clone());

    let first = executions
        .append(&execution("exec-1", HookExecutionStatus::Succeeded))
        .expect("first");
    let second = executions
        .append(&execution("exec-2", HookExecutionStatus::Failed))
        .expect("second");

    assert_eq!((first.sequence, second.sequence), (1, 2));
    assert_eq!(
        executions
            .recent(&hook(), 10)
            .expect("recent")
            .iter()
            .map(|record| record.sequence)
            .collect::<Vec<_>>(),
        vec![2, 1],
        "newest first, ordered by sequence rather than by a timestamp that can tie"
    );
}

#[test]
fn re_appending_an_execution_id_is_refused_rather_than_treated_as_an_update() {
    let fixture = fixture("hooks-duplicate");
    let executions = SqliteHookExecutionRepository::new(fixture.database.clone());
    executions
        .append(&execution("exec-1", HookExecutionStatus::Succeeded))
        .expect("first");

    let error = executions
        .append(&execution("exec-1", HookExecutionStatus::Failed))
        .expect_err("duplicate");

    assert_eq!(error, HookExecutionError::DuplicateExecution);
}

#[test]
fn an_execution_cannot_reference_a_subject_that_does_not_exist() {
    let fixture = fixture("hooks-execution-fk");
    let executions = SqliteHookExecutionRepository::new(fixture.database.clone());
    let orphan = HookExecutionRecord {
        hook: HookGlobalId::parse("ext::nobody.nothing::never").expect("hook"),
        ..execution("exec-1", HookExecutionStatus::Succeeded)
    };

    assert_eq!(
        executions.append(&orphan).expect_err("no such subject"),
        HookExecutionError::UnknownSubject
    );
}

#[test]
fn retention_removes_old_terminal_rows_and_never_an_unfinished_one() {
    // The one deletion an execution log must never make. A pending or running row is not old, it
    // is unfinished; removing it turns a Hook that is still going into a Hook that never happened,
    // and the completion that arrives afterwards has nothing to attach to.
    let fixture = fixture("hooks-retention");
    let executions = SqliteHookExecutionRepository::new(fixture.database.clone());
    let running = executions
        .append(&execution("exec-running", HookExecutionStatus::Running))
        .expect("running");
    let pending = executions
        .append(&execution("exec-pending", HookExecutionStatus::Pending))
        .expect("pending");
    for index in 0..6 {
        executions
            .append(&execution(
                &format!("exec-done-{index}"),
                HookExecutionStatus::Succeeded,
            ))
            .expect("terminal");
    }

    let removed = executions
        .prune(&hook(), HookExecutionRetention::new(3).expect("window"))
        .expect("prune");

    let kept: Vec<i64> = executions
        .recent(&hook(), 100)
        .expect("recent")
        .iter()
        .map(|record| record.sequence)
        .collect();
    assert_eq!(removed, 3, "three terminal rows fell outside the window");
    assert!(
        kept.contains(&running.sequence) && kept.contains(&pending.sequence),
        "unfinished rows survive regardless of age: {kept:?}"
    );
    assert_eq!(kept.len(), 5, "the newest three plus the two unfinished");
}

#[test]
fn a_sequence_is_never_reissued_after_a_prune() {
    // Retention keeps at least the newest row, so MAX never returns to nothing for a subject that
    // has run -- which is what makes "monotonic" unconditional rather than true until first prune.
    let fixture = fixture("hooks-sequence-after-prune");
    let executions = SqliteHookExecutionRepository::new(fixture.database.clone());
    for index in 0..5 {
        executions
            .append(&execution(
                &format!("exec-{index}"),
                HookExecutionStatus::Succeeded,
            ))
            .expect("append");
    }

    executions
        .prune(&hook(), HookExecutionRetention::new(1).expect("window"))
        .expect("prune");
    let next = executions
        .append(&execution("exec-after", HookExecutionStatus::Succeeded))
        .expect("append after prune");

    assert_eq!(
        next.sequence, 6,
        "the next sequence follows the survivor, not the emptied history"
    );
}

#[test]
fn two_connections_appending_and_pruning_leave_a_consistent_log() {
    // Appends take a write lock for the read of MAX and the insert together, so two of them cannot
    // compute the same sequence. Pruning runs against the same database at the same time to prove
    // it never removes the row an in-flight append is about to follow.
    let fixture = fixture("hooks-append-cas");
    let first = Arc::new(SqliteHookExecutionRepository::new(fixture.database.clone()));
    let second = Arc::new(SqliteHookExecutionRepository::new(fixture.database.clone()));
    first
        .append(&execution("exec-seed", HookExecutionStatus::Running))
        .expect("seed");

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let appending = std::thread::spawn(move || {
        (0..8)
            .map(|index| {
                one.append(&execution(
                    &format!("exec-a-{index}"),
                    HookExecutionStatus::Succeeded,
                ))
            })
            .collect::<Vec<_>>()
    });
    let pruning = std::thread::spawn(move || {
        (0..8)
            .map(|_| two.prune(&hook(), HookExecutionRetention::new(2).expect("window")))
            .collect::<Vec<_>>()
    });

    let appended = appending.join().expect("thread");
    pruning.join().expect("thread");

    let mut sequences: Vec<i64> = appended
        .iter()
        .map(|result| result.as_ref().expect("append").sequence)
        .collect();
    let total = sequences.len();
    sequences.sort_unstable();
    sequences.dedup();
    assert_eq!(
        sequences.len(),
        total,
        "two appends computed the same sequence: {sequences:?}"
    );

    let survivors = first.recent(&hook(), 100).expect("recent");
    assert!(
        survivors
            .iter()
            .any(|record| record.status == HookExecutionStatus::Running),
        "the unfinished row survived every prune: {survivors:?}"
    );
}
