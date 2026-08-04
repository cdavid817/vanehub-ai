export type McpTransportType = "stdio" | "sse" | "streamable_http";
export type McpConnectionStatus = "connected" | "disconnected" | "error" | "disabled";
export type McpScope = "user" | "project";
export type McpImportTransportType = "sse" | "http" | "streamable_http";

export const MCP_ERROR_CODES = [
  "validation",
  "spawn",
  "timeout",
  "cancelled",
  "protocol",
  "upstream_http",
  "limit_exceeded",
  "transport",
  "cleanup",
] as const;

export type McpErrorCode = (typeof MCP_ERROR_CODES)[number];

export const MCP_LIMITS = {
  importDocumentBytes: 1024 * 1024,
  importServerEntries: 128,
  configurationCollectionEntries: 128,
  configurationSerializedBytes: 256 * 1024,
  protocolMessageBytes: 2 * 1024 * 1024,
  toolsPerServer: 128,
  catalogSerializedBytes: 2 * 1024 * 1024,
  providerTools: 256,
  toolNameBytes: 256,
  toolDescriptionBytes: 8 * 1024,
  schemaBytes: 128 * 1024,
  jsonDepth: 32,
  toolArgumentsBytes: 256 * 1024,
  toolResultBytes: 1024 * 1024,
  stderrBytes: 64 * 1024,
} as const;

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
  errorCode?: McpErrorCode | null;
  durationMs?: number | null;
}

export interface McpTestResult {
  success: boolean;
  operationId?: string | null;
  tools: McpToolInfo[];
  error?: string | null;
  errorCode?: McpErrorCode | null;
  durationMs?: number | null;
}

export interface McpToolCallResult {
  content: string;
  isError: boolean;
  errorCode?: McpErrorCode | null;
}

export interface McpImportResult {
  imported: string[];
  skipped: string[];
  failures: McpImportFailure[];
}

export interface McpImportFailure {
  name: string;
  stage: "validation" | "storage";
  errorCode?: McpErrorCode | null;
  message: string;
}

export interface McpImportServerEntry {
  type?: McpImportTransportType;
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  url?: string;
  headers?: Record<string, string>;
}

export interface McpImportExport {
  mcpServers: Record<string, McpImportServerEntry>;
}
