//! The durable approval-resolution ledger.
//!
//! One thing distinguishes this adapter from the other repositories in this context: it is the
//! consistency boundary. `commit_resolution` writes three things — the immutable decision, its
//! audit row, and the remembered-grant intent — inside one explicit transaction, because the
//! previous code wrote the grant through one repository and the audit through another with nothing
//! joining them. A failure between those two calls left an action already released and no record
//! of why.
//!
//! The transaction never contains an external effect. Delivering the decision to a native Agent or
//! an HTTP waiter happens after the commit returns, which is the whole ordering the change exists
//! to establish: a rollback can undo rows, and nothing can undo a tool that already ran.

use super::grant_repository::{activate_grant_for_resolution_on, upsert_pending_grant_intent_on};
use crate::contexts::permissions::application::{
    ApprovalResolutionRepository, AuditDecider, AuditRecord, NewApprovalResolution,
    PermissionsApplicationError, ResolutionCommit,
};
use crate::contexts::permissions::domain::{
    ApprovalDecisionRecord, ApprovalResolution, ApprovalResolutionId, ApprovalResolutionState,
    Effect, PermissionsDomainError, ResolutionChannel, ResolutionDecider, RiskLevel, Scope,
    ALL_RESOLUTION_STATES,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, Connection, OptionalExtension, Row};

#[derive(Clone)]
pub(crate) struct SqliteApprovalResolutionRepository {
    database: NativeDatabase,
}

impl SqliteApprovalResolutionRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

const RESOLUTION_COLUMNS: &str = "id, request_id, principal_id, session_id, generation_id, \
                                  decision_effect, decision_scope, decider, channel, state, \
                                  delivery_attempts, last_error_code";

fn resolution_from_row(row: &Row<'_>) -> rusqlite::Result<ApprovalResolution> {
    let effect = match row.get::<_, String>(5)?.as_str() {
        "allow" => Effect::Allow,
        "deny" => Effect::Deny,
        other => {
            return Err(invalid_column(
                5,
                PermissionsDomainError::UnknownResolutionField(match other {
                    "ask" => "decision_effect",
                    _ => "decision_effect",
                }),
            ))
        }
    };
    let scope = match row.get::<_, String>(6)?.as_str() {
        "once" => Scope::Once,
        "session" => Scope::Session,
        "project" => Scope::Project,
        "global" => Scope::Global,
        _ => {
            return Err(invalid_column(
                6,
                PermissionsDomainError::UnknownResolutionField("decision_scope"),
            ))
        }
    };
    let decider = ResolutionDecider::from_token(&row.get::<_, String>(7)?)
        .map_err(|e| invalid_column(7, e))?;
    let channel = ResolutionChannel::from_token(&row.get::<_, String>(8)?)
        .map_err(|e| invalid_column(8, e))?;
    let state = ApprovalResolutionState::from_token(&row.get::<_, String>(9)?)
        .map_err(|e| invalid_column(9, e))?;

    Ok(ApprovalResolution {
        id: ApprovalResolutionId::parse(row.get::<_, String>(0)?)
            .map_err(|e| invalid_column(0, e))?,
        request_id: row.get(1)?,
        principal_id: row.get(2)?,
        session_id: row.get(3)?,
        generation_id: row.get(4)?,
        decision: ApprovalDecisionRecord::new(effect, scope, decider, channel)
            .map_err(|e| invalid_column(5, e))?,
        state,
        delivery_attempts: row.get(10)?,
        last_error_code: row.get(11)?,
    })
}

fn invalid_column(index: usize, error: PermissionsDomainError) -> rusqlite::Error {
    rusqlite::Error::InvalidColumnType(index, error.to_string(), rusqlite::types::Type::Text)
}

fn effect_token(effect: Effect) -> &'static str {
    match effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
        // Unreachable: `ApprovalDecisionRecord::new` refuses `Ask`, and the column's own CHECK
        // refuses the token. Written rather than panicked so the guard stays in one place.
        Effect::Ask => "ask",
    }
}

fn scope_token(scope: Scope) -> &'static str {
    match scope {
        Scope::Once => "once",
        Scope::Session => "session",
        Scope::Project => "project",
        Scope::Global => "global",
    }
}

