use crate::commands::error::CommandError;
use crate::commands::tooling::mcp::dto::{McpScope, McpServerConfig, McpTransportType};
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn command_registration_source() -> &'static str {
    concat!(
        include_str!("commands/core_registry.rs"),
        include_str!("commands/builtin_tool_registry.rs"),
        include_str!("commands/supplemental_registry.rs")
    )
}

#[test]
fn operation_contract_keeps_lowercase_enums_and_camel_case_fields() {
    let value = serde_json::to_value(OperationTask {
        id: "operation-1".to_string(),
        execution_run_id: None,
        trace_id: None,
        kind: OperationKind::Mcp,
        status: OperationStatus::Running,
        related_entity_id: Some("server-1".to_string()),
        message: Some("Connecting".to_string()),
        logs: Vec::new(),
        result: Some(json!({ "ready": true })),
        error: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        phase: None,
        completed_units: None,
        total_units: None,
        cancellable: None,
    })
    .expect("serialize operation");

    assert_eq!(value["kind"], "mcp");
    assert_eq!(value["status"], "running");
    assert_eq!(value["relatedEntityId"], "server-1");
    assert!(value.get("related_entity_id").is_none());
    assert!(value.get("executionRunId").is_none());
    assert!(value.get("traceId").is_none());
    // An operation that declares no progress serializes exactly as it did before these fields
    // existed, so every existing non-CLI consumer stays byte-compatible.
    assert!(value.get("phase").is_none());
    assert!(value.get("completedUnits").is_none());
    assert!(value.get("totalUnits").is_none());
    assert!(value.get("cancellable").is_none());
}

#[test]
fn operation_contract_exposes_cli_kind_and_camel_case_progress() {
    let value = serde_json::to_value(OperationTask {
        id: "operation-cli-1".to_string(),
        execution_run_id: None,
        trace_id: None,
        kind: OperationKind::Cli,
        status: OperationStatus::Running,
        related_entity_id: Some("claude-code".to_string()),
        message: None,
        logs: Vec::new(),
        result: None,
        error: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
        phase: Some("querying-catalog".to_string()),
        completed_units: Some(1),
        total_units: Some(3),
        cancellable: Some(true),
    })
    .expect("serialize cli operation");

    assert_eq!(value["kind"], "cli");
    // Status stays authoritative and unchanged; phase is additive and descriptive.
    assert_eq!(value["status"], "running");
    assert_eq!(value["phase"], "querying-catalog");
    assert_eq!(value["completedUnits"], 1);
    assert_eq!(value["totalUnits"], 3);
    assert_eq!(value["cancellable"], true);
    assert!(value.get("completed_units").is_none());
    assert!(value.get("total_units").is_none());
}

#[test]
fn operation_contract_exposes_optional_execution_correlation() {
    let mut operation = OperationTask::start(
        "operation-2".to_string(),
        OperationKind::Agent,
        Some("session-1".to_string()),
        None,
        "2026-01-01T00:00:00Z".to_string(),
    );
    operation.correlate_execution(
        "018f0f17-4d6a-7e20-b41d-66c5271a28d0".to_string(),
        "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
    );

    let value = serde_json::to_value(operation).expect("serialize correlated operation");
    assert_eq!(
        value["executionRunId"],
        "018f0f17-4d6a-7e20-b41d-66c5271a28d0"
    );
    assert_eq!(value["traceId"], "4bf92f3577b34da6a3ce929d0e0e4736");
}

#[test]
fn mcp_contract_keeps_transport_and_scope_values() {
    let value = serde_json::to_value(McpServerConfig {
        name: "fixture".to_string(),
        transport_type: McpTransportType::StreamableHttp,
        command: None,
        args: None,
        env: None,
        url: Some("https://example.test/mcp".to_string()),
        headers: None,
        description: None,
        active: true,
        scope: McpScope::Project,
        project_path: Some("D:\\code\\fixture".to_string()),
    })
    .expect("serialize MCP config");

    assert_eq!(value["transportType"], "streamable_http");
    assert_eq!(value["scope"], "project");
    assert_eq!(value["projectPath"], "D:\\code\\fixture");
}

