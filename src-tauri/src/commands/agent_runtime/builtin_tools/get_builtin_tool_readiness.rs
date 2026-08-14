use crate::contexts::agent_runtime::api::AgentRuntimeApi;
use serde::Serialize;
use tauri::State;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinToolModeReadiness {
    mode: &'static str,
    state: &'static str,
    reason_code: Option<&'static str>,
    simulated: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinToolCapabilityReadiness {
    capability: &'static str,
    modes: Vec<BuiltinToolModeReadiness>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuiltinToolReadiness {
    agent_id: String,
    observed_at: String,
    capabilities: Vec<BuiltinToolCapabilityReadiness>,
}

#[tauri::command]
pub(crate) fn get_builtin_tool_readiness(
    api: State<'_, AgentRuntimeApi>,
    agent_id: String,
) -> BuiltinToolReadiness {
    let onepiece = agent_id == "onepiece";
    BuiltinToolReadiness {
        agent_id,
        observed_at: chrono::Utc::now().to_rfc3339(),
        capabilities: vec![
            baseline_capability("filesystem", &["read", "write"], onepiece),
            baseline_capability("command", &["execute"], onepiece),
            capability(
                &api,
                "browser",
                &[("read", &["browser"]), ("execute", &["browser"])],
                onepiece,
            ),
            capability(
                &api,
                "web",
                &[("read", &["web_search", "web_fetch"])],
                onepiece,
            ),
            capability(
                &api,
                "code_execution",
                &[("execute", &["code_execution"])],
                onepiece,
            ),
            capability(&api, "ocr", &[("read", &["ocr"])], onepiece),
            capability(
                &api,
                "artifact",
                &[
                    ("read", &["artifact"]),
                    ("publish", &["artifact"]),
                    ("download", &["artifact"]),
                ],
                onepiece,
            ),
            capability(
                &api,
                "delegation",
                &[
                    ("read", &["delegate_cli"]),
                    ("write", &["delegate_cli"]),
                    ("apply", &["apply_delegation_changes"]),
                ],
                onepiece,
            ),
        ],
    }
}

fn capability(
    api: &AgentRuntimeApi,
    capability: &'static str,
    modes: &[(&'static str, &[&str])],
    onepiece: bool,
) -> BuiltinToolCapabilityReadiness {
    BuiltinToolCapabilityReadiness {
        capability,
        modes: modes
            .iter()
            .map(|(mode, tools)| {
                let backend_ready = tools
                    .iter()
                    .all(|tool| api.is_native_tool_backend_ready(tool));
                let backend_reason = tools
                    .iter()
                    .find_map(|tool| api.native_tool_readiness_reason(tool))
                    .or_else(|| (!backend_ready).then_some("backend_unavailable"));
                let (state, reason_code) = if !onepiece {
                    ("unavailable", Some("policy_unavailable"))
                } else if !api.is_native_tool_feature_enabled(capability, mode) {
                    ("unavailable", Some("disabled"))
                } else if backend_ready {
                    ("ready", None)
                } else {
                    ("unavailable", backend_reason)
                };
                BuiltinToolModeReadiness {
                    mode,
                    state,
                    reason_code,
                    simulated: false,
                }
            })
            .collect(),
    }
}

fn baseline_capability(
    capability: &'static str,
    modes: &[&'static str],
    onepiece: bool,
) -> BuiltinToolCapabilityReadiness {
    BuiltinToolCapabilityReadiness {
        capability,
        modes: modes
            .iter()
            .map(|mode| BuiltinToolModeReadiness {
                mode,
                state: if onepiece { "ready" } else { "unavailable" },
                reason_code: (!onepiece).then_some("policy_unavailable"),
                simulated: false,
            })
            .collect(),
    }
}
