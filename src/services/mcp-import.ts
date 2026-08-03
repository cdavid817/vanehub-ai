import { McpValidationError, validateMcpServerConfig } from "./mcp-validation";
import {
  MCP_LIMITS,
  type McpImportExport,
  type McpImportFailure,
  type McpImportServerEntry,
  type McpTransportType,
} from "../types/mcp";

const encoder = new TextEncoder();
const compatibleUrlTypes = ["sse", "http", "streamable_http"] as const;

export type ParsedMcpImport = {
  data: McpImportExport;
  failures: McpImportFailure[];
};

export function parseMcpImportText(input: string): ParsedMcpImport {
  if (encoder.encode(input).byteLength > MCP_LIMITS.importDocumentBytes) {
    throw importError(
      "limit_exceeded",
      `MCP import document exceeds ${MCP_LIMITS.importDocumentBytes} UTF-8 bytes`,
    );
  }

  let parsed: unknown;
  try {
    parsed = JSON.parse(input) as unknown;
  } catch {
    throw importError("validation", "MCP import document must be valid JSON");
  }
  if (!isPlainRecord(parsed) || !isPlainRecord(parsed.mcpServers)) {
    throw importError("validation", "MCP import document must contain an mcpServers object");
  }

  const entries = Object.entries(parsed.mcpServers);
  if (entries.length > MCP_LIMITS.importServerEntries) {
    throw importError(
      "limit_exceeded",
      `MCP import document exceeds ${MCP_LIMITS.importServerEntries} server entries`,
    );
  }

  const mcpServers: McpImportExport["mcpServers"] = {};
  const failures: McpImportFailure[] = [];
  for (const [name, value] of entries) {
    const entry = parseEntry(name, value);
    if (entry.success) mcpServers[name] = entry.value;
    else failures.push(entry.failure);
  }
  return { data: { mcpServers }, failures };
}

export function previewMcpImportNames(input: string): string[] {
  try {
    return Object.keys(parseMcpImportText(input).data.mcpServers);
  } catch {
    return [];
  }
}

function parseEntry(
  name: string,
  value: unknown,
): { success: true; value: McpImportServerEntry } | { success: false; failure: McpImportFailure } {
  if (!/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(name) || !isPlainRecord(value)) {
    return failedEntry(name, "MCP import entry has an invalid name or shape");
  }
  if (value.type !== undefined && !isCompatibleUrlType(value.type)) {
    return failedEntry(name, "MCP import entry has an unknown transport type");
  }
  if (value.command !== undefined && typeof value.command !== "string") {
    return failedEntry(name, "MCP import command must be a string");
  }

  const command = typeof value.command === "string" ? value.command.trim() : "";
  const transportType: McpTransportType = command
    ? "stdio"
    : value.type === "sse"
      ? "sse"
      : "streamable_http";
  const config = {
    name,
    transportType,
    command: transportType === "stdio" ? command : null,
    args: transportType === "stdio" ? value.args ?? null : null,
    env: transportType === "stdio" ? value.env ?? null : null,
    url: transportType === "stdio" ? null : value.url ?? null,
    headers: transportType === "stdio" ? null : value.headers ?? null,
    active: true,
    scope: "user",
  };
  const validation = validateMcpServerConfig(config);
  if (!validation.success) {
    return {
      success: false,
      failure: {
        name,
        stage: "validation",
        errorCode: validation.errorCode,
        message: validation.message,
      },
    };
  }

  return {
    success: true,
    value:
      transportType === "stdio"
        ? {
            command,
            args: config.args as string[] | null ?? undefined,
            env: config.env as Record<string, string> | null ?? undefined,
          }
        : {
            type: transportType === "sse" ? "sse" : "http",
            url: config.url as string,
            headers: config.headers as Record<string, string> | null ?? undefined,
          },
  };
}

function failedEntry(
  name: string,
  message: string,
): { success: false; failure: McpImportFailure } {
  return {
    success: false,
    failure: { name, stage: "validation", errorCode: "validation", message },
  };
}

function importError(errorCode: "validation" | "limit_exceeded", message: string): McpValidationError {
  return new McpValidationError({ success: false, errorCode, field: "import", message });
}

function isCompatibleUrlType(value: unknown): value is (typeof compatibleUrlTypes)[number] {
  return typeof value === "string" && (compatibleUrlTypes as readonly string[]).includes(value);
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value) as object | null;
  return prototype === Object.prototype || prototype === null;
}
