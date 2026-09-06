//! SQLite persistence for managed worktree records, their session bindings, and use gates.
//!
//! Bindings carry no foreign key to `sessions` on purpose: a session's deletion must not cascade
//! into forgetting that a directory was ours, and the retention of the record after its
//! sessions are gone is the whole point of tracking it separately.

use crate::contexts::workspaces::application::{
    GateClaim, GateHolder, GateOwner, GateRejection, ManagedWorktreeRepository,
    WorkspaceApplicationError, WorktreeCleanupClockPort, WorktreeIdPort, WorktreeUseGatePort,
};
use crate::contexts::workspaces::domain::{
    ManagedWorktree, ManagedWorktreeStatus, WorktreeIdentity, WorktreeOrigin, WorktreeProvenance,
};
use crate::platform::database::{DatabaseError, NativeDatabase, PooledSqlite};
use crate::platform::instance_lease::InstanceLease;
use rusqlite::{params, Connection, OptionalExtension, Row, TransactionBehavior};

pub(crate) fn apply_managed_worktree_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS managed_worktrees (
            id TEXT PRIMARY KEY,
            origin TEXT NOT NULL,
            provenance TEXT NOT NULL,
            status TEXT NOT NULL,
            requested_root TEXT NOT NULL,
            project_root TEXT NOT NULL,
            canonical_root TEXT,
            git_dir TEXT,
            common_dir TEXT,
            branch TEXT,
            head TEXT,
            fs_identity TEXT,
            creation_operation_id TEXT,
            attention_reason TEXT,
            revision INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_managed_worktrees_canonical_root
            ON managed_worktrees(canonical_root);
        CREATE INDEX IF NOT EXISTS idx_managed_worktrees_requested_root
            ON managed_worktrees(requested_root);
        CREATE TABLE IF NOT EXISTS managed_worktree_sessions (
            worktree_id TEXT NOT NULL REFERENCES managed_worktrees(id),
            session_id TEXT NOT NULL,
            binding_kind TEXT NOT NULL,
            bound_at TEXT NOT NULL,
            PRIMARY KEY (worktree_id, session_id)
        );
        CREATE INDEX IF NOT EXISTS idx_managed_worktree_sessions_session
            ON managed_worktree_sessions(session_id);
        CREATE TABLE IF NOT EXISTS workspace_use_gates (
            worktree_id TEXT PRIMARY KEY,
            canonical_root TEXT NOT NULL,
            owner_instance_id TEXT NOT NULL,
            owner_epoch INTEGER NOT NULL,
            operation_id TEXT NOT NULL,
            claimed_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_workspace_use_gates_root
            ON workspace_use_gates(canonical_root);
        "#,
    )?;
    Ok(())
}

const SELECT_COLUMNS: &str = "id, origin, provenance, status, requested_root, project_root, canonical_root, git_dir, common_dir, branch, head, fs_identity, creation_operation_id, attention_reason, revision, created_at, updated_at";

#[derive(Clone)]
pub(crate) struct SqliteManagedWorktreeRepository {
    database: NativeDatabase,
    clock: std::sync::Arc<dyn WorktreeCleanupClockPort>,
}

impl SqliteManagedWorktreeRepository {
    pub(crate) fn new(
        database: NativeDatabase,
        clock: std::sync::Arc<dyn WorktreeCleanupClockPort>,
    ) -> Self {
        Self { database, clock }
    }

    fn connection(&self) -> Result<PooledSqlite, WorkspaceApplicationError> {
        self.database.connection().map_err(app_error)
    }
}

fn read_record(row: &Row<'_>) -> rusqlite::Result<RecordRow> {
    Ok(RecordRow {
        id: row.get(0)?,
        origin: row.get(1)?,
        provenance: row.get(2)?,
        status: row.get(3)?,
        requested_root: row.get(4)?,
        project_root: row.get(5)?,
        canonical_root: row.get(6)?,
        git_dir: row.get(7)?,
        common_dir: row.get(8)?,
        branch: row.get(9)?,
        head: row.get(10)?,
        fs_identity: row.get(11)?,
        creation_operation_id: row.get(12)?,
        attention_reason: row.get(13)?,
        revision: row.get(14)?,
        created_at: row.get(15)?,
        updated_at: row.get(16)?,
    })
}