#[test]
fn command_error_contract_remains_a_display_string() {
    let value =
        serde_json::to_value(CommandError::validation("invalid fixture")).expect("serialize error");

    assert_eq!(
        value,
        Value::String("validation error: invalid fixture".to_string())
    );
}

#[test]
fn every_tauri_command_is_registered_exactly_once() {
    let command_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/commands");
    let registry = command_registration_source();
    let mut sources = Vec::new();
    collect_rust_sources(&command_root, &mut sources);

    for source_path in sources {
        let source = fs::read_to_string(&source_path).expect("read command source");
        let syntax = syn::parse_file(&source).expect("parse command source");
        for item in syntax.items {
            let syn::Item::Fn(function) = item else {
                continue;
            };
            if !function.attrs.iter().any(is_tauri_command) {
                continue;
            }
            let relative = source_path
                .strip_prefix(&command_root)
                .expect("command source is under command root");
            let mut segments = relative
                .parent()
                .into_iter()
                .flat_map(Path::components)
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>();
            segments.push(
                relative
                    .file_stem()
                    .expect("command source has a file stem")
                    .to_string_lossy()
                    .into_owned(),
            );
            segments.push(function.sig.ident.to_string());
            let handler = format!("crate::commands::{}", segments.join("::"));
            let registration_count = registry
                .lines()
                .filter(|line| line.trim().trim_end_matches(',') == handler)
                .count();
            assert_eq!(
                registration_count, 1,
                "Tauri command registration must contain {handler} exactly once"
            );
        }
    }
}

