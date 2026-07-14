import type {
  McpImportExport,
  McpImportResult,
  McpScope,
  McpServerConfig,
  McpServerStatus,
  McpTestResult,
  PartialMcpServerConfig,
} from "../types/mcp";

export interface McpService {
  listServers(): Promise<McpServerConfig[]>;
  addServer(config: McpServerConfig): Promise<void>;
  updateServer(name: string, config: PartialMcpServerConfig): Promise<void>;
  removeServer(name: string): Promise<void>;
  toggleServer(name: string, active: boolean): Promise<void>;
  testConnection(name: string): Promise<McpTestResult>;
  getServerStatus(name: string): Promise<McpServerStatus>;
  callTool(serverName: string, toolName: string, args?: Record<string, unknown>): Promise<unknown>;
  importServers(data: McpImportExport, scope: McpScope): Promise<McpImportResult>;
  exportServers(names: string[]): Promise<McpImportExport>;
}