struct RecordRow {
    id: String,
    origin: String,
    provenance: String,
    status: String,
    requested_root: String,
    project_root: String,
    canonical_root: Option<String>,
    git_dir: Option<String>,
    common_dir: Option<String>,
    branch: Option<String>,
    head: Option<String>,
    fs_identity: Option<String>,
    creation_operation_id: Option<String>,
    attention_reason: Option<String>,
    revision: i64,
    created_at: String,
    updated_at: String,
}

impl RecordRow {
    fn into_record(self) -> Result<ManagedWorktree, WorkspaceApplicationError> {
        let identity = match (self.canonical_root, self.git_dir, self.common_dir) {
            (Some(canonical_root), Some(git_dir), Some(common_dir)) => Some(WorktreeIdentity {
                canonical_root,
                git_dir,
                common_dir,
                branch: self.branch,
                head: self.head,
                fs_identity: self.fs_identity,
            }),
            _ => None,
        };
        Ok(ManagedWorktree {
            id: self.id,
            origin: WorktreeOrigin::parse(&self.origin)?,
            provenance: WorktreeProvenance::parse(&self.provenance)?,
            status: ManagedWorktreeStatus::parse(&self.status)?,
            requested_root: self.requested_root,
            project_root: self.project_root,
            identity,
            creation_operation_id: self.creation_operation_id,
            attention_reason: self.attention_reason,
            revision: u64::try_from(self.revision).unwrap_or(0),
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

fn query_one(
    connection: &Connection,
    condition: &str,
    parameter: &str,
) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError> {
    connection
        .query_row(
            &format!("SELECT {SELECT_COLUMNS} FROM managed_worktrees WHERE {condition} LIMIT 1"),
            [parameter],
            read_record,
        )
        .optional()
        .map_err(database_error)?
        .map(RecordRow::into_record)
        .transpose()
}

impl ManagedWorktreeRepository for SqliteManagedWorktreeRepository {
    fn insert(&self, record: &ManagedWorktree) -> Result<(), WorkspaceApplicationError> {
        let identity = record.identity.as_ref();
        self.connection()?
            .execute(
                &format!("INSERT INTO managed_worktrees ({SELECT_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)"),
                params![
                    record.id,
                    record.origin.as_str(),
                    record.provenance.as_str(),
                    record.status.as_str(),
                    record.requested_root,
                    record.project_root,
                    identity.map(|identity| identity.canonical_root.clone()),
                    identity.map(|identity| identity.git_dir.clone()),
                    identity.map(|identity| identity.common_dir.clone()),
                    identity.and_then(|identity| identity.branch.clone()),
                    identity.and_then(|identity| identity.head.clone()),
                    identity.and_then(|identity| identity.fs_identity.clone()),
                    record.creation_operation_id,
                    record.attention_reason,
                    i64::try_from(record.revision).unwrap_or(i64::MAX),
                    record.created_at,
                    record.updated_at,
                ],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn find(&self, id: &str) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError> {
        query_one(&*self.connection()?, "id = ?1", id)
    }

    fn find_by_root(
        &self,
        root: &str,
    ) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError> {
        let connection = self.connection()?;
        if let Some(record) = query_one(&connection, "canonical_root = ?1", root)? {
            return Ok(Some(record));
        }
        query_one(&connection, "requested_root = ?1", root)
    }

    fn save(
        &self,
        record: &ManagedWorktree,
        expected_revision: u64,
    ) -> Result<bool, WorkspaceApplicationError> {
        let identity = record.identity.as_ref();
        let changed = self
            .connection()?
            .execute(
                "UPDATE managed_worktrees SET origin = ?2, provenance = ?3, status = ?4, requested_root = ?5, project_root = ?6, canonical_root = ?7, git_dir = ?8, common_dir = ?9, branch = ?10, head = ?11, fs_identity = ?12, creation_operation_id = ?13, attention_reason = ?14, revision = ?15, updated_at = ?16 WHERE id = ?1 AND revision = ?17",
                params![
                    record.id,
                    record.origin.as_str(),
                    record.provenance.as_str(),
                    record.status.as_str(),
                    record.requested_root,
                    record.project_root,
                    identity.map(|identity| identity.canonical_root.clone()),
                    identity.map(|identity| identity.git_dir.clone()),
                    identity.map(|identity| identity.common_dir.clone()),
                    identity.and_then(|identity| identity.branch.clone()),
                    identity.and_then(|identity| identity.head.clone()),
                    identity.and_then(|identity| identity.fs_identity.clone()),
                    record.creation_operation_id,
                    record.attention_reason,
                    i64::try_from(record.revision).unwrap_or(i64::MAX),
                    record.updated_at,
                    i64::try_from(expected_revision).unwrap_or(i64::MAX),
                ],
            )
            .map_err(database_error)?;
        Ok(changed == 1)
    }

    fn bind_session(
        &self,
        worktree_id: &str,
        session_id: &str,
        binding_kind: &str,
    ) -> Result<(), WorkspaceApplicationError> {
        self.connection()?
            .execute(
                "INSERT OR REPLACE INTO managed_worktree_sessions (worktree_id, session_id, binding_kind, bound_at) VALUES (?1, ?2, ?3, ?4)",
                params![worktree_id, session_id, binding_kind, self.clock.now()],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn unbind_sessions(
        &self,
        worktree_id: &str,
        session_ids: &[String],
    ) -> Result<(), WorkspaceApplicationError> {
        let connection = self.connection()?;
        for session_id in session_ids {
            connection
                .execute(
                    "DELETE FROM managed_worktree_sessions WHERE worktree_id = ?1 AND session_id = ?2",
                    params![worktree_id, session_id],
                )
                .map_err(database_error)?;
        }
        Ok(())
    }

    fn bound_sessions(&self, worktree_id: &str) -> Result<Vec<String>, WorkspaceApplicationError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT session_id FROM managed_worktree_sessions WHERE worktree_id = ?1 ORDER BY bound_at")
            .map_err(database_error)?;
        let ids = statement
            .query_map([worktree_id], |row| row.get::<_, String>(0))
            .map_err(database_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(database_error)?;
        Ok(ids)
    }

    fn find_by_session(
        &self,
        session_id: &str,
    ) -> Result<Option<ManagedWorktree>, WorkspaceApplicationError> {
        let connection = self.connection()?;
        let worktree_id: Option<String> = connection
            .query_row(
                "SELECT worktree_id FROM managed_worktree_sessions WHERE session_id = ?1 ORDER BY bound_at DESC LIMIT 1",
                [session_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(database_error)?;
        match worktree_id {
            Some(worktree_id) => query_one(&connection, "id = ?1", &worktree_id),
            None => Ok(None),
        }
    }
}

/// Gate rows plus the OS-level proof of who is alive.
///
/// A row alone is a claim; the lease is what makes "the holder is gone" a fact rather than a
/// guess about a timestamp. A row whose owner cannot be shown alive *or* dead is treated as held.
#[derive(Clone)]
pub(crate) struct SqliteWorkspaceUseGate {
    database: NativeDatabase,
    lease: InstanceLease,
    clock: std::sync::Arc<dyn WorktreeCleanupClockPort>,
}

impl SqliteWorkspaceUseGate {
    pub(crate) fn new(
        database: NativeDatabase,
        lease: InstanceLease,
        clock: std::sync::Arc<dyn WorktreeCleanupClockPort>,
    ) -> Self {
        Self {
            database,
            lease,
            clock,
        }
    }

    pub(crate) fn lease(&self) -> &InstanceLease {
        &self.lease
    }

    fn read_holder(row: &Row<'_>) -> rusqlite::Result<GateHolder> {
        Ok(GateHolder {
            worktree_id: row.get(0)?,
            owner: GateOwner {
                instance_id: row.get(2)?,
                epoch: u64::try_from(row.get::<_, i64>(3)?).unwrap_or(0),
                operation_id: row.get(4)?,
            },
            claimed_at: row.get(5)?,
        })
    }
}

const GATE_COLUMNS: &str =
    "worktree_id, canonical_root, owner_instance_id, owner_epoch, operation_id, claimed_at";

impl WorktreeUseGatePort for SqliteWorkspaceUseGate {
    fn claim(
        &self,
        worktree_id: &str,
        canonical_root: &str,
        owner: &GateOwner,
    ) -> Result<GateClaim, GateRejection> {
        let mut connection = self
            .database
            .connection()
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        // Immediate, not deferred: the claim reads the gate and then writes it, and a deferred
        // transaction that upgrades while another connection has written in between gets
        // `SQLITE_BUSY` at once, without the busy timeout ever being consulted.
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        let existing = transaction
            .query_row(
                &format!("SELECT {GATE_COLUMNS} FROM workspace_use_gates WHERE worktree_id = ?1 OR canonical_root = ?2 LIMIT 1"),
                params![worktree_id, canonical_root],
                Self::read_holder,
            )
            .optional()
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        if let Some(holder) = existing {
            let same_owner = holder.owner.instance_id == owner.instance_id
                && holder.owner.operation_id == owner.operation_id;
            let alive = same_owner
                || self
                    .lease
                    .is_alive(&holder.owner.instance_id)
                    .unwrap_or(true);
            if alive && !same_owner {
                return Err(GateRejection::Held(holder));
            }
            transaction
                .execute(
                    "DELETE FROM workspace_use_gates WHERE worktree_id = ?1",
                    [holder.worktree_id],
                )
                .map_err(|error| GateRejection::Storage(error.to_string()))?;
        }
        let claimed_at = self.clock.now();
        transaction
            .execute(
                &format!("INSERT INTO workspace_use_gates ({GATE_COLUMNS}) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"),
                params![
                    worktree_id,
                    canonical_root,
                    owner.instance_id,
                    i64::try_from(owner.epoch).unwrap_or(i64::MAX),
                    owner.operation_id,
                    claimed_at,
                ],
            )
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        transaction
            .commit()
            .map_err(|error| GateRejection::Storage(error.to_string()))?;
        Ok(GateClaim {
            worktree_id: worktree_id.to_string(),
            canonical_root: canonical_root.to_string(),
            owner: owner.clone(),
            claimed_at,
        })
    }

    fn release(&self, claim: &GateClaim) -> Result<(), WorkspaceApplicationError> {
        self.database
            .connection()
            .map_err(app_error)?
            .execute(
                "DELETE FROM workspace_use_gates WHERE worktree_id = ?1 AND owner_instance_id = ?2 AND operation_id = ?3",
                params![claim.worktree_id, claim.owner.instance_id, claim.owner.operation_id],
            )
            .map_err(database_error)?;
        Ok(())
    }

    fn holder_for_root(
        &self,
        canonical_root: &str,
    ) -> Result<Option<GateHolder>, WorkspaceApplicationError> {
        self.database
            .connection()
            .map_err(app_error)?
            .query_row(
                &format!("SELECT {GATE_COLUMNS} FROM workspace_use_gates WHERE canonical_root = ?1 LIMIT 1"),
                [canonical_root],
                Self::read_holder,
            )
            .optional()
            .map_err(database_error)
    }

    fn owner_is_alive(&self, holder: &GateHolder) -> Result<bool, WorkspaceApplicationError> {
        self.lease.is_alive(&holder.owner.instance_id).map_err(|_| {
            WorkspaceApplicationError::Storage("instance lock is unavailable".to_string())
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidWorktreeIds;

impl WorktreeIdPort for UuidWorktreeIds {
    fn next_worktree_id(&self) -> String {
        format!("wt-{}", uuid::Uuid::new_v4())
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemWorktreeCleanupClock;

impl WorktreeCleanupClockPort for SystemWorktreeCleanupClock {
    fn now(&self) -> String {
        crate::platform::clock::SystemClock.rfc3339()
    }
}

fn app_error(error: DatabaseError) -> WorkspaceApplicationError {
    WorkspaceApplicationError::Repository(error.to_string())
}

fn database_error(error: rusqlite::Error) -> WorkspaceApplicationError {
    WorkspaceApplicationError::Repository(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::TempDirectory;
    use std::sync::Arc;

    fn fixture(label: &str) -> (TempDirectory, NativeDatabase) {
        let directory = TempDirectory::new(label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (directory, database)
    }

    fn record(id: &str, root: &str) -> ManagedWorktree {
        ManagedWorktree::provisioning(
            id.to_string(),
            WorktreeOrigin::OrdinarySession,
            "/repo".to_string(),
            root.to_string(),
            Some("op".to_string()),
            "2026-09-05T00:00:00Z".to_string(),
        )
        .expect("record")
    }

    #[test]
    fn records_round_trip_with_compare_and_set_revisions() {
        let (_directory, database) = fixture("managed-worktrees");
        let repository =
            SqliteManagedWorktreeRepository::new(database, Arc::new(SystemWorktreeCleanupClock));
        let mut first = record("wt-1", "/repo-a");
        repository.insert(&first).expect("insert");
        assert_eq!(repository.find("wt-1").expect("find"), Some(first.clone()));
        assert_eq!(
            repository.find_by_root("/repo-a").expect("by root"),
            Some(first.clone())
        );
        first
            .confirm_created(
                WorktreeIdentity {
                    canonical_root: "/canon/repo-a".to_string(),
                    git_dir: "/repo/.git/worktrees/repo-a".to_string(),
                    common_dir: "/repo/.git".to_string(),
                    branch: Some("vanehub/a".to_string()),
                    head: Some("abc".to_string()),
                    fs_identity: Some("1:2".to_string()),
                },
                "t1".to_string(),
            )
            .expect("confirm");
        assert!(repository.save(&first, 1).expect("save"));
        assert!(!repository.save(&first, 1).expect("stale save"));
        assert_eq!(
            repository.find_by_root("/canon/repo-a").expect("canonical"),
            Some(first.clone())
        );
        repository
            .bind_session("wt-1", "session-1", "owner")
            .expect("bind");
        assert_eq!(
            repository.find_by_session("session-1").expect("by session"),
            Some(first.clone())
        );
        assert_eq!(
            repository.bound_sessions("wt-1").expect("bound"),
            vec!["session-1"]
        );
        repository
            .unbind_sessions("wt-1", &["session-1".to_string()])
            .expect("unbind");
        assert!(repository.bound_sessions("wt-1").expect("bound").is_empty());
        // The record outlives the binding.
        assert!(repository.find("wt-1").expect("find").is_some());
    }

    #[test]
    fn gates_are_exclusive_per_root_and_released_by_their_owner_only() {
        let (directory, database) = fixture("use-gates");
        let lease = InstanceLease::acquire(directory.path()).expect("lease");
        let gate = SqliteWorkspaceUseGate::new(
            database,
            lease.clone(),
            Arc::new(SystemWorktreeCleanupClock),
        );
        let owner = GateOwner {
            instance_id: lease.id().to_string(),
            epoch: lease.epoch(),
            operation_id: "op-1".to_string(),
        };
        let claim = gate.claim("wt-1", "/canon/a", &owner).expect("claim");
        let other = GateOwner {
            operation_id: "op-2".to_string(),
            ..owner.clone()
        };
        assert!(matches!(
            gate.claim("wt-1", "/canon/a", &other),
            Err(GateRejection::Held(holder)) if holder.owner.operation_id == "op-1"
        ));
        assert!(gate.holder_for_root("/canon/a").expect("holder").is_some());
        let foreign = GateClaim {
            owner: other,
            ..claim.clone()
        };
        gate.release(&foreign).expect("foreign release is a no-op");
        assert!(gate.holder_for_root("/canon/a").expect("holder").is_some());
        gate.release(&claim).expect("release");
        assert!(gate.holder_for_root("/canon/a").expect("holder").is_none());
    }

    #[test]
    fn a_gate_held_by_a_dead_instance_can_be_taken_over() {
        let (directory, database) = fixture("use-gates-dead");
        let lease = InstanceLease::acquire(directory.path()).expect("lease");
        let gate = SqliteWorkspaceUseGate::new(
            database.clone(),
            lease.clone(),
            Arc::new(SystemWorktreeCleanupClock),
        );
        database
            .connection()
            .expect("connection")
            .execute(
                &format!("INSERT INTO workspace_use_gates ({GATE_COLUMNS}) VALUES ('wt-x', '/canon/x', 'dead-instance', 1, 'op-old', 't0')"),
                [],
            )
            .expect("seed");
        let holder = gate
            .holder_for_root("/canon/x")
            .expect("holder")
            .expect("seeded");
        assert_eq!(gate.owner_is_alive(&holder), Ok(false));
        let owner = GateOwner {
            instance_id: lease.id().to_string(),
            epoch: lease.epoch(),
            operation_id: "op-new".to_string(),
        };
        gate.claim("wt-x", "/canon/x", &owner).expect("take over");
    }
}
