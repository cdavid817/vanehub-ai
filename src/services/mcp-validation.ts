import type { McpService } from "./mcp-service";
import {
  MCP_LIMITS,
  type McpErrorCode,
  type McpServerConfig,
  type McpTransportType,
  type PartialMcpServerConfig,
} from "../types/mcp";

const encoder = new TextEncoder();
const transportTypes = ["stdio", "sse", "streamable_http"] as const;

export type McpValidationField =
  | "name"
  | "transportType"
  | "command"
  | "url"
  | "args"
  | "env"
  | "headers"
  | "arguments"
  | "catalog"
  | "result"
  | "import"
  | "form";

export type McpValidationFailure = {
  success: false;
  errorCode: Extract<McpErrorCode, "validation" | "limit_exceeded">;
  field: McpValidationField;
  message: string;
};

export type McpValidationResult = { success: true } | McpValidationFailure;

export class McpValidationError extends Error {
  readonly errorCode: McpValidationFailure["errorCode"];
  readonly field: McpValidationField;

  constructor(failure: McpValidationFailure) {
    super(failure.message);
    this.name = "McpValidationError";
    this.errorCode = failure.errorCode;
    this.field = failure.field;
  }
}

export function isMcpTransportType(value: unknown): value is McpTransportType {
  return typeof value === "string" && (transportTypes as readonly string[]).includes(value);
}

export function mcpTransportLabel(transportType: McpTransportType): string {
  switch (transportType) {
    case "stdio":
      return "stdio";
    case "sse":
      return "legacy SSE";
    case "streamable_http":
      return "Streamable HTTP";
  }
}

export function validateMcpServerConfig(
  value: unknown,
  options: { partial?: boolean } = {},
): McpValidationResult {
  if (!isPlainRecord(value)) {
    return invalid("form", "MCP server configuration must be an object");
  }

  const partial = options.partial === true;
  const transportType = value.transportType;
  if ((!partial || transportType !== undefined) && !isMcpTransportType(transportType)) {
    return invalid("transportType", "MCP transport must be stdio, legacy SSE, or Streamable HTTP");
  }

  if (!partial || value.name !== undefined) {
    if (typeof value.name !== "string" || !/^[a-z0-9]+(?:-[a-z0-9]+)*$/.test(value.name.trim())) {
      return invalid("name", "MCP server name must use kebab-case lowercase letters, numbers, and hyphens");
    }
  }

  if (!partial && transportType === "stdio" && !nonEmptyString(value.command)) {
    return invalid("command", "stdio MCP server requires a command");
  }
  if (!partial && (transportType === "sse" || transportType === "streamable_http") && !nonEmptyString(value.url)) {
    return invalid("url", `${mcpTransportLabel(transportType)} MCP server requires a URL`);
  }

  for (const field of ["command", "url"] as const) {
    const fieldValue = value[field];
    if (fieldValue !== undefined && fieldValue !== null && typeof fieldValue !== "string") {
      return invalid(field, `MCP ${field} must be a string`);
    }
  }

  const args = validateStringArray(value.args, "args");
  if (!args.success) return args;
  const env = validateStringRecord(value.env, "env");
  if (!env.success) return env;
  const headers = validateStringRecord(value.headers, "headers");
  if (!headers.success) return headers;

  const serialized = serializeTransportConfiguration(value);
  if (!serialized.success) return serialized;
  if (serialized.byteLength > MCP_LIMITS.configurationSerializedBytes) {
    return limitExceeded(
      "form",
      `MCP transport configuration exceeds ${MCP_LIMITS.configurationSerializedBytes} UTF-8 bytes`,
    );
  }

  return { success: true };
}

export function validateMcpToolArguments(value: unknown): McpValidationResult {
  if (value === undefined || value === null) return { success: true };
  if (!isPlainRecord(value)) {
    return invalid("arguments", "MCP tool arguments must be a JSON object or null");
  }

  const jsonShape = validateJsonShapeAndDepth(value, MCP_LIMITS.jsonDepth);
  if (!jsonShape.success) return jsonShape;

  try {
    const byteLength = encoder.encode(JSON.stringify(value)).byteLength;
    if (byteLength > MCP_LIMITS.toolArgumentsBytes) {
      return limitExceeded(
        "arguments",
        `MCP tool arguments exceed ${MCP_LIMITS.toolArgumentsBytes} UTF-8 bytes`,
      );
    }
  } catch {
    return invalid("arguments", "MCP tool arguments must be valid JSON");
  }

  return { success: true };
}