fn audit_decider_token(decider: AuditDecider) -> &'static str {
    match decider {
        AuditDecider::Policy => "policy",
        AuditDecider::Human => "human",
        AuditDecider::Timeout => "timeout",
        AuditDecider::StaleGeneration => "stale_generation",
        AuditDecider::EmergencyFailClosed => "emergency_fail_closed",
        AuditDecider::EvaluationError => "evaluation_error",
    }
}

fn risk_level_token(risk_level: RiskLevel) -> &'static str {
    match risk_level {
        RiskLevel::L0 => "l0",
        RiskLevel::L1 => "l1",
        RiskLevel::L2 => "l2",
        RiskLevel::L3 => "l3",
    }
}

/// Appends one audit row on a caller-supplied connection.
///
/// Shared with `SqliteAuditRepository` so the transactional and non-transactional writers cannot
/// disagree about the column list — a second copy of this `INSERT` is how a new column ends up
/// populated on one path and null on the other.
pub(crate) fn append_audit_on(
    connection: &Connection,
    record: &AuditRecord,
) -> rusqlite::Result<()> {
    connection
        .execute(
            "INSERT INTO approval_audit \
             (id, principal_id, session_id, generation_id, action, resource, effect, \
              risk_level, decider, channel, created_at, resolution_id, outcome_reason) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                record.id,
                record.principal_id,
                record.session_id,
                record.generation_id,
                record.action.as_str(),
                record.resource.as_str(),
                effect_token(record.effect),
                risk_level_token(record.risk_level),
                audit_decider_token(record.decider),
                record.channel,
                record.created_at,
                record.resolution_id,
                record.outcome_reason,
            ],
        )
        .map(|_| ())
}

impl ApprovalResolutionRepository for SqliteApprovalResolutionRepository {
    fn commit_resolution(
        &self,
        commit: &ResolutionCommit,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        let resolution: &NewApprovalResolution = &commit.resolution;

        transaction
            .execute(
                "INSERT INTO approval_resolutions \
                 (id, request_id, principal_id, session_id, generation_id, call_id_hash, action, \
                  resource, risk_level, decision_effect, decision_scope, decider, channel, state, \
                  created_at, updated_at, delivery_attempts, last_error_code) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?15, 0, NULL)",
                params![
                    resolution.id.as_str(),
                    resolution.request_id,
                    resolution.principal_id,
                    resolution.session_id,
                    resolution.generation_id,
                    resolution.call_id_hash,
                    resolution.action.as_str(),
                    resolution.resource.as_str(),
                    risk_level_token(resolution.risk_level),
                    effect_token(resolution.decision.effect),
                    scope_token(resolution.decision.scope),
                    resolution.decision.decider.token(),
                    resolution.decision.channel.token(),
                    resolution.state.token(),
                    resolution.now,
                ],
            )
            .map_err(repository_error)?;

        append_audit_on(&transaction, &commit.audit).map_err(repository_error)?;

        if let Some(intent) = &commit.grant_intent {
            // Written inside the same transaction and left inactive. The row exists so an
            // acknowledgement has something to activate; until then it is recorded intent and
            // evaluation does not see it.
            upsert_pending_grant_intent_on(&transaction, intent).map_err(repository_error)?;
        }

        let committed = read_by_id(&transaction, &resolution.id).map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(committed)
    }

