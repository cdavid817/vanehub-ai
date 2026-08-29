pub(super) fn invoke_handler(
) -> impl Fn(tauri::ipc::Invoke<tauri::Wry>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![
        crate::commands::personalization::create_personalization_memory::create_personalization_memory,
        crate::commands::personalization::delete_personalization_memory::delete_personalization_memory,
        crate::commands::personalization::execute_personalization_reset::execute_personalization_reset,
        crate::commands::personalization::get_personalization_health::get_personalization_health,
        crate::commands::personalization::get_personalization_memory::get_personalization_memory,
        crate::commands::personalization::get_personalization_policy::get_personalization_policy,
        crate::commands::personalization::list_personalization_agent_capabilities::list_personalization_agent_capabilities,
        crate::commands::personalization::list_personalization_candidates::list_personalization_candidates,
        crate::commands::personalization::list_personalization_policies::list_personalization_policies,
        crate::commands::personalization::patch_personalization_policy::patch_personalization_policy,
        crate::commands::personalization::preview_effective_personalization::preview_effective_personalization,
        crate::commands::personalization::preview_personalization_reset::preview_personalization_reset,
        crate::commands::personalization::query_personalization_memories::query_personalization_memories,
        crate::commands::personalization::reconcile_personalization_memories::reconcile_personalization_memories,
        crate::commands::personalization::resolve_personalization_workspace::resolve_personalization_workspace,
        crate::commands::personalization::review_personalization_candidate::review_personalization_candidate,
        crate::commands::personalization::update_personalization_memory::update_personalization_memory,
        crate::commands::skill_evolution_evidence::save_message_feedback::save_message_feedback,
        crate::commands::skill_evolution_evidence::revoke_reusable_guidance_authorization::revoke_reusable_guidance_authorization,
        crate::commands::skill_evolution_evidence::query_evidence::query_skill_evolution_evidence,
        crate::commands::skill_evolution_evidence::query_evidence::get_skill_evolution_seed_lineage,
        crate::commands::skill_evolution_evidence::purge_evidence::purge_skill_evolution_evidence,
        crate::commands::skill_evolution_assessment::query_skill_evolution_assessments,
        crate::commands::skill_evolution_assessment::get_skill_evolution_assessment,
        crate::commands::skill_evolution_assessment::get_skill_evolution_assessment_policy,
        crate::commands::skill_evolution_assessment::update_skill_evolution_assessment_consent,
        crate::commands::skill_evolution_assessment::schedule_skill_evolution_reassessment,
        crate::commands::skill_evolution_generation::get_skill_evolution_generation_policy,
        crate::commands::skill_evolution_generation::update_skill_evolution_generation_policy,
        crate::commands::skill_evolution_generation::query_skill_evolution_generation_jobs,
        crate::commands::skill_evolution_generation::get_skill_evolution_generation_job,
        crate::commands::skill_evolution_generation::cancel_skill_evolution_generation_job,
        crate::commands::skill_evolution_generation::regenerate_skill_evolution_generation_job,
        crate::commands::skill_evolution_generation::get_skill_evolution_generation_dossier_section,
        crate::commands::skill_evolution_generation::get_skill_evolution_generation_provenance,
        crate::commands::skill_evolution_generation::query_skill_evolution_generation_quarantine,
        crate::commands::skill_evolution_generation::handoff_skill_evolution_generation_package,
        crate::commands::skill_evolution_generation::export_skill_evolution_generation_dossier,
        crate::commands::skill_evolution_orchestration::get_skill_evolution_scheduler_overview,
        crate::commands::skill_evolution_orchestration::get_skill_evolution_policy,
        crate::commands::skill_evolution_orchestration::update_skill_evolution_policy,
        crate::commands::skill_evolution_orchestration::list_skill_evolution_runs,
        crate::commands::skill_evolution_orchestration::get_skill_evolution_run,
        crate::commands::skill_evolution_orchestration::list_skill_evolution_eligibility,
        crate::commands::skill_evolution_orchestration::list_skill_evolution_applications,
        crate::commands::skill_evolution_orchestration::list_skill_evolution_probations,
        crate::commands::skill_evolution_orchestration::list_skill_evolution_breakers,
        crate::commands::skill_evolution_orchestration::request_skill_evolution_run,
        crate::commands::skill_evolution_orchestration::cancel_skill_evolution_run,
        crate::commands::skill_evolution_orchestration::acknowledge_skill_evolution_breaker,
        crate::commands::skill_evolution_orchestration::dispatch_skill_evolution_notifications,
        crate::commands::skill_evolution_curation::query_skill_curator_queue,
        crate::commands::skill_evolution_curation::get_skill_curator_candidate,
        crate::commands::skill_evolution_curation::query_skill_curator_audit,
        crate::commands::skill_evolution_curation::get_skill_curator_policy,
        crate::commands::skill_evolution_curation::dispatch_skill_curator_notifications,
        crate::commands::skill_evolution_curation::update_skill_curator_policy,
        crate::commands::skill_evolution_curation::save_skill_curator_draft,
        crate::commands::skill_evolution_curation::preview_skill_curator_candidate,
        crate::commands::skill_evolution_curation::approve_skill_curator_candidate,
        crate::commands::skill_evolution_curation::reject_skill_curator_candidate,
        crate::commands::skill_evolution_curation::defer_skill_curator_candidate,
        crate::commands::skill_evolution_curation::resume_skill_curator_candidate,
        crate::commands::skill_evolution_curation::retry_skill_curator_application,
        crate::commands::skill_evolution_system_activity::list_system_activity_sessions,
        crate::commands::skill_evolution_system_activity::query_system_activity_timeline,
        crate::commands::skill_evolution_system_activity::get_system_activity_read_state,
        crate::commands::skill_evolution_system_activity::advance_system_activity_read_cursor,
        crate::commands::skill_evolution_system_activity::mark_system_activity_unread,
        crate::commands::skill_evolution_system_activity::get_system_activity_preferences,
        crate::commands::skill_evolution_system_activity::update_system_activity_preferences,
        crate::commands::skill_evolution_system_activity::get_system_activity_dashboard,
        crate::commands::skill_evolution_system_activity::get_system_activity_health,
        crate::commands::skill_evolution_system_activity::open_system_activity_notification,
        crate::commands::skill_evolution_system_activity::dismiss_system_activity_notification,
        crate::commands::skill_evolution_system_activity::claim_system_activity_digests,
        crate::commands::skill_evolution_system_activity::begin_system_activity_rebuild,
        crate::commands::skill_evolution_system_activity::advance_system_activity_rebuild,
        crate::commands::skill_evolution_system_activity::validate_system_activity_rebuild,
        crate::commands::skill_evolution_system_activity::activate_system_activity_rebuild,
        crate::commands::skill_evolution_system_activity::cancel_system_activity_rebuild,
        crate::commands::skill_evolution_system_activity::export_system_activity,
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
        crate::commands::communications::begin_im_pairing::begin_im_pairing,
        crate::commands::communications::cancel_im_pairing::cancel_im_pairing,
        crate::commands::communications::get_im_session_binding::get_im_session_binding,
        crate::commands::communications::set_im_binding_paused::set_im_binding_paused,
        crate::commands::communications::set_im_completion_notifications::set_im_completion_notifications,
        crate::commands::communications::set_im_session_access::set_im_session_access,
        crate::commands::communications::remove_im_session_binding::remove_im_session_binding,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::communications::fixture_feishu_im::fixture_feishu_im_setup,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::communications::fixture_feishu_im::fixture_feishu_im_inject,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::communications::fixture_feishu_im::fixture_feishu_im_set_fault,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::communications::fixture_feishu_im::fixture_feishu_im_ledger,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::communications::fixture_feishu_im::fixture_feishu_im_reset,
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
        crate::commands::local_media::profile::get_local_media_profile,
        crate::commands::local_media::profile::discover_local_media_python_environments,
        crate::commands::local_media::profile::save_local_media_profile,
        crate::commands::local_media::profile::validate_local_media_profile,
        crate::commands::local_media::profile::get_local_media_status,
        crate::commands::local_media::profile::list_local_media_audio_devices,
        crate::commands::local_media::operations::start_local_media_probe,
        crate::commands::local_media::operations::stage_local_media_ocr_source,
        #[cfg(feature = "desktop-e2e")]
        crate::commands::local_media::operations::fixture_local_media_ocr_source,
        crate::commands::local_media::operations::start_local_media_ocr,
        crate::commands::local_media::operations::cleanup_local_media_staged_source,
        crate::commands::local_media::operations::start_microphone_recording,
        crate::commands::local_media::operations::stop_recording_and_transcribe,
        crate::commands::local_media::operations::cancel_microphone_recording,
        crate::commands::local_media::operations::start_local_media_tts,
        crate::commands::local_media::operations::stop_local_media_playback,
        crate::commands::local_media::operations::cancel_local_media_operation,
        crate::commands::local_media::operations::get_local_media_operation_result,
        crate::commands::local_media::screenshot::select_and_stage_screenshot_region,
        crate::commands::local_media::screenshot::commit_screenshot_selection,
        crate::commands::local_media::screenshot::cancel_screenshot_selection,
        crate::commands::local_media::screenshot::cancel_active_screenshot_selection,
    ]
}

