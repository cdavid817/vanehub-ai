import { invoke } from "@tauri-apps/api/core";
import type { McpService } from "./mcp-service";
import { parseMcpImportText } from "./mcp-import";
import { unsupportedRuntimeError } from "./service-error";
import type {
  McpErrorCode,
  McpImportExport,
  McpImportResult,
  McpScope,
  McpServerConfig,
  McpServerStatus,
  PartialMcpServerConfig,
} from "../types/mcp";
import { MCP_ERROR_CODES } from "../types/mcp";
import type { OperationTask } from "../types/operation";

type NativeMcpServerStatus = Omit<McpServerStatus, "errorCode"> & { errorCode?: unknown };

function mapMcpErrorCode(value: unknown): McpErrorCode | null | undefined {
  if (value === null || value === undefined) return value;
  return typeof value === "string" && (MCP_ERROR_CODES as readonly string[]).includes(value)
    ? (value as McpErrorCode)
    : null;
}

function mapServerStatus(status: NativeMcpServerStatus): McpServerStatus {
  return { ...status, errorCode: mapMcpErrorCode(status.errorCode) };
}

function mapConnectionOperation(operation: OperationTask): OperationTask {
  if (operation.kind !== "mcp" || !operation.result) return operation;
  return {
    ...operation,
    result: {
      ...operation.result,
      errorCode: mapMcpErrorCode(operation.result.errorCode) ?? null,
    },
  };
}

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

  async testConnection(name: string) {
    return mapConnectionOperation(await invoke<OperationTask>("test_mcp_connection", { name }));
  },

  async getServerStatus(name: string) {
    return mapServerStatus(await invoke<NativeMcpServerStatus>("get_mcp_server_status", { name }));
  },

  callTool() {
    return Promise.reject(unsupportedRuntimeError("MCP tool calling is reserved for a later VaneHub release."));
  },

  async importServers(input: string, scope: McpScope) {
    const parsed = parseMcpImportText(input);
    const result = await invoke<McpImportResult>("import_mcp_servers", { data: parsed.data, scope });
    return { ...result, failures: [...parsed.failures, ...(result.failures ?? [])] };
  },

  exportServers(names: string[]) {
    return invoke<McpImportExport>("export_mcp_servers", { names });
  },
};
