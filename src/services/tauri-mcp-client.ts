import { invoke } from "@tauri-apps/api/core";
import type { McpService } from "./mcp-service";
import { unsupportedRuntimeError } from "./service-error";
import type {
  McpImportExport,
  McpImportResult,
  McpScope,
  McpServerConfig,
  McpServerStatus,
  PartialMcpServerConfig,
} from "../types/mcp";
import type { OperationTask } from "../types/operation";

export const tauriMcpClient: McpService = {
  listServers() {
    return invoke<McpServerConfig[]>("list_mcp_servers");
  },

  addServer(config) {
    return invoke<void>("add_mcp_server", { config });
  },

  updateServer(name: string, config: PartialMcpServerConfig) {
    return invoke<void>("update_mcp_server", { name, config });
  },

  removeServer(name: string) {
    return invoke<void>("remove_mcp_server", { name });
  },

  toggleServer(name: string, active: boolean) {
    return invoke<void>("toggle_mcp_server", { name, active });
  },

  testConnection(name: string) {
    return invoke<OperationTask>("test_mcp_connection", { name });
  },

  getServerStatus(name: string) {
    return invoke<McpServerStatus>("get_mcp_server_status", { name });
  },

  callTool() {
    return Promise.reject(unsupportedRuntimeError("MCP tool calling is reserved for a later VaneHub release."));
  },

  importServers(data: McpImportExport, scope: McpScope) {
    return invoke<McpImportResult>("import_mcp_servers", { data, scope });
  },

  exportServers(names: string[]) {
    return invoke<McpImportExport>("export_mcp_servers", { names });
  },
};
