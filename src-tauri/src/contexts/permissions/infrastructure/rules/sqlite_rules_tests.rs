//! Migration 88 against a real database.
//!
//! Half of these assert what migration 88 does. The other half assert what it does **not** do:
//! templates are not seeded as rules, grants are untouched, and nothing about the existing
//! permissions path changes while the rule set is unwired. Those are the assertions that would
//! catch this migration quietly replacing the PDP instead of sitting beside it.
//!
//! The concurrency test opens **two independent connections** from the pool. A single connection
//! serialises by construction, so a CAS test that shares one proves nothing.

use super::{
    apply_authorization_rule_schema, rule_set_digest, SqliteActiveRuleSetRepository,
    SqliteRuleSetRepository,
};
use crate::contexts::permissions::application::rules::{
    ActiveRuleSetRepository, RuleSetRepository,
};
use crate::contexts::permissions::application::{DefaultTemplatePort, EvaluationService};
use crate::contexts::permissions::domain::rules::{
    AllowedScopes, AuthorizationRule, GrantScope, Matcher, OperationName, RuleEffect, RuleId,
    RuleProvenance, RuleScope, RuleScopeKind, RuleSetDigest, RuleSetId, RuleSetOutcome, RuleSource,
    SourceId,
};
use crate::contexts::permissions::domain::{Action, Effect, PolicyTemplateName, Resource};
use crate::contexts::permissions::infrastructure::{
    PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
    SqliteGrantRepository, SqlitePrincipalRepository,
};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use rusqlite::{params, Connection};
use std::sync::Arc;

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
    Fixture {
        _directory: directory,
        database,
    }
}

fn sets(fixture: &Fixture) -> SqliteRuleSetRepository {
    SqliteRuleSetRepository::new(fixture.database.clone())
}

fn pointer(fixture: &Fixture) -> SqliteActiveRuleSetRepository {
    SqliteActiveRuleSetRepository::new(fixture.database.clone())
}

fn set_id(value: &str) -> RuleSetId {
    RuleSetId::parse(value).expect("rule set")
}

fn rule(id: &str, source: RuleSource, effect: RuleEffect) -> AuthorizationRule {
    AuthorizationRule {
        source,
        source_id: SourceId::parse("user-settings").expect("source"),
        rule_id: RuleId::parse(id).expect("rule"),
        scope: RuleScope::global(),
        operation: OperationName::parse("shell.exec").expect("operation"),
        matcher: Matcher::Prefix("git push --force".to_string()),
        effect,
        allowed_scopes: if effect == RuleEffect::Ask {
            AllowedScopes::of(&[GrantScope::Once])
        } else {
            AllowedScopes::none()
        },
        priority: 0,
        expires_at: None,
        provenance: RuleProvenance::UserSettings,
    }
}

fn record(fixture: &Fixture, id: &str, rules: &[AuthorizationRule]) -> RuleSetOutcome {
    let digest = rule_set_digest(rules).expect("digest");
    sets(fixture)
        .record(&set_id(id), &digest, rules, AT)
        .expect("record")
}

// ---------------------------------------------------------------------------
// Schema
// ---------------------------------------------------------------------------

