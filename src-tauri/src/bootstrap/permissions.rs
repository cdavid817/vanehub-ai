use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use crate::contexts::agent_runtime::application::ToolApprovalDecision;
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::permissions::api::{Effect, PermissionsApi, PermissionsApplicationError};
use crate::contexts::permissions::application::{
    ApprovalBroker, ApprovalDeliveryPort, ApprovalResolutionRepository, ClaudeCodeHookPort,
    DeliveryAcknowledgement, DeliveryReservation, EvaluationService, PermissionsClockPort,
    ResolveApprovalUseCase,
};
use crate::contexts::permissions::domain::{ApprovalRequest, ApprovalResolutionId};
use crate::contexts::permissions::infrastructure::{
    start_hook_bridge_server, ClaudeCodeHookAdapter, DesktopDefaultTemplateAdapter,
    HookWaitRegistry, PermissionsSystemClock, PermissionsUuidIdGenerator,
    SqliteApprovalResolutionRepository, SqliteAuditRepository, SqliteGrantRepository,
    SqlitePrincipalRepository, TauriPendingApprovalEventAdapter, UnifiedLogDiagnosticsAdapter,
};
use crate::contexts::tooling::cli_config::infrastructure::NativeCliGlobalConfigAdapter;
use crate::platform::database::NativeDatabase;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Manager};
use tokio::time::{sleep, Duration};

/// A pending approval left unresolved this long is denied automatically (design.md D5,
/// `permissions-approval`'s "Unresolved approvals expire as a fail-closed denial").
const APPROVAL_TIMEOUT_SECONDS: i64 = 300;
const TIMEOUT_SWEEP_INTERVAL_SECONDS: u64 = 30;

/// Everything the composition root keeps hold of after assembling `permissions`.
///
/// The published API is what commands use, but resolving an approval also needs the pending
/// broker, the durable ledger, and the hook registry — and it needs `agent_runtime`, which does
/// not exist yet at this point in bootstrap. Returning the parts lets the resolver be assembled a
/// few lines later without any of them becoming globally reachable.
pub(crate) struct PermissionsAssembly {
    pub(crate) api: PermissionsApi,
    approvals: ApprovalBroker,
    resolutions: Arc<SqliteApprovalResolutionRepository>,
    clock: Arc<PermissionsSystemClock>,
    ids: Arc<PermissionsUuidIdGenerator>,
}

pub(crate) fn assemble_permissions_api(
    database: NativeDatabase,
    desktop_settings: DesktopSettingsApi,
    app: AppHandle,
) -> PermissionsAssembly {
    // Resolved before `app` is moved into the event adapter below.
    let wrapper_path = wrapper_binary_path(&app);

    let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
    let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
    let audit = Arc::new(SqliteAuditRepository::new(database.clone()));
    let resolutions = Arc::new(SqliteApprovalResolutionRepository::new(database));
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
        Arc::new(UnifiedLogDiagnosticsAdapter),
    );
    let approvals = ApprovalBroker::new(
        principals,
        clock.clone(),
        ids.clone(),
        events,
        APPROVAL_TIMEOUT_SECONDS,
    );
    let permissions = PermissionsApi::new(
        evaluation,
        approvals.clone(),
        hook_waits.clone(),
        claude_code_hook,
    );

    // Startup reconciliation (`permissions-approval`'s "Restart reconciliation never revives
    // pre-restart work"). A resolution left `committed` or `delivery_failed` had a waiter in a
    // process that no longer exists, so it becomes durable evidence and nothing else: no pending
    // request is recreated, no effect is delivered to a new generation, and its grant stays
    // inactive. Best-effort, because failing to reconcile is not a reason to refuse to start —
    // the states it leaves behind are already the fail-closed ones.
    let _ = resolutions.mark_aborted_by_restart(&clock.now());

    // Best-effort (mirrors cli_config's own startup-sync philosophy): a failure here just means
    // Claude Code CLI sessions behave as though VaneHub isn't running at all
    // (`claude-code-permission-hook`'s risk-tiered offline fallback), not a reason to fail the
    // rest of permissions bootstrap.
    let _ = start_hook_bridge_server(permissions.clone(), hook_waits.clone());

    // Same best-effort stance: when hook management was previously enabled, refresh the global
    // settings entry so it names this build's wrapper path instead of a pre-update location
    // (`add-permission-hook-recovery`'s startup reconvergence). A failure (for example a dev
    // build without the wrapper binary on disk) leaves CLI sessions in the same risk-tiered
    // offline fallback they were already in, and must not fail permissions bootstrap.
    let _ = permissions.reconverge_claude_code_hook();

    PermissionsAssembly {
        api: permissions,
        approvals,
        resolutions,
        clock,
        ids,
    }
}