pub(super) fn is_command(command: &str) -> bool {
    // Gated separately from the list below: a default build must not answer this name at all.
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_local_media_ocr_source" {
        return true;
    }
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_feishu_im_setup" {
        return true;
    }
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_feishu_im_inject" {
        return true;
    }
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_feishu_im_set_fault" {
        return true;
    }
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_feishu_im_ledger" {
        return true;
    }
    #[cfg(feature = "desktop-e2e")]
    if command == "fixture_feishu_im_reset" {
        return true;
    }
    matches!(
        command,
        "save_message_feedback"
            | "revoke_reusable_guidance_authorization"
            | "query_skill_evolution_evidence"
            | "get_skill_evolution_seed_lineage"
            | "purge_skill_evolution_evidence"
            | "query_skill_evolution_assessments"
            | "get_skill_evolution_assessment"
            | "get_skill_evolution_assessment_policy"
            | "update_skill_evolution_assessment_consent"
            | "schedule_skill_evolution_reassessment"
            | "get_skill_evolution_generation_policy"
            | "update_skill_evolution_generation_policy"
            | "query_skill_evolution_generation_jobs"
            | "get_skill_evolution_generation_job"
            | "cancel_skill_evolution_generation_job"
            | "regenerate_skill_evolution_generation_job"
            | "get_skill_evolution_generation_dossier_section"
            | "get_skill_evolution_generation_provenance"
            | "query_skill_evolution_generation_quarantine"
            | "handoff_skill_evolution_generation_package"
            | "export_skill_evolution_generation_dossier"
            | "get_skill_evolution_scheduler_overview"
            | "get_skill_evolution_policy"
            | "update_skill_evolution_policy"
            | "list_skill_evolution_runs"
            | "get_skill_evolution_run"
            | "list_skill_evolution_eligibility"
            | "list_skill_evolution_applications"
            | "list_skill_evolution_probations"
            | "list_skill_evolution_breakers"
            | "request_skill_evolution_run"
            | "cancel_skill_evolution_run"
            | "acknowledge_skill_evolution_breaker"
            | "dispatch_skill_evolution_notifications"
            | "query_skill_curator_queue"
            | "get_skill_curator_candidate"
            | "query_skill_curator_audit"
            | "get_skill_curator_policy"
            | "dispatch_skill_curator_notifications"
            | "update_skill_curator_policy"
            | "save_skill_curator_draft"
            | "preview_skill_curator_candidate"
            | "approve_skill_curator_candidate"
            | "reject_skill_curator_candidate"
            | "defer_skill_curator_candidate"
            | "resume_skill_curator_candidate"
            | "retry_skill_curator_application"
            | "list_system_activity_sessions"
            | "query_system_activity_timeline"
            | "get_system_activity_read_state"
            | "advance_system_activity_read_cursor"
            | "mark_system_activity_unread"
            | "get_system_activity_preferences"
            | "update_system_activity_preferences"
            | "get_system_activity_dashboard"
            | "get_system_activity_health"
            | "open_system_activity_notification"
            | "dismiss_system_activity_notification"
            | "claim_system_activity_digests"
            | "begin_system_activity_rebuild"
            | "advance_system_activity_rebuild"
            | "validate_system_activity_rebuild"
            | "activate_system_activity_rebuild"
            | "cancel_system_activity_rebuild"
            | "export_system_activity"
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
            | "begin_im_pairing"
            | "cancel_im_pairing"
            | "get_im_session_binding"
            | "set_im_binding_paused"
            | "set_im_completion_notifications"
            | "set_im_session_access"
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
            | "get_local_media_profile"
            | "discover_local_media_python_environments"
            | "save_local_media_profile"
            | "validate_local_media_profile"
            | "get_local_media_status"
            | "list_local_media_audio_devices"
            | "start_local_media_probe"
            | "stage_local_media_ocr_source"
            | "start_local_media_ocr"
            | "cleanup_local_media_staged_source"
            | "start_microphone_recording"
            | "stop_recording_and_transcribe"
            | "cancel_microphone_recording"
            | "start_local_media_tts"
            | "stop_local_media_playback"
            | "cancel_local_media_operation"
            | "get_local_media_operation_result"
            | "select_and_stage_screenshot_region"
            | "commit_screenshot_selection"
            | "cancel_screenshot_selection"
            | "cancel_active_screenshot_selection"
            | "create_personalization_memory"
            | "delete_personalization_memory"
            | "execute_personalization_reset"
            | "get_personalization_health"
            | "get_personalization_memory"
            | "get_personalization_policy"
            | "list_personalization_agent_capabilities"
            | "list_personalization_candidates"
            | "list_personalization_policies"
            | "patch_personalization_policy"
            | "preview_effective_personalization"
            | "preview_personalization_reset"
            | "query_personalization_memories"
            | "reconcile_personalization_memories"
            | "resolve_personalization_workspace"
            | "review_personalization_candidate"
            | "update_personalization_memory"
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
            // A feature-gated name cannot be a conditional arm of `matches!`, so it is routed by a
            // guarded early return instead. Both shapes count as routed.
            .map(|line| match line.split_once("command == ") {
                Some((_, rest)) => rest.trim_end_matches(" {").trim(),
                None => line,
            })
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