#[test]
fn migration_88_creates_every_table_the_subdomain_owns() {
    let fixture = fixture("rules-migration");
    let connection = fixture.database.connection().expect("connection");

    for table in [
        "permission_rule_sets",
        "permission_authorization_rules",
        "permission_active_rule_set",
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
fn migration_88_is_a_no_op_on_a_database_that_already_has_it() {
    let directory = TempDirectory::new("rules-idempotent");
    let path = directory.path().join("repeat.sqlite");
    let connection = Connection::open(&path).expect("open");
    migrate(&connection).expect("first migrate");

    apply_authorization_rule_schema(&connection).expect("re-apply");
    apply_authorization_rule_schema(&connection).expect("re-apply again");
}

#[test]
fn the_active_pointer_starts_at_nothing_rather_than_at_a_fabricated_empty_set() {
    // "No rules have been published" and "a published set that happens to be empty" are different
    // facts. Inventing the second would claim a digest nobody produced.
    let fixture = fixture("rules-initial-pointer");

    let active = pointer(&fixture).active().expect("active");

    assert_eq!(active.rule_set_id, None);
    assert_eq!(active.revision, 0);

    let connection = fixture.database.connection().expect("connection");
    let sets_stored: i64 = connection
        .query_row("SELECT COUNT(*) FROM permission_rule_sets", [], |row| {
            row.get(0)
        })
        .expect("count");
    assert_eq!(
        sets_stored, 0,
        "no rule set is fabricated at migration time"
    );
}

#[test]
fn the_active_pointer_is_a_singleton() {
    let fixture = fixture("rules-singleton");
    let connection = fixture.database.connection().expect("connection");

    let error = connection
        .execute(
            "INSERT INTO permission_active_rule_set (id, rule_set_id, revision, updated_at) \
             VALUES (2, NULL, 0, ?1)",
            params![AT],
        )
        .expect_err("a second pointer row must be unrepresentable");

    assert!(
        error.to_string().contains("CHECK"),
        "expected the singleton check to refuse it, got {error}"
    );
}

// ---------------------------------------------------------------------------
// What migration 88 leaves alone
// ---------------------------------------------------------------------------

#[test]
fn template_assignments_survive_the_upgrade_unchanged() {
    // Templates stay with the existing PDP. If migration 88 ever rewrote a principal's template --
    // to "compile" it into rules, say -- an agent's policy would change during an upgrade nobody
    // asked for.
    let directory = TempDirectory::new("rules-templates-preserved");
    let path = directory.path().join("templates.sqlite");
    let connection = Connection::open(&path).expect("open");
    migrate(&connection).expect("migrate");
    connection
        .execute(
            "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
             VALUES ('principal-1', 'agent-1', 'readonly', ?1, ?1)",
            params![AT],
        )
        .expect("seed principal");

    // Re-running every migration is what an upgrade does.
    migrate(&connection).expect("re-migrate");
    apply_authorization_rule_schema(&connection).expect("re-apply");

    let template: String = connection
        .query_row(
            "SELECT template_name FROM agent_principals WHERE id = 'principal-1'",
            [],
            |row| row.get(0),
        )
        .expect("read template");
    assert_eq!(template, "readonly");
}

#[test]
fn no_rule_is_seeded_for_any_template() {
    // There is no `source_kind = 'template'`, and the four shipped templates contribute no rows.
    // A template compiled into rules would make every template assignment publish an immutable
    // rule set, and would put the host's own fallback behind the same review path as a downloaded
    // extension's rules.
    let fixture = fixture("rules-no-template-seed");
    let connection = fixture.database.connection().expect("connection");

    let seeded: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM permission_authorization_rules",
            [],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(seeded, 0);

    let error = connection
        .execute(
            "INSERT INTO permission_authorization_rules \
                 (rule_set_id, source_kind, source_id, rule_id, scope_kind, scope_key, operation, \
                  matcher, effect, allowed_scopes, priority, specificity, expires_at, provenance) \
             VALUES ('set-a', 'template', 'standard', 'r1', 'global', '', 'shell.exec', 'any', \
                     'ask', 'once', 0, 0, NULL, 'host_default')",
            [],
        )
        .expect_err("template is not a persisted rule source");
    assert!(
        error.to_string().contains("CHECK"),
        "expected the source check to refuse it, got {error}"
    );
}

#[test]
fn existing_grants_are_neither_moved_nor_counted() {
    // Grants stay with the Approval Broker. Migration 88 does not copy, rebuild, or delete one,
    // and no grant is part of any rule-set digest.
    let directory = TempDirectory::new("rules-grants-preserved");
    let path = directory.path().join("grants.sqlite");
    let connection = Connection::open(&path).expect("open");
    migrate(&connection).expect("migrate");
    connection
        .execute(
            "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
             VALUES ('principal-1', 'agent-1', 'standard', ?1, ?1)",
            params![AT],
        )
        .expect("seed principal");
    connection
        .execute(
            "INSERT INTO permission_grants \
                 (id, principal_id, action, resource, effect, scope, session_id, project_key, \
                  created_at) \
             VALUES ('grant-1', 'principal-1', 'shell.exec', 'workspace', 'allow', 'session', \
                     'session-1', NULL, ?1)",
            params![AT],
        )
        .expect("seed grant");

    migrate(&connection).expect("re-migrate");
    apply_authorization_rule_schema(&connection).expect("re-apply");

    let (count, effect, scope): (i64, String, String) = connection
        .query_row(
            "SELECT COUNT(*), MAX(effect), MAX(scope) FROM permission_grants",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("read grants");
    assert_eq!(
        (count, effect.as_str(), scope.as_str()),
        (1, "allow", "session")
    );
}

#[test]
fn activating_a_rule_set_does_not_touch_grants_or_templates() {
    // Rolling a rule set forward or back must not destroy a user's remembered answers; eligibility
    // is re-decided the next time a grant would be used.
    let fixture = fixture("rules-activation-isolated");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
             VALUES ('principal-1', 'agent-1', 'trusted', ?1, ?1)",
            params![AT],
        )
        .expect("seed principal");
    connection
        .execute(
            "INSERT INTO permission_grants \
                 (id, principal_id, action, resource, effect, scope, session_id, project_key, \
                  created_at) \
             VALUES ('grant-1', 'principal-1', 'shell.exec', 'workspace', 'allow', 'global', \
                     NULL, NULL, ?1)",
            params![AT],
        )
        .expect("seed grant");
    drop(connection);

    record(
        &fixture,
        "set-a",
        &[rule("r1", RuleSource::User, RuleEffect::Deny)],
    );
    pointer(&fixture)
        .activate(&set_id("set-a"), 0, AT)
        .expect("activate");
    record(
        &fixture,
        "set-b",
        &[rule("r2", RuleSource::User, RuleEffect::Ask)],
    );
    pointer(&fixture)
        .activate(&set_id("set-b"), 1, AT)
        .expect("re-activate");

    let connection = fixture.database.connection().expect("connection");
    let grants: i64 = connection
        .query_row("SELECT COUNT(*) FROM permission_grants", [], |row| {
            row.get(0)
        })
        .expect("count");
    let template: String = connection
        .query_row(
            "SELECT template_name FROM agent_principals WHERE id = 'principal-1'",
            [],
            |row| row.get(0),
        )
        .expect("read template");

    assert_eq!(grants, 1, "a rule-set switch does not delete a grant");
    assert_eq!(
        template, "trusted",
        "nor does it rewrite a template assignment"
    );
}

#[test]
fn a_template_change_does_not_move_the_active_rule_set_pointer() {
    let fixture = fixture("rules-template-change");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
             VALUES ('principal-1', 'agent-1', 'standard', ?1, ?1)",
            params![AT],
        )
        .expect("seed principal");

    let before = pointer(&fixture).active().expect("active");
    connection
        .execute(
            "UPDATE agent_principals SET template_name = 'yolo' WHERE id = 'principal-1'",
            [],
        )
        .expect("change template");
    let after = pointer(&fixture).active().expect("active");

    assert_eq!(before, after);
    assert_eq!(after.rule_set_id, None);
    assert_eq!(after.revision, 0);
}

// ---------------------------------------------------------------------------
// Rule sets
// ---------------------------------------------------------------------------

#[test]
fn recording_the_same_contents_twice_is_idempotent() {
    let fixture = fixture("rules-idempotent-digest");
    let rules = [rule("r1", RuleSource::User, RuleEffect::Deny)];

    assert_eq!(record(&fixture, "set-a", &rules), RuleSetOutcome::Recorded);
    assert_eq!(
        record(&fixture, "set-a", &rules),
        RuleSetOutcome::AlreadyRecorded {
            existing: set_id("set-a")
        }
    );
}

#[test]
fn the_same_contents_under_a_new_id_return_the_id_already_in_storage() {
    // A caller recompiling the same rules gets a fresh id each time. Handing back the fresh id
    // would leave it activating a rule set that does not exist.
    let fixture = fixture("rules-dedup");
    let rules = [rule("r1", RuleSource::User, RuleEffect::Deny)];
    record(&fixture, "set-a", &rules);

    let outcome = record(&fixture, "set-b", &rules);

    assert_eq!(
        outcome,
        RuleSetOutcome::AlreadyRecorded {
            existing: set_id("set-a")
        }
    );
    assert_eq!(outcome.activatable(&set_id("set-b")), Some(set_id("set-a")));
    assert!(sets(&fixture)
        .rule_set(&set_id("set-b"))
        .expect("read")
        .is_none());
}

#[test]
fn one_id_with_different_contents_is_a_conflict_and_the_stored_set_is_untouched() {
    let fixture = fixture("rules-conflict");
    let first = [rule("r1", RuleSource::User, RuleEffect::Deny)];
    let second = [rule("r1", RuleSource::User, RuleEffect::Allow)];
    record(&fixture, "set-a", &first);

    let outcome = record(&fixture, "set-a", &second);

    assert!(!outcome.admits_activation(), "{outcome:?}");
    assert_eq!(outcome.code(), "rule_set_content_conflict");
    assert_eq!(
        sets(&fixture)
            .rule_set(&set_id("set-a"))
            .expect("read")
            .expect("present")
            .rules,
        first.to_vec(),
        "a rebuild cannot change what an already-published set means"
    );
}

#[test]
fn a_rule_set_reads_back_exactly_as_it_was_written() {
    let fixture = fixture("rules-round-trip");
    let written = [
        rule("r1", RuleSource::User, RuleEffect::Deny),
        AuthorizationRule {
            scope: RuleScope::scoped(RuleScopeKind::Project, "d:/work/repo").expect("project"),
            matcher: Matcher::Any,
            expires_at: Some("2099-01-01T00:00:00Z".to_string()),
            priority: 7,
            provenance: RuleProvenance::ProjectSettings,
            ..rule("r2", RuleSource::Project, RuleEffect::Ask)
        },
    ];
    record(&fixture, "set-a", &written);

    let read = sets(&fixture)
        .rule_set(&set_id("set-a"))
        .expect("read")
        .expect("present");

    // Compared as a set: the read orders by `(source_kind, source_id, rule_id)` while the written
    // order is whatever the caller happened to build. Asserting the caller's order would pin a
    // detail the digest deliberately does not depend on.
    let mut expected: Vec<String> = written.iter().map(|rule| format!("{rule:?}")).collect();
    let mut actual: Vec<String> = read.rules.iter().map(|rule| format!("{rule:?}")).collect();
    expected.sort_unstable();
    actual.sort_unstable();
    assert_eq!(actual, expected);
    assert_eq!(
        read.content_digest,
        rule_set_digest(&written).expect("digest")
    );
}

#[test]
fn the_stored_digest_does_not_depend_on_the_order_rows_come_back_in() {
    // The read orders by `(source_kind, source_id, rule_id)`, and the digest must not care. If it
    // did, adding an index could change the digest of a set nobody edited.
    let fixture = fixture("rules-digest-order");
    let forward = [
        rule("aaa", RuleSource::User, RuleEffect::Deny),
        rule("bbb", RuleSource::User, RuleEffect::Ask),
    ];
    let reversed = [forward[1].clone(), forward[0].clone()];
    record(&fixture, "set-a", &forward);

    let read = sets(&fixture)
        .rule_set(&set_id("set-a"))
        .expect("read")
        .expect("present");

    assert_eq!(
        read.content_digest,
        rule_set_digest(&reversed).expect("digest"),
        "the same rules in another order are the same set"
    );
}

#[test]
fn the_stored_specificity_is_the_one_the_domain_computes() {
    // Derived, so it is written here and never taken from a caller. A stored ordering that
    // disagreed with the evaluator's would only show up as a trace naming the wrong decisive rule.
    let fixture = fixture("rules-specificity");
    let written = AuthorizationRule {
        scope: RuleScope::scoped(RuleScopeKind::Session, "session-1").expect("session"),
        matcher: Matcher::Exact("git push".to_string()),
        ..rule("r1", RuleSource::User, RuleEffect::Deny)
    };
    record(&fixture, "set-a", std::slice::from_ref(&written));

    let stored: i64 = fixture
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT specificity FROM permission_authorization_rules WHERE rule_id = 'r1'",
            [],
            |row| row.get(0),
        )
        .expect("read specificity");

    assert_eq!(stored, written.specificity());
}

