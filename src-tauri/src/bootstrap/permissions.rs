use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::permissions::api::{Effect, PermissionsApi, PermissionsApplicationError};
use crate::contexts::permissions::application::{
    ApprovalBroker, ClaudeCodeHookPort, EvaluationService,
};
use crate::contexts::permissions::infrastructure::{
    start_hook_bridge_server, ClaudeCodeHookAdapter, DesktopDefaultTemplateAdapter,
    HookWaitRegistry, PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
    SqliteGrantRepository, SqlitePrincipalRepository, TauriPendingApprovalEventAdapter,
};
use crate::contexts::tooling::cli_config::infrastructure::NativeCliGlobalConfigAdapter;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::time::{sleep, Duration};

/// A pending approval left unresolved this long is denied automatically (design.md D5,
/// `permissions-approval`'s "Unresolved approvals expire as a fail-closed denial").
const APPROVAL_TIMEOUT_SECONDS: i64 = 300;
const TIMEOUT_SWEEP_INTERVAL_SECONDS: u64 = 30;

pub(crate) fn assemble_permissions_api(
    database: NativeDatabase,
    desktop_settings: DesktopSettingsApi,
    app: AppHandle,
) -> PermissionsApi {
    // Resolved before `app` is moved into the event adapter below.
    let wrapper_path = wrapper_binary_path(&app);

    let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
    let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
    let audit = Arc::new(SqliteAuditRepository::new(database));
    let clock = Arc::new(PermissionsSystemClock);
    let ids = Arc::new(PermissionsUuidIdGenerator);
    let default_template = Arc::new(DesktopDefaultTemplateAdapter::new(desktop_settings));
    let events = Arc::new(TauriPendingApprovalEventAdapter::new(app));
    let hook_waits = Arc::new(HookWaitRegistry::new());
    let claude_code_hook = claude_code_hook_port(wrapper_path);

    let evaluation = EvaluationService::new(
        principals.clone(),
        grants.clone(),
        audit.clone(),
        clock.clone(),
        ids.clone(),
        default_template,
    );
    let approvals = ApprovalBroker::new(
        principals,
        grants,
        audit,
        clock,
        ids,
        events,
        APPROVAL_TIMEOUT_SECONDS,
    );
    let permissions =
        PermissionsApi::new(evaluation, approvals, hook_waits.clone(), claude_code_hook);

    // Best-effort (mirrors cli_config's own startup-sync philosophy): a failure here just means
    // Claude Code CLI sessions behave as though VaneHub isn't running at all
    // (`claude-code-permission-hook`'s risk-tiered offline fallback), not a reason to fail the
    // rest of permissions bootstrap.
    let _ = start_hook_bridge_server(permissions.clone(), hook_waits);

    // Same best-effort stance: when hook management was previously enabled, refresh the global
    // settings entry so it names this build's wrapper path instead of a pre-update location
    // (`add-permission-hook-recovery`'s startup reconvergence). A failure (for example a dev
    // build without the wrapper binary on disk) leaves CLI sessions in the same risk-tiered
    // offline fallback they were already in, and must not fail permissions bootstrap.
    let _ = permissions.reconverge_claude_code_hook();

    permissions
}

/// A fresh, independent `NativeCliGlobalConfigAdapter` — deliberately *not* the same instance
/// `bootstrap::cli_config` constructs for `CliConfigApi`. They'd ideally share one (and its
/// per-agent lock), but `ClaudeCodeHookAdapter`'s own read-then-recheck-before-write drift guard
/// (`cli-agent-config-management`'s "Claude Code permission-hook projection") already makes a
/// genuine race between the two safe — worst case one of them surfaces a `DriftConflict` and the
/// caller can retry, never silent corruption. Constructing two adapters is simpler than threading
/// one shared instance across two otherwise-unrelated bootstrap modules for a property the drift
/// guard already provides.
fn claude_code_hook_port(wrapper_path: PathBuf) -> Arc<dyn ClaudeCodeHookPort> {
    match NativeCliGlobalConfigAdapter::new() {
        Ok(adapter) => Arc::new(ClaudeCodeHookAdapter::new(Arc::new(adapter), wrapper_path)),
        Err(_) => Arc::new(UnavailableClaudeCodeHook),
    }
}

struct UnavailableClaudeCodeHook;

impl ClaudeCodeHookPort for UnavailableClaudeCodeHook {
    fn install(&self) -> Result<(), PermissionsApplicationError> {
        Err(PermissionsApplicationError::infrastructure(
            "cli_config",
            "could not resolve the user's home directory",
        ))
    }

    fn remove(&self) -> Result<(), PermissionsApplicationError> {
        Err(PermissionsApplicationError::infrastructure(
            "cli_config",
            "could not resolve the user's home directory",
        ))
    }
}

/// Tauri external binaries are installed beside the main executable. The resource-directory
/// fallback keeps development and older custom packages diagnosable through the adapter's
/// existing-file guard.
fn wrapper_binary_path(app: &AppHandle) -> PathBuf {
    wrapper_binary_path_from(
        std::env::current_exe()
            .ok()
            .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf)),
        app.path().resource_dir().ok(),
    )
}

fn wrapper_binary_path_from(
    executable_directory: Option<PathBuf>,
    resource_directory: Option<PathBuf>,
) -> PathBuf {
    let file_name = wrapper_binary_file_name();
    executable_directory
        .or(resource_directory)
        .unwrap_or_default()
        .join(file_name)
}

fn wrapper_binary_file_name() -> &'static str {
    if cfg!(windows) {
        "vanehub-permission-hook.exe"
    } else {
        "vanehub-permission-hook"
    }
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
                // Both delivery attempts are unconditional and independently harmless no-ops: a
                // given request was only ever raised through exactly one PEP channel, so at most
                // one of these actually finds a live waiter. A `false` return means that
                // channel's generation/request had already ended before delivery was attempted —
                // nothing left to unblock, not an error.
                let _ = agent_runtime.resolve_tool_approval(
                    &request.session_id,
                    &request.call_id,
                    ToolApprovalDecision::Denied,
                );
                let _ = permissions.resolve_hook_wait(&request.id, Effect::Deny);
            }
            sleep(Duration::from_secs(TIMEOUT_SWEEP_INTERVAL_SECONDS)).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{wrapper_binary_file_name, wrapper_binary_path_from};
    use std::path::PathBuf;

    #[test]
    fn permission_hook_prefers_the_external_binary_beside_the_application() {
        let executable_directory = PathBuf::from("app/bin");
        let resource_directory = PathBuf::from("app/resources");

        assert_eq!(
            wrapper_binary_path_from(Some(executable_directory.clone()), Some(resource_directory)),
            executable_directory.join(wrapper_binary_file_name())
        );
    }

    #[test]
    fn permission_hook_retains_the_resource_directory_fallback() {
        let resource_directory = PathBuf::from("app/resources");

        assert_eq!(
            wrapper_binary_path_from(None, Some(resource_directory.clone())),
            resource_directory.join(wrapper_binary_file_name())
        );
    }
}
