use super::dto::{
    McpConnectionStatus, McpImportExport, McpImportResult, McpImportServerEntry, McpScope,
    McpServerConfig, McpServerStatus, McpToolInfo, McpTransportType, PartialMcpServerConfig,
};
use crate::contexts::operations::api::{OperationKind, OperationTask};
use crate::contexts::operations::domain::OperationStatus;
use crate::contexts::tooling::mcp::api::{
    ConnectionStatus, ImportBundle, ImportEntry, ImportResult, Scope, ServerConfiguration,
    ServerConfigurationDraft, ServerPatch, ServerStatus, StartedOperation, ToolDescriptor,
    TransportType,
};
use std::collections::BTreeMap;

pub(super) fn server_draft(config: McpServerConfig) -> ServerConfigurationDraft {
    ServerConfigurationDraft {
        name: config.name,
        transport_type: transport_to_domain(config.transport_type),
        command: config.command,
        args: config.args,
        env: config.env,
        url: config.url,
        headers: config.headers,
        description: config.description,
        active: config.active,
        scope: scope_to_domain(config.scope),
        project_path: config.project_path,
    }
}

pub(super) fn server_patch(config: PartialMcpServerConfig) -> ServerPatch {
    ServerPatch {
        name: config.name,
        transport_type: config.transport_type.map(transport_to_domain),
        command: config.command,
        args: config.args,
        env: config.env,
        url: config.url,
        headers: config.headers,
        description: config.description,
        active: config.active,
        scope: config.scope.map(scope_to_domain),
    }
}

pub(super) fn scope_to_domain(scope: McpScope) -> Scope {
    match scope {
        McpScope::User => Scope::User,
        McpScope::Project => Scope::Project,
    }
}

pub(super) fn servers_to_dto(servers: Vec<ServerConfiguration>) -> Vec<McpServerConfig> {
    servers.into_iter().map(server_to_dto).collect()
}

pub(super) fn status_to_dto(status: ServerStatus) -> McpServerStatus {
    McpServerStatus {
        name: status.name.as_str().to_string(),
        connection_status: match status.connection_status {
            ConnectionStatus::Connected => McpConnectionStatus::Connected,
            ConnectionStatus::Disconnected => McpConnectionStatus::Disconnected,
            ConnectionStatus::Error => McpConnectionStatus::Error,
            ConnectionStatus::Disabled => McpConnectionStatus::Disabled,
        },
        tools: status.tools.iter().map(tool_to_dto).collect(),
        last_connected: status.last_connected,
        error: status.error,
        duration_ms: status.duration_ms,
    }
}

pub(super) fn import_bundle(data: McpImportExport) -> ImportBundle {
    ImportBundle {
        servers: data
            .mcp_servers
            .into_iter()
            .map(|(name, entry)| {
                (
                    name,
                    ImportEntry {
                        command: entry.command,
                        args: entry.args,
                        env: entry.env,
                        url: entry.url,
                        headers: entry.headers,
                    },
                )
            })
            .collect(),
    }
}

pub(super) fn import_result_to_dto(result: ImportResult) -> McpImportResult {
    McpImportResult {
        imported: result.imported,
        skipped: result.skipped,
    }
}

pub(super) fn export_bundle_to_dto(bundle: ImportBundle) -> McpImportExport {
    McpImportExport {
        mcp_servers: bundle
            .servers
            .into_iter()
            .map(|(name, entry)| {
                (
                    name,
                    McpImportServerEntry {
                        command: entry.command,
                        args: entry.args,
                        env: entry.env,
                        url: entry.url,
                        headers: entry.headers,
                    },
                )
            })
            .collect::<BTreeMap<_, _>>(),
    }
}

pub(super) fn started_operation_to_dto(operation: &StartedOperation) -> OperationTask {
    OperationTask {
        id: operation.id.clone(),
        kind: OperationKind::Mcp,
        status: OperationStatus::Running,
        related_entity_id: operation.related_entity_id.clone(),
        message: operation.message.clone(),
        logs: Vec::new(),
        result: None,
        error: None,
        created_at: operation.created_at.clone(),
        updated_at: operation.updated_at.clone(),
    }
}