#[test]
fn an_extension_rule_that_allows_is_refused_by_the_database_too() {
    // `AuthorizationRule::admit` is the first guard. This is the second, because an `Allow` from a
    // downloaded package is a privilege escalation and one guard is a refactor away from gone.
    let fixture = fixture("rules-extension-allow");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO permission_rule_sets (rule_set_id, content_digest, rule_count, created_at) \
             VALUES ('set-a', ?1, 1, ?2)",
            params!["a".repeat(64), AT],
        )
        .expect("seed set");

    let error = connection
        .execute(
            "INSERT INTO permission_authorization_rules \
                 (rule_set_id, source_kind, source_id, rule_id, scope_kind, scope_key, operation, \
                  matcher, effect, allowed_scopes, priority, specificity, expires_at, provenance) \
             VALUES ('set-a', 'extension', 'acme', 'r1', 'global', '', 'shell.exec', 'any', \
                     'allow', '', 0, 0, NULL, 'extension_manifest')",
            [],
        )
        .expect_err("an extension may not allow");

    assert!(
        error.to_string().contains("CHECK"),
        "expected the effect check to refuse it, got {error}"
    );
}

#[test]
fn a_row_this_build_cannot_parse_makes_the_whole_set_unreadable() {
    // Skipping the row would silently produce a rule set missing exactly one `Deny`, and nothing
    // downstream could tell that had happened.
    let fixture = fixture("rules-fail-closed");
    record(
        &fixture,
        "set-a",
        &[rule("r1", RuleSource::User, RuleEffect::Deny)],
    );
    fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "UPDATE permission_authorization_rules SET matcher = 'regex:^git' WHERE rule_id = 'r1'",
            [],
        )
        .expect("corrupt the matcher");

    let error = sets(&fixture)
        .rule_set(&set_id("set-a"))
        .expect_err("an unreadable rule fails the read");

    assert!(
        error.contains("invalid_rule_matcher"),
        "expected the matcher code, got {error}"
    );
}