    fn find_by_request_id(
        &self,
        request_id: &str,
    ) -> Result<Option<ApprovalResolution>, PermissionsApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .query_row(
                &format!(
                    "SELECT {RESOLUTION_COLUMNS} FROM approval_resolutions WHERE request_id = ?1"
                ),
                params![request_id],
                resolution_from_row,
            )
            .optional()
            .map_err(repository_error)
    }

    fn record_delivery_failure(
        &self,
        id: &ApprovalResolutionId,
        error_code: &str,
        now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        // Guarded on the non-terminal states. A failure report arriving after the waiter already
        // acknowledged would otherwise walk a delivered decision back into a retryable one.
        connection
            .execute(
                &format!(
                    "UPDATE approval_resolutions \
                     SET state = '{failed}', delivery_attempts = delivery_attempts + 1, \
                         last_error_code = ?2, updated_at = ?3 \
                     WHERE id = ?1 AND state IN ({deliverable})",
                    failed = ApprovalResolutionState::DeliveryFailed.token(),
                    deliverable = deliverable_states(),
                ),
                params![id.as_str(), error_code, now],
            )
            .map_err(repository_error)?;
        read_by_id(&connection, id).map_err(repository_error)
    }

    fn acknowledge_delivery_and_activate(
        &self,
        id: &ApprovalResolutionId,
        now: &str,
    ) -> Result<ApprovalResolution, PermissionsApplicationError> {
        let mut connection = self.database.connection().map_err(repository_error)?;
        let transaction = connection.transaction().map_err(repository_error)?;
        transaction
            .execute(
                &format!(
                    "UPDATE approval_resolutions SET state = '{delivered}', updated_at = ?2 \
                     WHERE id = ?1 AND state IN ({deliverable})",
                    delivered = ApprovalResolutionState::Delivered.token(),
                    deliverable = deliverable_states(),
                ),
                params![id.as_str(), now],
            )
            .map_err(repository_error)?;
        // Activation is guarded on the grant's own pending state, so running this again after a
        // duplicate acknowledgement is a no-op rather than a second revision.
        activate_grant_for_resolution_on(&transaction, id.as_str(), now)
            .map_err(repository_error)?;
        let acknowledged = read_by_id(&transaction, id).map_err(repository_error)?;
        transaction.commit().map_err(repository_error)?;
        Ok(acknowledged)
    }

    fn mark_aborted_by_restart(&self, now: &str) -> Result<usize, PermissionsApplicationError> {
        // Deliberately does not touch grants. A resolution whose delivery was never acknowledged
        // leaves its intent inactive, which is the least-privilege reading of "we cannot tell
        // whether the agent ever received this".
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                &format!(
                    "UPDATE approval_resolutions SET state = '{aborted}', updated_at = ?1 \
                     WHERE state IN ({reconcilable})",
                    aborted = ApprovalResolutionState::AbortedByRestart.token(),
                    reconcilable =
                        state_list(ApprovalResolutionState::needs_restart_reconciliation),
                ),
                params![now],
            )
            .map_err(repository_error)
    }
}

