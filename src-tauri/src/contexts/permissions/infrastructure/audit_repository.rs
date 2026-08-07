use crate::contexts::permissions::application::{
    AuditDecider, AuditRecord, AuditRepository, PermissionsApplicationError,
};
use crate::contexts::permissions::domain::{Effect, RiskLevel};
use crate::platform::database::NativeDatabase;
use rusqlite::params;

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
    fn append(&self, record: AuditRecord) -> Result<(), PermissionsApplicationError> {
        self.database
            .connection()
            .map_err(repository_error)?
            .execute(
                "INSERT INTO approval_audit \
                 (id, principal_id, session_id, generation_id, action, resource, effect, \
                  risk_level, decider, channel, created_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    record.id,
                    record.principal_id,
                    record.session_id,
                    record.generation_id,
                    record.action.as_str(),
                    record.resource.as_str(),
                    effect_to_str(record.effect),
                    risk_level_to_str(record.risk_level),
                    decider_to_str(record.decider),
                    record.channel,
                    record.created_at,
                ],
            )
            .map(|_| ())
            .map_err(repository_error)
    }
}

fn effect_to_str(effect: Effect) -> &'static str {
    match effect {
        Effect::Allow => "allow",
        Effect::Deny => "deny",
        Effect::Ask => "ask",
    }
}

fn risk_level_to_str(risk_level: RiskLevel) -> &'static str {
    match risk_level {
        RiskLevel::L0 => "l0",
        RiskLevel::L1 => "l1",
        RiskLevel::L2 => "l2",
        RiskLevel::L3 => "l3",
    }
}

fn decider_to_str(decider: AuditDecider) -> &'static str {
    match decider {
        AuditDecider::Policy => "policy",
        AuditDecider::Human => "human",
        AuditDecider::Timeout => "timeout",
        AuditDecider::StaleGeneration => "stale_generation",
    }
}

fn repository_error(error: impl std::fmt::Display) -> PermissionsApplicationError {
    PermissionsApplicationError::infrastructure("sqlite", error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::domain::{Action, Resource};
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
