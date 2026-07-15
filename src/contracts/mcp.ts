export type McpTransportType = "stdio" | "sse" | "streamable_http";
export type McpConnectionStatus = "connected" | "disconnected" | "error" | "disabled";
export type McpScope = "user" | "project";

export interface McpServerConfig {
  name: string;
  transportType: McpTransportType;
  command?: string | null;
  args?: string[] | null;
  env?: Record<string, string> | null;
  url?: string | null;
  headers?: Record<string, string> | null;
  description?: string | null;
  active: boolean;
  scope: McpScope;
  projectPath?: string | null;
}

export type PartialMcpServerConfig = Partial<Omit<McpServerConfig, "args" | "env" | "headers">> & {
  args?: string[] | null;
  env?: Record<string, string> | null;
  headers?: Record<string, string> | null;
};

export interface McpToolInfo {
  name: string;
  description?: string | null;
  inputSchema?: Record<string, unknown> | null;
}

export interface McpServerStatus {
  name: string;
  connectionStatus: McpConnectionStatus;
  tools: McpToolInfo[];
  lastConnected?: string | null;
  error?: string | null;
  durationMs?: number | null;
}

export interface McpTestResult {
  success: boolean;
  operationId?: string | null;
  tools: McpToolInfo[];
  error?: string | null;
  durationMs?: number | null;
}

export interface McpImportResult {
  imported: string[];
  skipped: string[];
}

export interface McpImportServerEntry {
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  headers?: Record<string, string>;
}

export interface McpImportExport {
  mcpServers: Record<string, McpImportServerEntry>;
}
