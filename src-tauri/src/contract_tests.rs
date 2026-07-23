use crate::commands::error::CommandError;
use crate::commands::tooling::mcp::dto::{McpScope, McpServerConfig, McpTransportType};
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

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
    })
    .expect("serialize operation");

    assert_eq!(value["kind"], "mcp");
    assert_eq!(value["status"], "running");
    assert_eq!(value["relatedEntityId"], "server-1");
    assert!(value.get("related_entity_id").is_none());
    assert!(value.get("executionRunId").is_none());
    assert!(value.get("traceId").is_none());
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
    let registry = include_str!("commands/registry.rs");
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
            assert_eq!(
                registry.matches(&handler).count(),
                1,
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
    let native_registration = include_str!("commands/registry.rs");
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
fn extension_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = include_str!("commands/registry.rs");
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
    let native_registration = include_str!("commands/registry.rs");
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
    let native_registration = include_str!("commands/registry.rs");
    let tauri_client = include_str!("../../src/services/tauri-session-workspace-client.ts");
    for command in [
        "list_session_directory",
        "read_session_file",
        "list_session_documents",
        "get_session_git_status",
        "get_session_git_diff",
        "list_session_logs",
        "export_session_logs",
        "shell_create",
        "shell_input",
        "shell_cd",
        "shell_resize",
        "shell_kill",
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

#[test]
fn agent_runtime_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = include_str!("commands/registry.rs");
    let tauri_client = include_str!("../../src/services/tauri-agent-client.ts");
    for command in [
        "list_agents",
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
}

#[test]
fn plugin_integration_command_registration_and_frontend_invokes_keep_stable_names() {
    let native_registration = include_str!("commands/registry.rs");
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