fn server_to_dto(server: ServerConfiguration) -> McpServerConfig {
    McpServerConfig {
        name: server.name().as_str().to_string(),
        transport_type: match server.transport_type() {
            TransportType::Stdio => McpTransportType::Stdio,
            TransportType::Sse => McpTransportType::Sse,
            TransportType::StreamableHttp => McpTransportType::StreamableHttp,
        },
        command: server.command().map(str::to_string),
        args: server.args().map(<[String]>::to_vec),
        env: server.env().cloned(),
        url: server.url().map(str::to_string),
        headers: server.headers().cloned(),
        description: server.description().map(str::to_string),
        active: server.is_active(),
        scope: match server.scope() {
            Scope::User => McpScope::User,
            Scope::Project => McpScope::Project,
        },
        project_path: server.project_path().map(str::to_string),
    }
}

fn tool_to_dto(tool: &ToolDescriptor) -> McpToolInfo {
    McpToolInfo {
        name: tool.name.clone(),
        description: tool.description.clone(),
        input_schema: tool.input_schema.clone(),
    }
}

fn transport_to_domain(transport: McpTransportType) -> TransportType {
    match transport {
        McpTransportType::Stdio => TransportType::Stdio,
        McpTransportType::Sse => TransportType::Sse,
        McpTransportType::StreamableHttp => TransportType::StreamableHttp,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::mcp::api::{ConnectionStatus, ServerStatus};
    use crate::contexts::tooling::mcp::domain::ServerName;
    use serde_json::json;

    #[test]
    fn tauri_request_dto_accepts_the_existing_camel_case_shape() {
        let dto: McpServerConfig = serde_json::from_value(json!({
            "name": "fixture-tools",
            "transportType": "stdio",
            "command": "node",
            "args": ["server.js"],
            "env": { "MODE": "fixture" },
            "url": null,
            "headers": null,
            "description": "Fixture",
            "active": true,
            "scope": "project",
            "projectPath": "D:\\code\\fixture"
        }))
        .expect("request DTO");

        let draft = server_draft(dto);

        assert_eq!(draft.transport_type, TransportType::Stdio);
        assert_eq!(draft.scope, Scope::Project);
        assert_eq!(draft.args, Some(vec!["server.js".to_string()]));
        assert_eq!(draft.project_path.as_deref(), Some("D:\\code\\fixture"));
    }

    #[test]
    fn tauri_status_response_keeps_values_camel_case_and_nullable_fields() {
        let status = status_to_dto(ServerStatus {
            name: ServerName::parse("fixture-tools").expect("name"),
            connection_status: ConnectionStatus::Connected,
            tools: vec![ToolDescriptor {
                name: "search".to_string(),
                description: None,
                input_schema: Some(json!({ "type": "object" })),
            }],
            last_connected: Some("1700000000".to_string()),
            error: None,
            duration_ms: Some(17),
        });

        let value = serde_json::to_value(status).expect("status DTO");

        assert_eq!(value["connectionStatus"], "connected");
        assert_eq!(value["tools"][0]["inputSchema"]["type"], "object");
        assert_eq!(value["lastConnected"], "1700000000");
        assert_eq!(value["durationMs"], 17);
        assert!(value.get("connection_status").is_none());
        assert!(value["error"].is_null());
    }

    #[test]
    fn started_connection_operation_preserves_the_frontend_task_contract() {
        let operation = started_operation_to_dto(&StartedOperation {
            id: "op-fixed".to_string(),
            related_entity_id: Some("fixture-tools".to_string()),
            message: Some("Testing MCP server fixture-tools".to_string()),
            created_at: "100".to_string(),
            updated_at: "100".to_string(),
        });

        let value = serde_json::to_value(operation).expect("operation DTO");

        assert_eq!(value["id"], "op-fixed");
        assert_eq!(value["kind"], "mcp");
        assert_eq!(value["status"], "running");
        assert_eq!(value["relatedEntityId"], "fixture-tools");
        assert_eq!(value["logs"], json!([]));
        assert!(value["result"].is_null());
        assert!(value["error"].is_null());
    }

    #[test]
    fn import_export_mapping_keeps_the_mcp_servers_envelope() {
        let dto: McpImportExport = serde_json::from_value(json!({
            "mcpServers": {
                "fixture-tools": {
                    "command": "node",
                    "args": ["server.js"]
                }
            }
        }))
        .expect("import DTO");

        let round_trip = export_bundle_to_dto(import_bundle(dto));
        let value = serde_json::to_value(round_trip).expect("export DTO");

        assert_eq!(value["mcpServers"]["fixture-tools"]["command"], "node");
        assert_eq!(
            value["mcpServers"]["fixture-tools"]["args"],
            json!(["server.js"])
        );
        assert!(value.get("mcp_servers").is_none());
    }
}