// ---------------------------------------------------------------------------
// The active pointer
// ---------------------------------------------------------------------------

#[test]
fn the_pointer_cannot_name_a_rule_set_that_does_not_exist() {
    let fixture = fixture("rules-pointer-fk");

    let error = pointer(&fixture)
        .activate(&set_id("set-missing"), 0, AT)
        .expect_err("no such rule set");

    assert_eq!(error.code(), "unknown_rule_set");
}

#[test]
fn an_active_rule_set_cannot_be_deleted_out_from_under_the_pointer() {
    let fixture = fixture("rules-pointer-restrict");
    record(
        &fixture,
        "set-a",
        &[rule("r1", RuleSource::User, RuleEffect::Deny)],
    );
    pointer(&fixture)
        .activate(&set_id("set-a"), 0, AT)
        .expect("activate");

    let error = fixture
        .database
        .connection()
        .expect("connection")
        .execute(
            "DELETE FROM permission_rule_sets WHERE rule_set_id = 'set-a'",
            [],
        )
        .expect_err("the reference must hold");

    assert!(
        error.to_string().contains("FOREIGN KEY"),
        "expected a foreign-key refusal, got {error}"
    );
}

#[test]
fn moving_the_pointer_from_a_stale_revision_is_refused() {
    let fixture = fixture("rules-pointer-stale");
    record(
        &fixture,
        "set-a",
        &[rule("r1", RuleSource::User, RuleEffect::Deny)],
    );
    pointer(&fixture)
        .activate(&set_id("set-a"), 0, AT)
        .expect("activate");

    let error = pointer(&fixture)
        .activate(&set_id("set-a"), 0, AT)
        .expect_err("stale");

    assert_eq!(error.code(), "active_rule_set_stale_revision");
}

