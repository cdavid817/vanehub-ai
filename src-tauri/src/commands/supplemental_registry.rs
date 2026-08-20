pub(super) fn invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::commands::skill_evolution_evidence::save_message_feedback::save_message_feedback,
        crate::commands::skill_evolution_evidence::query_evidence::query_skill_evolution_evidence,
        crate::commands::skill_evolution_evidence::query_evidence::get_skill_evolution_seed_lineage,
        crate::commands::skill_evolution_evidence::purge_evidence::purge_skill_evolution_evidence,
        crate::commands::task_orchestration::generate_plan_draft::generate_plan_draft,
        crate::commands::task_orchestration::save_plan_draft::save_plan_draft,
        crate::commands::task_orchestration::get_plan_draft::get_plan_draft,
        crate::commands::task_orchestration::plans::validate_plan_draft,
        crate::commands::task_orchestration::plans::list_plan_versions,
        crate::commands::task_orchestration::plans::delete_plan_draft,
        crate::commands::task_orchestration::plans::get_plan_attempt_evidence,
        crate::commands::task_orchestration::approve_plan::approve_plan,
        crate::commands::task_orchestration::start_plan_run::start_plan_run,
        crate::commands::task_orchestration::execute_next_plan_attempt::execute_next_plan_attempt,
        crate::commands::task_orchestration::controls::request_plan_control,
        crate::commands::task_orchestration::controls::retry_plan_subtask,
        crate::commands::task_orchestration::controls::accept_plan_run,
        crate::commands::task_orchestration::controls::recover_plan_run,
        crate::commands::task_orchestration::queries::list_plan_runs,
        crate::commands::task_orchestration::queries::get_plan_run_detail,
        crate::commands::task_orchestration::queries::get_plan_run_for_session,
        crate::commands::sessions::get_token_usage_details::get_token_usage_details,
        crate::commands::sessions::get_token_usage_summary::get_token_usage_summary,
        crate::commands::sessions::scheduled_tasks::list_scheduled_task_runs,
        crate::commands::work_board::commands::list_work_items,
        crate::commands::work_board::commands::create_work_item,
        crate::commands::work_board::commands::update_work_item,
        crate::commands::work_board::commands::move_work_item,
        crate::commands::work_board::commands::link_work_item_source,
        crate::commands::work_board::commands::archive_work_item,
        crate::commands::work_board::commands::restore_work_item,
        crate::commands::work_board::commands::delete_work_item,
        crate::commands::work_board::commands::list_plan_summaries,
        crate::commands::communications::begin_im_pairing::begin_im_pairing,
        crate::commands::communications::cancel_im_pairing::cancel_im_pairing,
        crate::commands::communications::get_im_session_binding::get_im_session_binding,
        crate::commands::communications::set_im_binding_paused::set_im_binding_paused,
        crate::commands::communications::set_im_completion_notifications::set_im_completion_notifications,
        crate::commands::communications::remove_im_session_binding::remove_im_session_binding,
        crate::commands::goals::list_goals::list_goals,
        crate::commands::goals::get_goal::get_goal,
        crate::commands::goals::create_goal::create_goal,
        crate::commands::goals::update_goal::update_goal,
        crate::commands::goals::delete_goal::delete_goal,
        crate::commands::goals::link_goal_target::link_goal_target,
        crate::commands::goals::unlink_goal_target::unlink_goal_target,
        crate::commands::goals::activate_goal::activate_goal,
        crate::commands::goals::accept_goal::accept_goal,
        crate::commands::goals::reopen_goal::reopen_goal,
        crate::commands::goals::abandon_goal::abandon_goal,
    ]
}

pub(super) fn is_command(command: &str) -> bool {
    matches!(
        command,
        "save_message_feedback"
            | "query_skill_evolution_evidence"
            | "get_skill_evolution_seed_lineage"
            | "purge_skill_evolution_evidence"
            | "generate_plan_draft"
            | "save_plan_draft"
            | "get_plan_draft"
            | "validate_plan_draft"
            | "list_plan_versions"
            | "delete_plan_draft"
            | "get_plan_attempt_evidence"
            | "approve_plan"
            | "start_plan_run"
            | "execute_next_plan_attempt"
            | "request_plan_control"
            | "retry_plan_subtask"
            | "accept_plan_run"
            | "recover_plan_run"
            | "list_plan_runs"
            | "get_plan_run_detail"
            | "get_plan_run_for_session"
            | "get_token_usage_details"
            | "get_token_usage_summary"
            | "list_scheduled_task_runs"
            | "list_work_items"
            | "create_work_item"
            | "update_work_item"
            | "move_work_item"
            | "link_work_item_source"
            | "archive_work_item"
            | "restore_work_item"
            | "delete_work_item"
            | "list_plan_summaries"
            | "begin_im_pairing"
            | "cancel_im_pairing"
            | "get_im_session_binding"
            | "set_im_binding_paused"
            | "set_im_completion_notifications"
            | "remove_im_session_binding"
            | "list_goals"
            | "get_goal"
            | "create_goal"
            | "update_goal"
            | "delete_goal"
            | "link_goal_target"
            | "unlink_goal_target"
            | "activate_goal"
            | "accept_goal"
            | "reopen_goal"
            | "abandon_goal"
    )
}

#[cfg(test)]
mod tests {
    /// This file holds the same command list twice: `generate_handler!` registers the handlers,
    /// and `is_command` decides what gets routed to them. Nothing but agreement between two
    /// hand-edited lists keeps them in step, and when they drifted the whole Goals domain became
    /// unreachable on the desktop -- eleven commands registered, none listed, so every call fell
    /// through to the core handler and came back "unknown command" (D-01, fixed in 13b0738f).
    ///
    /// Neither list is enumerable at runtime: `generate_handler!` expands to a closure and
    /// `is_command` to a `matches!`. So the check reads this file's own source, which is exactly
    /// what the drift is in. Adding a command to one list and not the other fails here rather than
    /// in whichever feature silently stops working.
    #[test]
    fn every_registered_supplemental_command_is_also_routed_to() {
        let source = include_str!("supplemental_registry.rs");
        let (handler_block, routing_block) = source
            .split_once("pub(super) fn is_command")
            .expect("the routing function should follow the handler list");

        let registered: std::collections::BTreeSet<&str> = handler_block
            .lines()
            .filter_map(|line| line.trim().strip_suffix(','))
            .filter(|line| line.starts_with("crate::commands::"))
            .filter_map(|path| path.rsplit("::").next())
            .collect();

        // Skips this comment block's own quoted names by taking only lines whose whole content is
        // one quoted identifier, optionally preceded by the `|` of the `matches!` arm.
        let routed: std::collections::BTreeSet<&str> = routing_block
            .lines()
            .map(|line| line.trim().trim_start_matches('|').trim())
            .filter_map(|line| line.strip_prefix('"')?.strip_suffix('"'))
            .filter(|name| {
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit())
            })
            .collect();

        assert!(
            !registered.is_empty() && !routed.is_empty(),
            "the lists could not be parsed: {} registered, {} routed",
            registered.len(),
            routed.len(),
        );
        let registered_not_routed: Vec<_> = registered.difference(&routed).collect();
        assert!(
            registered_not_routed.is_empty(),
            "registered with generate_handler! but not routed by is_command, so calls report \
             \"unknown command\": {registered_not_routed:?}",
        );
        let routed_not_registered: Vec<_> = routed.difference(&registered).collect();
        assert!(
            routed_not_registered.is_empty(),
            "routed by is_command but not registered with generate_handler!, so calls reach no \
             handler at all: {routed_not_registered:?}",
        );
    }
}
