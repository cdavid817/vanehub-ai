use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::permissions::application::{ApprovalBroker, EvaluationService};
use crate::contexts::permissions::infrastructure::{
    PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
    SqliteGrantRepository, SqlitePrincipalRepository,
};
use crate::platform::database::NativeDatabase;
use std::sync::Arc;
use tokio::time::{sleep, Duration};

/// A pending approval left unresolved this long is denied automatically (design.md D5,
/// `permissions-approval`'s "Unresolved approvals expire as a fail-closed denial").
const APPROVAL_TIMEOUT_SECONDS: i64 = 300;
const TIMEOUT_SWEEP_INTERVAL_SECONDS: u64 = 30;

pub(crate) fn assemble_permissions_api(database: NativeDatabase) -> PermissionsApi {
    let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
    let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
    let audit = Arc::new(SqliteAuditRepository::new(database));
    let clock = Arc::new(PermissionsSystemClock);
    let ids = Arc::new(PermissionsUuidIdGenerator);

    let evaluation = EvaluationService::new(
        principals.clone(),
        grants.clone(),
        audit.clone(),
        clock.clone(),
        ids.clone(),
    );
    let approvals = ApprovalBroker::new(
        principals,
        grants,
        audit,
        clock,
        ids,
        APPROVAL_TIMEOUT_SECONDS,
    );
    PermissionsApi::new(evaluation, approvals)
}

/// Periodically sweeps expired pending approvals and delivers their fail-closed denial back to
/// each one's waiting generation. `await_approval`'s own blocking-wait loop (in
/// `agent_runtime::infrastructure::api_process_adapter`) needs no changes to receive this — a
/// swept denial is delivered through the exact same `resolve_tool_approval` channel a human
/// `Deny` already uses (design.md D8's implementation-time refinement; tasks.md 6.5a).
pub(crate) fn start_permission_timeout_sweep_job(
    permissions: PermissionsApi,
    agent_runtime: AgentRuntimeApi,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            for request in permissions.sweep_timed_out_approvals() {
                // A `false` return means the generation had already ended between the timeout
                // being detected and delivery being attempted — a harmless no-op, nothing left
                // to unblock.
                let _ = agent_runtime.resolve_tool_approval(
                    &request.session_id,
                    &request.call_id,
                    ToolApprovalDecision::Denied,
                );
            }
            sleep(Duration::from_secs(TIMEOUT_SWEEP_INTERVAL_SECONDS)).await;
        }
    });
}