#[test]
fn two_connections_activating_from_the_same_revision_leave_one_winner() {
    // Two independent connections. The read and the write are in one write transaction, so the
    // loser sees the winner's revision rather than the one it started from.
    let fixture = fixture("rules-pointer-cas");
    record(
        &fixture,
        "set-a",
        &[rule("r1", RuleSource::User, RuleEffect::Deny)],
    );
    record(
        &fixture,
        "set-b",
        &[rule("r2", RuleSource::User, RuleEffect::Ask)],
    );
    let first = Arc::new(SqliteActiveRuleSetRepository::new(fixture.database.clone()));
    let second = Arc::new(SqliteActiveRuleSetRepository::new(fixture.database.clone()));

    let one = Arc::clone(&first);
    let two = Arc::clone(&second);
    let left = std::thread::spawn(move || one.activate(&set_id("set-a"), 0, AT));
    let right = std::thread::spawn(move || two.activate(&set_id("set-b"), 0, AT));

    let outcomes = [left.join().expect("thread"), right.join().expect("thread")];

    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.is_ok()).count(),
        1,
        "exactly one may activate: {outcomes:?}"
    );
    let loser = outcomes
        .iter()
        .find_map(|outcome| outcome.as_ref().err())
        .expect("one must lose");
    assert_eq!(loser.code(), "active_rule_set_stale_revision");
    assert_eq!(pointer(&fixture).active().expect("active").revision, 1);
}

