pub(super) fn invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::commands::agent_runtime::builtin_tools::get_builtin_tool_readiness::get_builtin_tool_readiness,
        crate::commands::agent_runtime::builtin_tools::operations::get_builtin_tool_operation,
        crate::commands::agent_runtime::builtin_tools::operations::list_builtin_tool_operations,
        crate::commands::agent_runtime::builtin_tools::operations::cancel_builtin_tool_operation,
        crate::commands::agent_runtime::builtin_tools::artifacts::list_artifacts,
        crate::commands::agent_runtime::builtin_tools::artifacts::get_artifact,
        crate::commands::agent_runtime::builtin_tools::artifacts::read_artifact,
        crate::commands::agent_runtime::builtin_tools::artifacts::publish_artifact,
        crate::commands::agent_runtime::builtin_tools::artifacts::download_artifact,
        crate::commands::agent_runtime::builtin_tools::manual_delegation::start_delegation,
        crate::commands::agent_runtime::builtin_tools::delegation::list_delegation_attempts,
        crate::commands::agent_runtime::builtin_tools::delegation::get_delegation_report,
        crate::commands::agent_runtime::builtin_tools::change_set_review::get_change_set_review,
        crate::commands::agent_runtime::builtin_tools::manual_delegation::apply_delegation_changes,
        crate::commands::agent_runtime::builtin_tools::delegation::get_delegation_recovery,
        crate::commands::agent_runtime::builtin_tools::delegation::get_browser_handoff,
        crate::commands::agent_runtime::builtin_tools::delegation::begin_browser_handoff,
        crate::commands::agent_runtime::builtin_tools::delegation::resume_browser_automation,
    ]
}

pub(super) fn is_command(command: &str) -> bool {
    matches!(
        command,
        "get_builtin_tool_readiness"
            | "get_builtin_tool_operation"
            | "list_builtin_tool_operations"
            | "cancel_builtin_tool_operation"
            | "list_artifacts"
            | "get_artifact"
            | "read_artifact"
            | "publish_artifact"
            | "download_artifact"
            | "start_delegation"
            | "list_delegation_attempts"
            | "get_delegation_report"
            | "get_change_set_review"
            | "apply_delegation_changes"
            | "get_delegation_recovery"
            | "get_browser_handoff"
            | "begin_browser_handoff"
            | "resume_browser_automation"
    )
}
