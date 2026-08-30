//! End-to-end migration-equivalence regression (tasks.md group 9; `permissions-core`'s
//! "Previously trusted agent keeps its effective behavior after migration"): runs the *real*
//! migration against a *real* SQLite database — not fakes — starting from a pre-migration-44
//! state with `auto_approve_tools = 1` already set, then evaluates through the real repositories.
//! (Renumbered from 42 to 44 when merging with main: `agent-memory-shared-pool` and
//! `retrieval-vector-index` independently claimed 42/43 first.)
//! This is the actual acceptance test for the migration, not just a unit test of the backfill SQL
//! in isolation (`schema`'s own tests) or of template behavior in isolation
//! (`evaluation_service`'s fake-backed tests) — it proves the two compose correctly.
//!
//! Lives here rather than in `application::evaluation_service` because wiring a real
//! `EvaluationService` to real SQLite repositories is infrastructure-level composition;
//! `tests/architecture.rs` enforces that `application` code cannot depend on `infrastructure`.

use super::{
    PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
    SqliteGrantRepository, SqlitePrincipalRepository, UnifiedLogDiagnosticsAdapter,
};
use crate::contexts::permissions::application::{DefaultTemplatePort, EvaluationService};
use crate::contexts::permissions::domain::{Action, Effect, PolicyTemplateName, Resource};
use crate::platform::database::{migrate, NativeDatabase};
use crate::test_support::TempDirectory;
use std::sync::Arc;

/// This migration-equivalence suite only cares about the legacy `auto_approve_tools` backfill,
/// not the newer configurable-default feature — always `Standard`, matching the pre-existing
/// hardcoded behavior these tests were originally written against.
struct FixedStandardDefault;
impl DefaultTemplatePort for FixedStandardDefault {
    fn default_template(&self) -> PolicyTemplateName {
        PolicyTemplateName::Standard
    }
}

fn migrated_service(temp_label: &str, trusted: bool) -> EvaluationService {
    let directory = TempDirectory::new(temp_label);
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    {
        let connection = database.connection().expect("connection");
        if trusted {
            connection
                .execute(
                    "UPDATE agents SET auto_approve_tools = 1 WHERE id = 'onepiece'",
                    [],
                )
                .expect("mark onepiece trusted pre-migration");
        }
        // Both versions are withdrawn, not just 44. Migration 111 rebuilds `permission_grants`
        // under canonical identity, so replaying 44 alone would recreate the pre-111 table and
        // leave the reader querying columns that are not there — an upgrade path no real database
        // ever takes, failing in a way that looks like a policy regression.
        connection
            .execute_batch(
                "DELETE FROM schema_migrations WHERE version IN (44, 111);
                 DROP TABLE agent_principals;
                 DROP TABLE permission_grants;
                 DROP TABLE approval_audit;
                 DROP TABLE IF EXISTS approval_resolutions;",
            )
            .expect("simulate pre-migration-44 schema");
        migrate(&connection).expect("re-run migrations 44 and 111 against the trust-flag fixture");
    }

    EvaluationService::new(
        Arc::new(SqlitePrincipalRepository::new(database.clone())),
        Arc::new(SqliteGrantRepository::new(database.clone())),
        Arc::new(SqliteAuditRepository::new(database)),
        Arc::new(PermissionsSystemClock),
        Arc::new(PermissionsUuidIdGenerator),
        Arc::new(FixedStandardDefault),
        Arc::new(UnifiedLogDiagnosticsAdapter),
    )
}

#[test]
fn migration_preserves_a_previously_trusted_agents_effective_behavior() {
    let service = migrated_service("permissions-migration-trusted", true);

    assert_eq!(
        service.evaluate(
            "onepiece",
            Action::shell_exec(),
            Resource::workspace(),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Allow
    );
    assert_eq!(
        service.evaluate(
            "onepiece",
            Action::file_write(),
            Resource::file_path("a.txt"),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Allow
    );
    assert_eq!(
        service.evaluate(
            "onepiece",
            Action::mcp_tool(),
            Resource::mcp_tool("server", "tool"),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Ask,
        "the MCP floor must survive migration even for a previously trusted agent"
    );
}

#[test]
fn migration_leaves_a_previously_untrusted_agent_unaffected() {
    let service = migrated_service("permissions-migration-untrusted", false);

    assert_eq!(
        service.evaluate(
            "onepiece",
            Action::shell_exec(),
            Resource::workspace(),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Ask
    );
    assert_eq!(
        service.evaluate(
            "onepiece",
            Action::file_write(),
            Resource::file_path("a.txt"),
            "session-1",
            "generation-1",
            "project-1",
        ),
        Effect::Ask
    );
}
