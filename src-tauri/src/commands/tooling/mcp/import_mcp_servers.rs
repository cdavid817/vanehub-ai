use super::dto::{McpImportExport, McpImportResult, McpScope};
use super::mapper;
use crate::commands::error::{map_command_error, CommandError};
use crate::contexts::tooling::mcp::api::{McpApi, McpError, McpLimits};
use tauri::State;

#[tauri::command]
pub(crate) fn import_mcp_servers(
    api: State<'_, McpApi>,
    data: McpImportExport,
    scope: McpScope,
) -> Result<McpImportResult, CommandError> {
    validate_import_boundary(&data)?;
    api.import_servers(mapper::import_bundle(data), mapper::scope_to_domain(scope))
        .map(mapper::import_result_to_dto)
        .map_err(map_command_error)
}

fn validate_import_boundary(data: &McpImportExport) -> Result<(), CommandError> {
    let limits = McpLimits::DEFAULT;
    let serialized = serde_json::to_vec(data).map_err(|_| {
        map_command_error(McpError::Validation(
            "Invalid MCP import document".to_string(),
        ))
    })?;
    if serialized.len() > limits.import_document_bytes
        || data.mcp_servers.len() > limits.import_server_entries
    {
        return Err(map_command_error(McpError::LimitExceeded));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::tooling::mcp::dto::McpImportServerEntry;
    use std::collections::BTreeMap;

    #[test]
    fn native_import_boundary_rejects_count_and_serialized_size_limits() {
        let too_many = McpImportExport {
            mcp_servers: (0..=McpLimits::DEFAULT.import_server_entries)
                .map(|index| (format!("server-{index}"), McpImportServerEntry::default()))
                .collect(),
        };
        assert!(validate_import_boundary(&too_many).is_err());

        let oversized = McpImportExport {
            mcp_servers: BTreeMap::from([(
                "oversized".to_string(),
                McpImportServerEntry {
                    command: Some("x".repeat(McpLimits::DEFAULT.import_document_bytes)),
                    ..Default::default()
                },
            )]),
        };
        assert!(validate_import_boundary(&oversized).is_err());
    }
}