fn collect_rust_sources(directory: &Path, sources: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read command directory") {
        let path = entry.expect("read command entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    sources.sort();
}

fn is_tauri_command(attribute: &syn::Attribute) -> bool {
    attribute
        .path()
        .segments
        .iter()
        .map(|segment| segment.ident.to_string())
        .eq(["tauri", "command"].into_iter().map(str::to_string))
}

#[test]
fn mcp_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-mcp-client.ts");
    for command in [
        "list_mcp_servers",
        "add_mcp_server",
        "update_mcp_server",
        "remove_mcp_server",
        "toggle_mcp_server",
        "test_mcp_connection",
        "get_mcp_server_status",
        "import_mcp_servers",
        "export_mcp_servers",
    ] {
        assert!(
            native_registration.contains(&format!("::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}

#[test]
fn cli_config_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-agent-client.ts");
    for command in [
        "list_cli_config_profiles",
        "get_cli_config_status",
        "save_cli_config_profile",
        "duplicate_cli_config_profile",
        "delete_cli_config_profile",
        "import_cli_config_profile",
        "discover_cli_config_profiles",
        "import_discovered_cli_config_profiles",
        "apply_cli_config_profile",
        "validate_cli_config_credential",
    ] {
        assert!(
            native_registration.contains(&format!("::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}

#[test]
fn extension_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-extension-client.ts");
    for command in [
        "get_extension_overview",
        "refresh_extension_health",
        "get_extension_install_preview",
        "install_extension",
        "uninstall_extension",
        "set_extension_enabled",
        "start_extension",
        "stop_extension",
        "test_extension",
    ] {
        assert!(
            native_registration.contains(&format!("::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}

#[test]
fn workspace_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-agent-client.ts");
    for command in [
        "list_known_projects",
        "list_known_remote_workspaces",
        "inspect_project",
    ] {
        assert!(
            native_registration.contains(&format!("::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
    assert!(native_registration
        .contains("commands::workspaces::select_project_directory::select_project_directory"));
}

#[test]
fn workspace_query_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-session-workspace-client.ts");
    for command in [
        "list_session_directory",
        "read_session_file",
        "list_session_documents",
        "get_session_git_status",
        "get_session_git_diff",
        "list_session_logs",
        "export_session_logs",
    ] {
        assert!(
            native_registration.contains(&format!("commands::workspaces::{command}::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}

/// The whole session-log command surface, in one place.
///
/// Ten commands answer questions about logs, and they have to stay one surface: a page, the record
/// behind a live notice, the summary badge, where a subscriber resumes, what an export may read,
/// coverage, and the three that drive a repair. Registering nine of them is not a compile error and
/// not a runtime error either — the tenth simply is not there, and the caller sees a generic
/// "command not found" from a UI that has no way to say which capability went missing.
///
/// Registration path is asserted per command because the module is what carries the ownership: an
/// index command registered from somewhere other than `session_log_index` would be a second
/// implementation of a question this surface already answers.
#[test]
fn session_log_command_registration_covers_the_whole_index_surface() {
    let native_registration = command_registration_source();
    for command in [
        "get_session_log_record",
        "get_session_log_summary",
        "get_session_log_subscription_bootstrap",
        "get_session_log_export_sources",
        "get_session_log_coverage",
        "get_session_log_repair_status",
        "repair_session_log_index",
        "cancel_session_log_repair",
    ] {
        assert!(
            native_registration.contains(&format!(
                "commands::workspaces::session_log_index::{command}"
            )),
            "native command registration missing {command}"
        );
    }

    // The page and the export keep their own modules and their pre-migration names: the Logs tab
    // calls them by those names today, and the point of the migration was that it could not tell.
    for command in ["list_session_logs", "export_session_logs"] {
        assert!(
            native_registration.contains(&format!("commands::workspaces::{command}::{command}")),
            "native command registration missing {command}"
        );
    }
}

/// The export reads redacted files; the page reads the index. Neither borrows the other's source.
///
/// An export served from index rows would carry whatever the index happened to hold — a projection
/// that can be behind, partial, or repaired mid-read — and present it as the durable record. The
/// files are the durable record, so the export command is the one command that must not reach for
/// the index.
#[test]
fn the_export_command_reads_files_while_the_page_command_reads_the_index() {
    let export = include_str!("commands/workspaces/export_session_logs.rs");
    let page = include_str!("commands/workspaces/list_session_logs.rs");

    assert!(
        !export.contains("SessionLogApi") && !export.contains("session_log_mapper"),
        "the export command reached for the log index"
    );
    assert!(
        export.contains("WorkspaceApi"),
        "the export command no longer reads the redacted files"
    );
    assert!(
        page.contains("SessionLogApi"),
        "the page command no longer reads the index"
    );
    // The absent fallback is the assertion: a page that could quietly fall back to scanning would
    // answer with different filters and different bounds exactly when the index was in trouble,
    // and the reader would have no way to tell which implementation replied.
    assert!(
        !page.contains("WorkspaceApi"),
        "the page command kept a file-scan fallback"
    );
}

/// The retained Session Shell commands live in one grouped module, so their registration path is
/// `session_shell::<command>` rather than a module per command.
#[test]
fn session_shell_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-session-shell-client.ts");
    for command in [
        "list_session_shells",
        "create_session_shell",
        "attach_session_shell",
        "detach_session_shell",
        "write_session_shell",
        "resize_session_shell",
        "rename_session_shell",
        "close_session_shell",
    ] {
        assert!(
            native_registration
                .contains(&format!("commands::workspaces::session_shell::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}

#[test]
fn agent_runtime_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-agent-client.ts");
    for command in [
        "list_agents",
        "list_onepiece_provider_presets",
        "discover_onepiece_provider_models",
        "validate_onepiece_provider_credential",
        "get_agent_by_id",
        "get_workflow_state",
        "select_agent",
        "check_browser_readiness",
        "launch_active_workflow",
        "get_session_details",
        "send_message",
        "stop_generation",
    ] {
        assert!(
            native_registration.contains(&format!("commands::agent_runtime::{command}::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }

    let preset_command = include_str!("commands/agent_runtime/list_onepiece_provider_presets.rs");
    assert!(preset_command.contains("State<'_, AgentRuntimeApi>"));
    assert!(!preset_command.contains("Arc<AgentRuntimeApi>"));
}

#[test]
fn plugin_integration_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = command_registration_source();
    let tauri_client = include_str!("../../src/services/tauri-plugin-integration-client.ts");
    for command in [
        "get_plugin_integration_overview",
        "refresh_plugin_integrations",
        "test_plugin_integration",
    ] {
        assert!(
            native_registration.contains(&format!("::{command}")),
            "native command registration missing {command}"
        );
        assert!(
            tauri_client.contains(&format!("\"{command}\"")),
            "frontend invoke missing {command}"
        );
    }
}
