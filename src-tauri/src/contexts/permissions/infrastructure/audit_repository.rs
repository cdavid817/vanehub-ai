use super::resolution_repository::append_audit_on;
use crate::contexts::permissions::application::{
    AuditRecord, AuditRepository, PermissionsApplicationError,
};
use crate::platform::database::NativeDatabase;

#[derive(Clone)]
pub(crate) struct SqliteAuditRepository {
    database: NativeDatabase,
}

impl SqliteAuditRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl AuditRepository for SqliteAuditRepository {
    /// Appends outside any transaction, for the evaluation path — the decision the engine made on
    /// its own, with nobody waiting on it. Decisions that resolve a pending approval go through
    /// `commit_resolution` instead, which writes the same row inside the transaction that carries
    /// the resolution. Both call one `INSERT`, so a new column cannot end up populated on one path
    /// and null on the other.
    fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError> {
        let connection = self.database.connection().map_err(repository_error)?;
        append_audit_on(&connection, &record).map_err(repository_error)
    }
}

fn repository_error(error: impl std::fmt::Display) -> PermissionsApplicationError {
    PermissionsApplicationError::infrastructure("sqlite", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::application::AuditDecider;
    use crate::contexts::permissions::domain::{Action, Effect, Resource, RiskLevel};
    use crate::test_support::TempDirectory;

    fn repository() -> (SqliteAuditRepository, TempDirectory) {
        let directory = TempDirectory::new("permissions-audit-repository");
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
        (SqliteAuditRepository::new(database), directory)
    }

    #[test]
    fn append_persists_every_field() {
        let (repository, directory) = repository();
        repository
            .append(AuditRecord {
                id: "audit-1".to_string(),
                principal_id: "principal-1".to_string(),
                session_id: "session-1".to_string(),
                generation_id: "generation-1".to_string(),
                action: Action::shell_exec(),
                resource: Resource::workspace(),
                effect: Effect::Deny,
                risk_level: RiskLevel::L2,
                decider: AuditDecider::Timeout,
                channel: "native_agent",
                resolution_id: None,
                outcome_reason: None,
                created_at: "100".to_string(),
            })
            .unwrap();

        let database = NativeDatabase::new(directory.path().to_path_buf()).unwrap();
        let (effect, decider, risk_level): (String, String, String) = database
            .connection()
            .unwrap()
            .query_row(
                "SELECT effect, decider, risk_level FROM approval_audit WHERE id = 'audit-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(effect, "deny");
        assert_eq!(decider, "timeout");
        assert_eq!(risk_level, "l2");
    }
}