/// Assembles the one use case that turns a decision into executable authority.
///
/// Separate from `assemble_permissions_api` only because it needs `agent_runtime`, which bootstrap
/// builds later. Everything it composes was already created above; nothing here reaches into
/// either context's internals.
pub(crate) fn assemble_approval_resolver(
    permissions: &PermissionsAssembly,
    agent_runtime: AgentRuntimeApi,
) -> ResolveApprovalUseCase {
    ResolveApprovalUseCase::new(
        permissions.approvals.clone(),
        permissions.resolutions.clone(),
        Arc::new(RoutedApprovalDelivery::new(
            agent_runtime,
            permissions.api.clone(),
        )),
        permissions.clock.clone(),
        permissions.ids.clone(),
    )
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

/// Delivers a committed resolution to whichever waiter raised the request.
///
/// Lives in bootstrap because it is the one thing that legitimately knows both contexts: it holds
/// `agent_runtime`'s published API and the `permissions`-owned hook registry, and satisfies a port
/// that `permissions` declared. Neither context imports the other's internals to make this work.
///
/// Both channels are consulted unconditionally rather than branched on, because a request was only
/// ever raised through exactly one of them and asking the wrong one is a harmless `false`. That is
/// the same stance the command adapter it replaces already took.
struct RoutedApprovalDelivery {
    agent_runtime: AgentRuntimeApi,
    permissions: PermissionsApi,
    /// Resolution ids already applied, so a retry of the same immutable id is acknowledged rather
    /// than released a second time. Process-local by design: a resolution whose delivery did not
    /// survive the process is reconciled as aborted at startup, never replayed.
    applied: Mutex<HashSet<String>>,
}

impl RoutedApprovalDelivery {
    fn new(agent_runtime: AgentRuntimeApi, permissions: PermissionsApi) -> Self {
        Self {
            agent_runtime,
            permissions,
            applied: Mutex::new(HashSet::new()),
        }
    }

    fn lock_applied(&self) -> std::sync::MutexGuard<'_, HashSet<String>> {
        self.applied.lock().unwrap_or_else(|poisoned| {
            debug_assert!(false, "delivered-resolution set poisoned");
            poisoned.into_inner()
        })
    }
}

impl ApprovalDeliveryPort for RoutedApprovalDelivery {
    fn reserve(
        &self,
        request: &ApprovalRequest,
    ) -> Result<Option<DeliveryReservation>, PermissionsApplicationError> {
        let native_is_live = self
            .agent_runtime
            .has_live_tool_approval_waiter(&request.session_id)
            .map_err(|error| {
                PermissionsApplicationError::infrastructure("agent_runtime", error.to_string())
            })?;
        if native_is_live || self.permissions.hook_waiter_is_live(&request.id) {
            return Ok(Some(DeliveryReservation {
                token: request.id.clone(),
            }));
        }
        Ok(None)
    }

    fn deliver(
        &self,
        reservation: &DeliveryReservation,
        request: &ApprovalRequest,
        resolution_id: &ApprovalResolutionId,
        effect: Effect,
    ) -> Result<DeliveryAcknowledgement, PermissionsApplicationError> {
        // The reservation names the request it was taken for. A mismatch means the two came from
        // different resolutions, and delivering would release a waiter nobody reserved.
        if reservation.token != request.id {
            return Ok(DeliveryAcknowledgement::WaiterGone);
        }
        // Checked and recorded under one lock: two retries arriving together must not both decide
        // they are the first.
        {
            let mut applied = self.lock_applied();
            if !applied.insert(resolution_id.as_str().to_string()) {
                return Ok(DeliveryAcknowledgement::AlreadyApplied);
            }
        }

        let native = self
            .agent_runtime
            .resolve_tool_approval(
                &request.session_id,
                &request.call_id,
                tool_approval_decision(effect),
            )
            .map_err(|error| {
                // The id is released so a retry can genuinely try again: the transport failed
                // before anything was released, so this delivery did not happen.
                self.lock_applied().remove(resolution_id.as_str());
                PermissionsApplicationError::infrastructure("agent_runtime", error.to_string())
            })?;
        let hook = self.permissions.resolve_hook_wait(&request.id, effect);

        if native || hook {
            Ok(DeliveryAcknowledgement::Applied)
        } else {
            // Reserved a moment ago and gone now. The decision stays durable and the grant stays
            // inactive; nothing was executed.
            self.lock_applied().remove(resolution_id.as_str());
            Ok(DeliveryAcknowledgement::WaiterGone)
        }
    }
}

fn tool_approval_decision(effect: Effect) -> ToolApprovalDecision {
    match effect {
        Effect::Allow => ToolApprovalDecision::Approved,
        // Ask never reaches delivery — `ApprovalDecisionRecord` refuses to commit one — and a
        // denial is the only safe reading of anything that is not an explicit Allow.
        Effect::Deny | Effect::Ask => ToolApprovalDecision::Denied,
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
    resolver: Arc<ResolveApprovalUseCase>,
) {
    tauri::async_runtime::spawn(async move {
        loop {
            // Sweeps only the ids; the denial itself goes through the same single-winner
            // claim/commit/deliver path a human `Deny` uses. Giving the sweep its own delivery
            // shortcut is how it would come to race a human decision and produce a second one.
            for request in permissions.expired_pending_approval_ids() {
                let _ = resolver.resolve_timed_out(&request);
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
