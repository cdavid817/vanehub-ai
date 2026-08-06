//! The loopback HTTP bridge (`claude-code-permission-hook`'s core requirement): translates a
//! `PreToolUse` request the hook wrapper forwards into an `Action`/`Resource` pair and resolves
//! it through the same `evaluate()`/`ApprovalBroker` pipeline native agents already use — no
//! second decision engine (design.md D1).
//!
//! This is VaneHub's own wrapper<->server wire protocol, not Claude Code's hook JSON directly:
//! the wrapper (a separate binary, Group 4) is responsible for translating Claude Code's own
//! stdin/stdout hook contract to and from this one.

use super::hook_bridge_discovery::{discovery_file_path, generate_token, write_discovery_file};
use super::hook_bridge_mapping::map_tool_to_action;
use super::hook_bridge_wait_registry::HookWaitRegistry;
use crate::contexts::permissions::api::{
    Action, Effect, PermissionsApi, Resource, CLAUDE_CODE_AGENT_ID,
};
use axum::extract::{Json, Request, State};
use axum::http::{header, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::Router;
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::Arc;
use uuid::Uuid;

struct HookBridgeState {
    permissions: PermissionsApi,
    wait_registry: Arc<HookWaitRegistry>,
    expected_token: String,
}

#[derive(Deserialize)]
struct HookEvaluateRequest {
    tool_name: String,
    tool_input: serde_json::Value,
    session_id: String,
    cwd: String,
}

#[derive(Serialize)]
struct HookEvaluateResponse {
    decision: &'static str,
}

/// Binds a loopback listener on an OS-assigned port, writes the port and a fresh per-launch
/// bearer token to the discovery file the hook wrapper reads, and spawns the server. Mirrors
/// `start_permission_timeout_sweep_job`'s shape: a sync entry point that spawns its own async
/// work, so bootstrap call sites don't need to be async themselves.
pub(crate) fn start_hook_bridge_server(
    permissions: PermissionsApi,
    wait_registry: Arc<HookWaitRegistry>,
) -> io::Result<()> {
    let Some(discovery_path) = discovery_file_path() else {
        return Err(io::Error::other(
            "could not resolve a local-data directory for the permission hook discovery file",
        ));
    };
    let std_listener = std::net::TcpListener::bind("127.0.0.1:0")?;
    std_listener.set_nonblocking(true)?;
    let port = std_listener.local_addr()?.port();
    let token = generate_token();

    write_discovery_file(&discovery_path, port, &token)?;

    let state = Arc::new(HookBridgeState {
        permissions,
        wait_registry,
        expected_token: token,
    });

    let app = Router::new()
        .route("/evaluate", post(handle_evaluate))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_bearer_token,
        ))
        .with_state(state);

    tauri::async_runtime::spawn(async move {
        let Ok(listener) = tokio::net::TcpListener::from_std(std_listener) else {
            return;
        };
        let _ = axum::serve(listener, app).await;
    });

    Ok(())
}

async fn require_bearer_token(
    State(state): State<Arc<HookBridgeState>>,
    request: Request,
    next: Next,
) -> Response {
    let authorized = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| token == state.expected_token);

    if !authorized {
        return StatusCode::UNAUTHORIZED.into_response();
    }
    next.run(request).await
}

async fn handle_evaluate(
    State(state): State<Arc<HookBridgeState>>,
    Json(request): Json<HookEvaluateRequest>,
) -> Json<HookEvaluateResponse> {
    let Some((action, resource)) = map_tool_to_action(&request.tool_name, &request.tool_input)
    else {
        return Json(deny());
    };

    // No native-agent-style "generation" exists for a CLI hook call, so session_id doubles for
    // it here; nothing downstream treats generation_id specially outside the native-agent
    // stale-generation check, which has no equivalent case in this path — `HookWaitRegistry`'s
    // own `resolve()` already reports whether anything was still waiting.
    let effect = state.permissions.evaluate(
        CLAUDE_CODE_AGENT_ID,
        action.clone(),
        resource.clone(),
        &request.session_id,
        &request.session_id,
        &request.cwd,
    );

    let resolved = match effect {
        Effect::Allow | Effect::Deny => effect,
        Effect::Ask => await_human_decision(&state, action, resource, &request).await,
    };

    Json(match resolved {
        Effect::Allow => allow(),
        Effect::Deny | Effect::Ask => deny(),
    })
}