/// A quoted, comma-separated SQL list of the states satisfying `predicate`.
///
/// Derived rather than written out, so a new state cannot be silently omitted from a `state IN
/// (...)` guard — which would make it either unreachable or, worse, quietly eligible for an update
/// that was never meant to apply to it.
fn state_list(predicate: fn(ApprovalResolutionState) -> bool) -> String {
    ALL_RESOLUTION_STATES
        .iter()
        .copied()
        .filter(|state| predicate(*state))
        .map(|state| format!("'{}'", state.token()))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The states a delivery attempt may still move: everything that is not terminal.
fn deliverable_states() -> String {
    state_list(|state| !state.is_terminal())
}

fn read_by_id(
    connection: &Connection,
    id: &ApprovalResolutionId,
) -> rusqlite::Result<ApprovalResolution> {
    connection.query_row(
        &format!("SELECT {RESOLUTION_COLUMNS} FROM approval_resolutions WHERE id = ?1"),
        params![id.as_str()],
        resolution_from_row,
    )
}

fn repository_error(error: impl std::fmt::Display) -> PermissionsApplicationError {
    PermissionsApplicationError::infrastructure("sqlite", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::application::{
        GrantQuery, GrantRepository, PendingGrantIntent,
    };
    use crate::contexts::permissions::domain::{
        Action, CanonicalGrantKey, PersistedEffect, RememberedScope, Resource,
    };
    use crate::contexts::permissions::infrastructure::grant_repository::SqliteGrantRepository;
    use crate::test_support::TempDirectory;

    struct Fixture {
        resolutions: SqliteApprovalResolutionRepository,
        grants: SqliteGrantRepository,
        database: NativeDatabase,
        _directory: TempDirectory,
    }

    fn fixture() -> Fixture {
        let directory = TempDirectory::new("permissions-resolution-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        database
            .connection()
            .expect("connection")
            .execute(
                "INSERT INTO agent_principals (id, agent_id, template_name, created_at, updated_at) \
                 VALUES ('principal-1', 'agent-1', 'standard', '0', '0')",
                [],
            )
            .expect("seed principal");
        Fixture {
            resolutions: SqliteApprovalResolutionRepository::new(database.clone()),
            grants: SqliteGrantRepository::new(database.clone()),
            database,
            _directory: directory,
        }
    }

    fn decision() -> ApprovalDecisionRecord {
        ApprovalDecisionRecord::new(
            Effect::Allow,
            Scope::Session,
            ResolutionDecider::Human,
            ResolutionChannel::NativeAgent,
        )
        .expect("well-formed decision")
    }

    fn new_resolution(id: &str, request_id: &str) -> NewApprovalResolution {
        NewApprovalResolution {
            id: ApprovalResolutionId::parse(id).expect("id"),
            request_id: request_id.to_string(),
            principal_id: "principal-1".to_string(),
            session_id: "session-1".to_string(),
            generation_id: "generation-1".to_string(),
            call_id_hash: "hash-1".to_string(),
            action: Action::file_write(),
            resource: Resource::file_path("a.txt"),
            risk_level: RiskLevel::L1,
            decision: decision(),
            state: ApprovalResolutionState::Committed,
            now: "10".to_string(),
        }
    }

    fn audit(id: &str, resolution_id: &str) -> AuditRecord {
        AuditRecord {
            id: id.to_string(),
            principal_id: "principal-1".to_string(),
            session_id: "session-1".to_string(),
            generation_id: "generation-1".to_string(),
            action: Action::file_write(),
            resource: Resource::file_path("a.txt"),
            effect: Effect::Allow,
            risk_level: RiskLevel::L1,
            decider: AuditDecider::Human,
            channel: "native_agent",
            resolution_id: Some(resolution_id.to_string()),
            outcome_reason: None,
            created_at: "10".to_string(),
        }
    }

    fn intent(id: &str, resolution_id: &str, principal_id: &str) -> PendingGrantIntent {
        PendingGrantIntent {
            id: id.to_string(),
            key: CanonicalGrantKey::new(
                principal_id,
                Action::file_write(),
                Resource::file_path("a.txt"),
                RememberedScope::Session("session-1".to_string()),
            )
            .expect("well-formed key"),
            effect: PersistedEffect::Allow,
            resolution_id: resolution_id.to_string(),
            now: "10".to_string(),
        }
    }

    fn commit(id: &str, request_id: &str, audit_id: &str, grant: bool) -> ResolutionCommit {
        ResolutionCommit {
            resolution: new_resolution(id, request_id),
            audit: audit(audit_id, id),
            grant_intent: grant.then(|| intent("grant-1", id, "principal-1")),
        }
    }

    fn counts(fixture: &Fixture) -> (i64, i64, i64) {
        let connection = fixture.database.connection().expect("connection");
        let resolutions = connection
            .query_row("SELECT COUNT(*) FROM approval_resolutions", [], |row| {
                row.get(0)
            })
            .expect("resolutions");
        let audits = connection
            .query_row("SELECT COUNT(*) FROM approval_audit", [], |row| row.get(0))
            .expect("audits");
        let grants = connection
            .query_row("SELECT COUNT(*) FROM permission_grants", [], |row| {
                row.get(0)
            })
            .expect("grants");
        (resolutions, audits, grants)
    }

    fn effective(fixture: &Fixture) -> Option<crate::contexts::permissions::domain::Grant> {
        let action = Action::file_write();
        let resource = Resource::file_path("a.txt");
        fixture
            .grants
            .find_effective_grant(&GrantQuery {
                principal_id: "principal-1",
                action: &action,
                resource: &resource,
                session_id: "session-1",
                project_key: "project-1",
            })
            .expect("query")
    }

    #[test]
    fn one_commit_writes_the_resolution_its_audit_and_an_inactive_grant_intent() {
        let fixture = fixture();

        let committed = fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");

        assert_eq!(committed.state, ApprovalResolutionState::Committed);
        assert_eq!(committed.delivery_attempts, 0);
        assert_eq!(counts(&fixture), (1, 1, 1));
        // The decision is durable and the grant exists, but nothing has been delivered yet — so
        // the grant must not be able to authorize the next evaluation.
        assert!(
            effective(&fixture).is_none(),
            "a committed but undelivered approval authorized the next evaluation"
        );
    }

    #[test]
    fn a_failed_audit_write_rolls_back_the_resolution_and_the_grant() {
        let fixture = fixture();
        // The audit id is the primary key, so reusing one is a deterministic failure at the second
        // statement of the transaction — no fault-injection scaffolding needed.
        fixture
            .database
            .connection()
            .expect("connection")
            .execute(
                "INSERT INTO approval_audit \
                 (id, principal_id, session_id, generation_id, action, resource, effect, \
                  risk_level, decider, channel, created_at) \
                 VALUES ('audit-1', 'principal-1', 's', 'g', 'file.write', 'a.txt', 'allow', \
                  'l1', 'human', 'native_agent', '0')",
                [],
            )
            .expect("pre-existing audit row");

        let failed = fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true));

        assert!(failed.is_err());
        // One audit row: the pre-existing one. No resolution, no grant.
        assert_eq!(counts(&fixture), (0, 1, 0));
    }

    #[test]
    fn a_failed_grant_write_rolls_back_the_resolution_and_the_audit() {
        let fixture = fixture();
        // The grant's principal foreign key does not resolve, which fails at the third statement.
        let mut broken = commit("res-1", "req-1", "audit-1", false);
        broken.grant_intent = Some(intent("grant-1", "res-1", "principal-does-not-exist"));

        let failed = fixture.resolutions.commit_resolution(&broken);

        assert!(failed.is_err());
        assert_eq!(counts(&fixture), (0, 0, 0));
        assert!(effective(&fixture).is_none());
    }

    #[test]
    fn a_second_resolution_for_one_request_is_refused_and_changes_nothing() {
        let fixture = fixture();
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("first commit");

        let second = fixture
            .resolutions
            .commit_resolution(&commit("res-2", "req-1", "audit-2", true));

        assert!(second.is_err(), "one request produced two resolutions");
        assert_eq!(counts(&fixture), (1, 1, 1));
        let existing = fixture
            .resolutions
            .find_by_request_id("req-1")
            .expect("lookup")
            .expect("the first resolution");
        assert_eq!(existing.id.as_str(), "res-1");
    }

    #[test]
    fn a_retry_finds_the_resolution_by_request_id_rather_than_a_not_found() {
        let fixture = fixture();
        assert!(fixture
            .resolutions
            .find_by_request_id("req-1")
            .expect("lookup")
            .is_none());

        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");

        let found = fixture
            .resolutions
            .find_by_request_id("req-1")
            .expect("lookup")
            .expect("the committed resolution");
        assert_eq!(found.decision, decision());
        assert_eq!(found.generation_id, "generation-1");
    }

    #[test]
    fn acknowledgement_delivers_the_resolution_and_activates_its_grant_exactly_once() {
        let fixture = fixture();
        let id = ApprovalResolutionId::parse("res-1").expect("id");
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");

        let delivered = fixture
            .resolutions
            .acknowledge_delivery_and_activate(&id, "20")
            .expect("acknowledge");
        assert_eq!(delivered.state, ApprovalResolutionState::Delivered);
        let grant = effective(&fixture).expect("the grant is now active");
        assert_eq!(grant.revision, 1);

        let repeated = fixture
            .resolutions
            .acknowledge_delivery_and_activate(&id, "30")
            .expect("duplicate acknowledgement");
        assert_eq!(repeated.state, ApprovalResolutionState::Delivered);
        assert_eq!(
            effective(&fixture).expect("still active").revision,
            1,
            "a duplicate acknowledgement produced a second grant revision"
        );
        assert_eq!(counts(&fixture), (1, 1, 1));
    }

    #[test]
    fn a_delivery_failure_is_durable_and_leaves_the_grant_inactive() {
        let fixture = fixture();
        let id = ApprovalResolutionId::parse("res-1").expect("id");
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");

        let first = fixture
            .resolutions
            .record_delivery_failure(&id, "waiter_unavailable", "20")
            .expect("record failure");
        assert_eq!(first.state, ApprovalResolutionState::DeliveryFailed);
        assert_eq!(first.delivery_attempts, 1);
        assert_eq!(first.last_error_code.as_deref(), Some("waiter_unavailable"));
        assert!(effective(&fixture).is_none());

        let second = fixture
            .resolutions
            .record_delivery_failure(&id, "waiter_unavailable", "30")
            .expect("record second failure");
        assert_eq!(second.delivery_attempts, 2);
        // The decision itself never moves. Only the delivery state does.
        assert_eq!(second.decision, decision());
    }

    #[test]
    fn a_delivery_failure_reported_after_acknowledgement_cannot_walk_it_back() {
        let fixture = fixture();
        let id = ApprovalResolutionId::parse("res-1").expect("id");
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");
        fixture
            .resolutions
            .acknowledge_delivery_and_activate(&id, "20")
            .expect("acknowledge");

        let late = fixture
            .resolutions
            .record_delivery_failure(&id, "transport_closed", "30")
            .expect("late failure report");

        assert_eq!(late.state, ApprovalResolutionState::Delivered);
        assert_eq!(late.delivery_attempts, 0);
        assert!(effective(&fixture).is_some());
    }

    #[test]
    fn a_failure_recorded_first_can_still_be_acknowledged_when_the_retry_lands() {
        let fixture = fixture();
        let id = ApprovalResolutionId::parse("res-1").expect("id");
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");
        fixture
            .resolutions
            .record_delivery_failure(&id, "waiter_unavailable", "20")
            .expect("record failure");

        let delivered = fixture
            .resolutions
            .acknowledge_delivery_and_activate(&id, "30")
            .expect("retry acknowledged");

        assert_eq!(delivered.state, ApprovalResolutionState::Delivered);
        assert!(effective(&fixture).is_some());
    }

    #[test]
    fn restart_reconciliation_aborts_undelivered_rows_and_leaves_their_grants_inactive() {
        let fixture = fixture();
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");
        fixture
            .resolutions
            .record_delivery_failure(
                &ApprovalResolutionId::parse("res-1").expect("id"),
                "waiter_unavailable",
                "20",
            )
            .expect("record failure");

        let reconciled = fixture
            .resolutions
            .mark_aborted_by_restart("40")
            .expect("reconcile");

        assert_eq!(reconciled, 1);
        let row = fixture
            .resolutions
            .find_by_request_id("req-1")
            .expect("lookup")
            .expect("still durable evidence");
        assert_eq!(row.state, ApprovalResolutionState::AbortedByRestart);
        // Least privilege: an approval whose delivery was never confirmed does not authorize the
        // next attempt, so the grant stays where the commit left it.
        assert!(effective(&fixture).is_none());
    }

    #[test]
    fn restart_reconciliation_leaves_an_already_delivered_resolution_alone() {
        let fixture = fixture();
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");
        fixture
            .resolutions
            .acknowledge_delivery_and_activate(
                &ApprovalResolutionId::parse("res-1").expect("id"),
                "20",
            )
            .expect("acknowledge");

        assert_eq!(
            fixture
                .resolutions
                .mark_aborted_by_restart("40")
                .expect("reconcile"),
            0
        );
        assert!(effective(&fixture).is_some());
    }

    #[test]
    fn a_stale_resolution_commits_its_evidence_without_a_grant() {
        let fixture = fixture();
        let mut stale = commit("res-1", "req-1", "audit-1", false);
        stale.resolution.state = ApprovalResolutionState::Stale;
        stale.resolution.decision = ApprovalDecisionRecord::new(
            Effect::Allow,
            Scope::Session,
            ResolutionDecider::StaleGeneration,
            ResolutionChannel::NativeAgent,
        )
        .expect("stale decision");

        let committed = fixture
            .resolutions
            .commit_resolution(&stale)
            .expect("commit");

        assert_eq!(committed.state, ApprovalResolutionState::Stale);
        assert!(committed.state.is_terminal());
        assert_eq!(counts(&fixture), (1, 1, 0));
        // A terminal row is never reconciled at startup and never delivered.
        assert_eq!(
            fixture
                .resolutions
                .mark_aborted_by_restart("40")
                .expect("reconcile"),
            0
        );
    }

    #[test]
    fn the_audit_row_carries_the_resolution_it_belongs_to() {
        let fixture = fixture();
        fixture
            .resolutions
            .commit_resolution(&commit("res-1", "req-1", "audit-1", true))
            .expect("commit");

        let (resolution_id, outcome_reason): (Option<String>, Option<String>) = fixture
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT resolution_id, outcome_reason FROM approval_audit WHERE id = 'audit-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("audit row");
        assert_eq!(resolution_id.as_deref(), Some("res-1"));
        assert_eq!(outcome_reason, None);
    }

    #[test]
    fn nothing_the_ledger_stores_carries_the_providers_own_call_id() {
        // The correlation hash is enough to match a delivery to its waiter; the raw call id is a
        // provider-chosen string that can carry request content, and this table is read by
        // operators.
        let fixture = fixture();
        let mut with_hash = commit("res-1", "req-1", "audit-1", false);
        with_hash.resolution.call_id_hash = "sha256:abcdef".to_string();
        fixture
            .resolutions
            .commit_resolution(&with_hash)
            .expect("commit");

        let stored: String = fixture
            .database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT call_id_hash FROM approval_resolutions WHERE id = 'res-1'",
                [],
                |row| row.get(0),
            )
            .expect("resolution row");
        assert_eq!(stored, "sha256:abcdef");
    }
}