#[test]
fn a_digest_lookup_finds_the_set_it_names() {
    let fixture = fixture("rules-by-digest");
    let rules = [rule("r1", RuleSource::User, RuleEffect::Deny)];
    record(&fixture, "set-a", &rules);

    assert_eq!(
        sets(&fixture)
            .by_digest(&rule_set_digest(&rules).expect("digest"))
            .expect("lookup"),
        Some(set_id("set-a"))
    );
    assert_eq!(
        sets(&fixture)
            .by_digest(&RuleSetDigest::parse(&"f".repeat(64)).expect("digest"))
            .expect("lookup"),
        None
    );
}

// ---------------------------------------------------------------------------
// The existing PDP, while the rule set is unwired
// ---------------------------------------------------------------------------

struct FixedDefault(PolicyTemplateName);

impl DefaultTemplatePort for FixedDefault {
    fn default_template(&self) -> PolicyTemplateName {
        self.0
    }
}

#[test]
fn the_existing_pdp_answers_exactly_as_it_did_while_the_rule_set_is_unwired() {
    // Migration 88 lands the store. Wiring it into evaluation -- the `NoMatch` fallthrough and the
    // trace -- belongs to the task group that owns the PDP. Until then the permissions answer must
    // be what it is today, and the way to prove that is to publish and activate a rule set that
    // would flip every answer, then check that no answer moved.
    let fixture = fixture("rules-pdp-baseline");
    let denies_everything = AuthorizationRule {
        matcher: Matcher::Any,
        ..rule("deny-all", RuleSource::User, RuleEffect::Deny)
    };
    record(
        &fixture,
        "set-deny-all",
        std::slice::from_ref(&denies_everything),
    );
    pointer(&fixture)
        .activate(&set_id("set-deny-all"), 0, AT)
        .expect("activate");

    let database = (*fixture.database).clone();
    let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
    let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
    let audit = Arc::new(SqliteAuditRepository::new(database));
    let service = EvaluationService::new(
        principals,
        grants,
        audit,
        Arc::new(PermissionsSystemClock),
        Arc::new(PermissionsUuidIdGenerator),
        Arc::new(FixedDefault(PolicyTemplateName::Standard)),
    );

    // `standard` asks for shell and allows reads. An active Deny-everything rule set changes
    // neither, because nothing consults it yet.
    assert_eq!(
        service.evaluate(
            "agent-1",
            Action::shell_exec(),
            Resource::workspace(),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Ask,
        "the template still decides shell.exec"
    );
    assert_eq!(
        service.evaluate(
            "agent-1",
            Action::file_read(),
            Resource::workspace(),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Allow,
        "and still allows reads -- a rule set that is not wired in cannot deny them"
    );
}