export function withMcpServiceValidation(service: McpService): McpService {
  return {
    listServers: () => service.listServers(),
    async addServer(config: McpServerConfig) {
      assertValid(validateMcpServerConfig(config));
      await service.addServer(config);
    },
    async updateServer(name: string, config: PartialMcpServerConfig) {
      assertValid(validateMcpServerConfig(config, { partial: true }));
      await service.updateServer(name, config);
    },
    removeServer: (name) => service.removeServer(name),
    toggleServer: (name, active) => service.toggleServer(name, active),
    testConnection: (name) => service.testConnection(name),
    getServerStatus: (name) => service.getServerStatus(name),
    async callTool(serverName, toolName, args) {
      assertValid(validateMcpToolArguments(args));
      return service.callTool(serverName, toolName, args);
    },
    importServers: (input, scope) => service.importServers(input, scope),
    exportServers: (names) => service.exportServers(names),
  };
}

function validateStringArray(value: unknown, field: "args"): McpValidationResult {
  if (value === undefined || value === null) return { success: true };
  if (!Array.isArray(value) || value.some((item) => typeof item !== "string")) {
    return invalid(field, "MCP args must contain only strings");
  }
  if (value.length > MCP_LIMITS.configurationCollectionEntries) {
    return limitExceeded(field, `MCP args exceeds ${MCP_LIMITS.configurationCollectionEntries} entries`);
  }
  return { success: true };
}

function validateStringRecord(value: unknown, field: "env" | "headers"): McpValidationResult {
  if (value === undefined || value === null) return { success: true };
  if (!isPlainRecord(value) || Object.values(value).some((item) => typeof item !== "string")) {
    return invalid(field, `MCP ${field} values must contain only strings`);
  }
  if (Object.keys(value).length > MCP_LIMITS.configurationCollectionEntries) {
    return limitExceeded(field, `MCP ${field} exceeds ${MCP_LIMITS.configurationCollectionEntries} entries`);
  }
  return { success: true };
}

function serializeTransportConfiguration(
  value: Record<string, unknown>,
): { success: true; byteLength: number } | McpValidationFailure {
  try {
    const serialized = JSON.stringify({
      transportType: value.transportType ?? null,
      command: value.command ?? null,
      args: value.args ?? null,
      env: value.env ?? null,
      url: value.url ?? null,
      headers: value.headers ?? null,
    });
    return { success: true, byteLength: encoder.encode(serialized).byteLength };
  } catch {
    return invalid("form", "MCP transport configuration must be valid JSON");
  }
}

function validateJsonShapeAndDepth(value: unknown, maximumDepth: number): McpValidationResult {
  const pending: Array<{ value: unknown; depth: number }> = [{ value, depth: 1 }];
  const visited = new WeakSet<object>();

  while (pending.length) {
    const current = pending.pop();
    if (!current) break;
    if (current.depth > maximumDepth) {
      return limitExceeded("arguments", `MCP tool arguments exceed JSON depth ${maximumDepth}`);
    }
    if (current.value === null || typeof current.value === "string" || typeof current.value === "boolean") {
      continue;
    }
    if (typeof current.value === "number") {
      if (!Number.isFinite(current.value)) return invalid("arguments", "MCP tool arguments must be valid JSON");
      continue;
    }
    if (typeof current.value !== "object") {
      return invalid("arguments", "MCP tool arguments must be valid JSON");
    }
    if (visited.has(current.value)) return invalid("arguments", "MCP tool arguments must be valid JSON");
    visited.add(current.value);

    if (Array.isArray(current.value)) {
      pending.push(...current.value.map((item) => ({ value: item, depth: current.depth + 1 })));
      continue;
    }
    if (!isPlainRecord(current.value)) return invalid("arguments", "MCP tool arguments must be valid JSON");
    pending.push(...Object.values(current.value).map((item) => ({ value: item, depth: current.depth + 1 })));
  }

  return { success: true };
}

function assertValid(result: McpValidationResult): asserts result is { success: true } {
  if (!result.success) throw new McpValidationError(result);
}

function nonEmptyString(value: unknown): value is string {
  return typeof value === "string" && value.trim().length > 0;
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value) as object | null;
  return prototype === Object.prototype || prototype === null;
}

function invalid(field: McpValidationField, message: string): McpValidationFailure {
  return { success: false, errorCode: "validation", field, message };
}

function limitExceeded(field: McpValidationField, message: string): McpValidationFailure {
  return { success: false, errorCode: "limit_exceeded", field, message };
}