async fn await_human_decision(
    state: &HookBridgeState,
    action: Action,
    resource: Resource,
    request: &HookEvaluateRequest,
) -> Effect {
    let call_id = Uuid::new_v4().to_string();
    let pending = state.permissions.create_pending_approval(
        CLAUDE_CODE_AGENT_ID,
        action,
        resource,
        &request.session_id,
        &request.session_id,
        &call_id,
        &request.cwd,
    );

    let Ok(pending) = pending else {
        return Effect::Deny;
    };

    // No additional timeout here: the existing pending-approval sweep (bootstrap/permissions.rs)
    // already delivers a fail-closed Deny through this same registry once the approval times
    // out, and a dropped sender resolves the receiver immediately rather than hanging — so this
    // await is already bounded by mechanisms that exist independently of this handler.
    state
        .wait_registry
        .register(&pending.id)
        .await
        .unwrap_or(Effect::Deny)
}

fn allow() -> HookEvaluateResponse {
    HookEvaluateResponse { decision: "allow" }
}

fn deny() -> HookEvaluateResponse {
    HookEvaluateResponse { decision: "deny" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::application::{
        ApprovalBroker, DefaultTemplatePort, EvaluationService, PendingApprovalEventPort,
        PermissionsApplicationError,
    };
    use crate::contexts::permissions::domain::{ApprovalRequest, PolicyTemplateName};
    use crate::contexts::permissions::infrastructure::{
        PermissionsSystemClock, PermissionsUuidIdGenerator, SqliteAuditRepository,
        SqliteGrantRepository, SqlitePrincipalRepository,
    };
    use crate::platform::database::NativeDatabase;
    use crate::test_support::TempDirectory;
    use serde_json::json;
    use std::time::Duration;

    struct FixedTemplate(PolicyTemplateName);
    impl DefaultTemplatePort for FixedTemplate {
        fn default_template(&self) -> PolicyTemplateName {
            self.0
        }
    }

    struct NoopEvents;
    impl PendingApprovalEventPort for NoopEvents {
        fn publish(&self, _request: &ApprovalRequest) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    struct NoopClaudeCodeHook;
    impl crate::contexts::permissions::application::ClaudeCodeHookPort for NoopClaudeCodeHook {
        fn install(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
        fn remove(&self) -> Result<(), PermissionsApplicationError> {
            Ok(())
        }
    }

    /// A real, SQLite-backed `PermissionsApi` (not fakes) — this suite exercises the actual
    /// axum wiring end-to-end, matching `migration_equivalence_tests.rs`'s own rationale for why
    /// that composition needs a real infrastructure-level test rather than a unit test in
    /// isolation. Returns the same `HookWaitRegistry` instance the api was built with, so tests
    /// can resolve pending waits exactly as `resolve_pending_approval`/the timeout sweep would in
    /// production — reusing a *different* registry instance here would silently hang the Ask
    /// path tests, since `register()` and `resolve()` would never see each other.
    fn test_permissions_api(
        temp_label: &str,
        template: PolicyTemplateName,
    ) -> (PermissionsApi, Arc<HookWaitRegistry>) {
        let directory = TempDirectory::new(temp_label);
        let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");

        let principals = Arc::new(SqlitePrincipalRepository::new(database.clone()));
        let grants = Arc::new(SqliteGrantRepository::new(database.clone()));
        let audit = Arc::new(SqliteAuditRepository::new(database));
        let clock = Arc::new(PermissionsSystemClock);
        let ids = Arc::new(PermissionsUuidIdGenerator);
        let hook_waits = Arc::new(HookWaitRegistry::new());

        let evaluation = EvaluationService::new(
            principals.clone(),
            grants.clone(),
            audit.clone(),
            clock.clone(),
            ids.clone(),
            Arc::new(FixedTemplate(template)),
        );
        let approvals = ApprovalBroker::new(
            principals,
            grants,
            audit,
            clock,
            ids,
            Arc::new(NoopEvents),
            300,
        );
        let permissions = PermissionsApi::new(
            evaluation,
            approvals,
            hook_waits.clone(),
            Arc::new(NoopClaudeCodeHook),
        );
        (permissions, hook_waits)
    }

    async fn spawn_test_server(
        permissions: PermissionsApi,
        wait_registry: Arc<HookWaitRegistry>,
    ) -> (String, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let token = "test-token".to_string();

        let state = Arc::new(HookBridgeState {
            permissions,
            wait_registry,
            expected_token: token.clone(),
        });

        let app = Router::new()
            .route("/evaluate", post(handle_evaluate))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                require_bearer_token,
            ))
            .with_state(state);

        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        (format!("http://{addr}/evaluate"), token)
    }

    #[tokio::test]
    async fn mapped_tool_with_an_allow_resolving_policy_returns_allow() {
        let (permissions, waits) = test_permissions_api("hook-bridge-allow", PolicyTemplateName::Trusted);
        let (url, token) = spawn_test_server(permissions, waits).await;

        let response = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&token)
            .json(&json!({
                "tool_name": "Bash",
                "tool_input": {"command": "ls"},
                "session_id": "session-1",
                "cwd": "/tmp/project"
            }))
            .send()
            .await
            .expect("request should succeed");

        assert_eq!(response.status(), 200);
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["decision"], "allow");
    }

    #[tokio::test]
    async fn wrong_token_is_rejected_before_evaluation() {
        let (permissions, waits) =
            test_permissions_api("hook-bridge-auth-wrong", PolicyTemplateName::Trusted);
        let (url, _token) = spawn_test_server(permissions, waits).await;

        let response = reqwest::Client::new()
            .post(&url)
            .bearer_auth("not-the-real-token")
            .json(&json!({"tool_name": "Bash", "tool_input": {}, "session_id": "s", "cwd": "/tmp"}))
            .send()
            .await
            .expect("request should succeed at the transport level");

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn missing_token_is_rejected() {
        let (permissions, waits) =
            test_permissions_api("hook-bridge-auth-missing", PolicyTemplateName::Trusted);
        let (url, _token) = spawn_test_server(permissions, waits).await;

        let response = reqwest::Client::new()
            .post(&url)
            .json(&json!({"tool_name": "Bash", "tool_input": {}, "session_id": "s", "cwd": "/tmp"}))
            .send()
            .await
            .expect("request should succeed at the transport level");

        assert_eq!(response.status(), 401);
    }

    #[tokio::test]
    async fn unmapped_tool_denies_without_reaching_evaluate() {
        let (permissions, waits) =
            test_permissions_api("hook-bridge-unmapped", PolicyTemplateName::Readonly);
        let (url, token) = spawn_test_server(permissions, waits).await;

        let response = reqwest::Client::new()
            .post(&url)
            .bearer_auth(&token)
            .json(&json!({
                "tool_name": "WebFetch",
                "tool_input": {},
                "session_id": "session-1",
                "cwd": "/tmp"
            }))
            .send()
            .await
            .expect("request should succeed");

        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["decision"], "deny");
    }

    #[tokio::test]
    async fn ask_resolving_policy_blocks_until_a_human_decision_arrives() {
        let (permissions, waits) =
            test_permissions_api("hook-bridge-ask", PolicyTemplateName::Standard);
        let (url, token) = spawn_test_server(permissions.clone(), waits).await;

        let in_flight = tokio::spawn({
            let url = url.clone();
            let token = token.clone();
            async move {
                reqwest::Client::new()
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&json!({
                        "tool_name": "Bash",
                        "tool_input": {"command": "ls"},
                        "session_id": "session-1",
                        "cwd": "/tmp/project"
                    }))
                    .send()
                    .await
                    .expect("request should succeed")
            }
        });

        let mut pending_id = None;
        for _ in 0..50 {
            if let Some(request) = permissions.list_pending_approvals().into_iter().next() {
                pending_id = Some(request.id);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let pending_id = pending_id.expect("a pending approval should have been created");

        assert!(permissions.resolve_hook_wait(&pending_id, Effect::Allow));

        let response = in_flight.await.expect("join");
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["decision"], "allow");
    }

    #[tokio::test]
    async fn ask_resolving_policy_denies_when_swept_as_timed_out() {
        let (permissions, waits) =
            test_permissions_api("hook-bridge-ask-timeout", PolicyTemplateName::Standard);
        let (url, token) = spawn_test_server(permissions.clone(), waits).await;

        let in_flight = tokio::spawn({
            let url = url.clone();
            let token = token.clone();
            async move {
                reqwest::Client::new()
                    .post(&url)
                    .bearer_auth(&token)
                    .json(&json!({
                        "tool_name": "Bash",
                        "tool_input": {"command": "ls"},
                        "session_id": "session-1",
                        "cwd": "/tmp/project"
                    }))
                    .send()
                    .await
                    .expect("request should succeed")
            }
        });

        let mut pending_id = None;
        for _ in 0..50 {
            if let Some(request) = permissions.list_pending_approvals().into_iter().next() {
                pending_id = Some(request.id);
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let pending_id = pending_id.expect("a pending approval should have been created");

        // Simulates what `start_permission_timeout_sweep_job` does once a pending approval times
        // out: deliver a Deny through this same registry, independent of any human decision.
        assert!(permissions.resolve_hook_wait(&pending_id, Effect::Deny));

        let response = in_flight.await.expect("join");
        let body: serde_json::Value = response.json().await.expect("json body");
        assert_eq!(body["decision"], "deny");
    }
}
