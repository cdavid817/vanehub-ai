use super::dto::{
    ChatConfig, CreateSessionInput, InteractionMode, SessionExportFormat, UsageStatisticsRange,
};
use crate::commands::error::{map_command_error, CommandErrorCategory};
use crate::contexts::sessions::api::SessionsError;
use serde_json::{json, Value};

const MIGRATED_SESSION_COMMANDS: [(&str, &str); 23] = [
    ("create_session", include_str!("create_session.rs")),
    ("list_sessions", include_str!("list_sessions.rs")),
    (
        "list_archived_sessions",
        include_str!("list_archived_sessions.rs"),
    ),
    ("search_sessions", include_str!("search_sessions.rs")),
    (
        "list_session_categories",
        include_str!("list_session_categories.rs"),
    ),
    (
        "create_session_category",
        include_str!("create_session_category.rs"),
    ),
    (
        "rename_session_category",
        include_str!("rename_session_category.rs"),
    ),
    (
        "delete_session_category",
        include_str!("delete_session_category.rs"),
    ),
    (
        "assign_session_category",
        include_str!("assign_session_category.rs"),
    ),
    ("get_active_session", include_str!("get_active_session.rs")),
    (
        "get_session_chat_config",
        include_str!("get_session_chat_config.rs"),
    ),
    (
        "save_session_chat_config",
        include_str!("save_session_chat_config.rs"),
    ),
    ("switch_session", include_str!("switch_session.rs")),
    ("rename_session", include_str!("rename_session.rs")),
    ("pin_session", include_str!("pin_session.rs")),
    ("unpin_session", include_str!("unpin_session.rs")),
    ("archive_session", include_str!("archive_session.rs")),
    ("unarchive_session", include_str!("unarchive_session.rs")),
    ("export_session", include_str!("export_session.rs")),
    ("delete_session", include_str!("delete_session.rs")),
    ("list_messages", include_str!("list_messages.rs")),
    (
        "get_session_usage_summary",
        include_str!("get_session_usage_summary.rs"),
    ),
    (
        "get_usage_statistics",
        include_str!("get_usage_statistics.rs"),
    ),
];

#[test]
fn every_migrated_session_command_keeps_registration_frontend_and_error_boundaries() {
    let native_registration = include_str!("../registry.rs");
    let tauri_client = include_str!("../../../../src/services/tauri-agent-client.ts");

    for (command, handler) in MIGRATED_SESSION_COMMANDS {
        assert!(
            native_registration.contains(&format!("commands::sessions::{command}::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
        assert!(
            handler.contains("#[tauri::command]")
                && handler.contains(&format!("fn {command}("))
                && handler.contains("map_command_error"),
            "{command} must remain a Tauri adapter using the shared safe error mapper"
        );
    }
}

#[test]
fn session_command_input_dtos_keep_existing_serde_shapes() {
    let input: CreateSessionInput = serde_json::from_value(json!({
        "agentId": "codex-cli",
        "interactionMode": "cli",
        "title": "Remote",
        "folder": null,
        "projectPath": null,
        "remoteWorkspace": {
            "host": "dev.example",
            "user": "developer",
            "path": "/workspace",
            "displayName": "Remote fixture"
        },
        "worktree": { "enabled": false, "name": null }
    }))
    .expect("deserialize create session");
    assert_eq!(input.agent_id, "codex-cli");
    assert_eq!(input.interaction_mode, InteractionMode::Cli);
    assert_eq!(
        input
            .remote_workspace
            .expect("remote workspace")
            .display_name
            .as_deref(),
        Some("Remote fixture")
    );

    let config: ChatConfig = serde_json::from_value(json!({
        "agentId": "codex-cli",
        "interactionMode": "native-desktop",
        "permissionMode": "agent",
        "providerId": "openai",
        "modelId": "gpt-5.1-codex",
        "reasoningDepth": "high",
        "streaming": true,
        "thinking": true,
        "longContext": false
    }))
    .expect("deserialize chat config");
    assert_eq!(config.interaction_mode, InteractionMode::NativeDesktop);
    assert_eq!(config.reasoning_depth.as_deref(), Some("high"));

    assert_eq!(
        serde_json::from_value::<SessionExportFormat>(json!("markdown")).expect("export format"),
        SessionExportFormat::Markdown
    );
    assert_eq!(
        serde_json::from_value::<UsageStatisticsRange>(json!("last30Days")).expect("usage range"),
        UsageStatisticsRange::Last30Days
    );
}

#[test]
fn session_command_errors_keep_legacy_safe_strings() {
    let cases = [
        (
            SessionsError::Validation("invalid fixture".to_string()),
            CommandErrorCategory::Validation,
            "validation error: invalid fixture",
        ),
        (
            SessionsError::AgentNotFound("agent-1".to_string()),
            CommandErrorCategory::NotFound,
            "agent not found: agent-1",
        ),
        (
            SessionsError::UnsupportedInteractionMode("terminal".to_string()),
            CommandErrorCategory::Unsupported,
            "unsupported interaction mode: terminal",
        ),
        (
            SessionsError::SessionNotFound("session-1".to_string()),
            CommandErrorCategory::NotFound,
            "session not found: session-1",
        ),
        (
            SessionsError::MessageNotFound("message-1".to_string()),
            CommandErrorCategory::Validation,
            "validation error: Message not found: message-1",
        ),
        (
            SessionsError::CategoryNotFound("category-1".to_string()),
            CommandErrorCategory::NotFound,
            "validation error: Category not found: category-1",
        ),
        (
            SessionsError::CategoryNameConflict("Feature".to_string()),
            CommandErrorCategory::Conflict,
            "validation error: Category name already exists.",
        ),
        (
            SessionsError::Repository("secret database detail".to_string()),
            CommandErrorCategory::Infrastructure,
            "database error: secret database detail",
        ),
        (
            SessionsError::RuntimeLaunch("agent unavailable".to_string()),
            CommandErrorCategory::Unavailable,
            "launch failed: agent unavailable",
        ),
        (
            SessionsError::FileContent("export failed".to_string()),
            CommandErrorCategory::Infrastructure,
            "storage error: export failed",
        ),
    ];

    for (source, category, message) in cases {
        let error = map_command_error(source);
        assert_eq!(error.category(), category);
        assert_eq!(error.message(), message);
        assert_eq!(
            serde_json::to_value(error).expect("serialize command error"),
            Value::String(message.to_string())
        );
    }
}
