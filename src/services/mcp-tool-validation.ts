import { MCP_LIMITS } from "../types/mcp";
import type { McpValidationField, McpValidationResult } from "./mcp-validation";

const encoder = new TextEncoder();

export function validateMcpToolCatalog(value: unknown): McpValidationResult {
  if (!Array.isArray(value)) return invalid("catalog", "MCP tool catalog must be an array");
  if (value.length > MCP_LIMITS.toolsPerServer) {
    return limitExceeded("catalog", `MCP tool catalog exceeds ${MCP_LIMITS.toolsPerServer} tools`);
  }

  for (const tool of value) {
    const descriptor = validateToolDescriptor(tool);
    if (!descriptor.success) return descriptor;
  }

  const serialized = serializedBytes(value, "catalog", "MCP tool catalog must be valid JSON");
  if (!serialized.success) return serialized;
  if (serialized.byteLength > MCP_LIMITS.catalogSerializedBytes) {
    return limitExceeded(
      "catalog",
      `MCP tool catalog exceeds ${MCP_LIMITS.catalogSerializedBytes} UTF-8 bytes`,
    );
  }
  return { success: true };
}

export function validateMcpToolResult(value: unknown): McpValidationResult {
  if (typeof value !== "string") return invalid("result", "Rendered MCP tool result must be text");
  if (encoder.encode(value).byteLength > MCP_LIMITS.toolResultBytes) {
    return limitExceeded("result", `MCP tool result exceeds ${MCP_LIMITS.toolResultBytes} UTF-8 bytes`);
  }
  return { success: true };
}

function validateToolDescriptor(value: unknown): McpValidationResult {
  if (!isPlainRecord(value) || typeof value.name !== "string" || value.name.length === 0) {
    return invalid("catalog", "MCP tool descriptor requires a non-empty name");
  }
  if (encoder.encode(value.name).byteLength > MCP_LIMITS.toolNameBytes) {
    return limitExceeded("catalog", `MCP tool name exceeds ${MCP_LIMITS.toolNameBytes} UTF-8 bytes`);
  }

  if (value.description !== undefined && value.description !== null) {
    if (typeof value.description !== "string") {
      return invalid("catalog", "MCP tool description must be text");
    }
    if (encoder.encode(value.description).byteLength > MCP_LIMITS.toolDescriptionBytes) {
      return limitExceeded(
        "catalog",
        `MCP tool description exceeds ${MCP_LIMITS.toolDescriptionBytes} UTF-8 bytes`,
      );
    }
  }

  if (value.inputSchema !== undefined && value.inputSchema !== null) {
    if (!isPlainRecord(value.inputSchema)) {
      return invalid("catalog", "MCP tool input schema must be a JSON object");
    }
    const shape = validateJsonShapeAndDepth(value.inputSchema, MCP_LIMITS.jsonDepth);
    if (!shape.success) return shape;
    const serialized = serializedBytes(value.inputSchema, "catalog", "MCP tool input schema must be valid JSON");
    if (!serialized.success) return serialized;
    if (serialized.byteLength > MCP_LIMITS.schemaBytes) {
      return limitExceeded("catalog", `MCP tool input schema exceeds ${MCP_LIMITS.schemaBytes} UTF-8 bytes`);
    }
  }
  return { success: true };
}

function validateJsonShapeAndDepth(value: unknown, maximumDepth: number): McpValidationResult {
  const pending: Array<{ value: unknown; depth: number }> = [{ value, depth: 1 }];
  const visited = new WeakSet<object>();
  while (pending.length) {
    const current = pending.pop();
    if (!current) break;
    if (current.depth > maximumDepth) {
      return limitExceeded("catalog", `MCP tool input schema exceeds JSON depth ${maximumDepth}`);
    }
    if (current.value === null || typeof current.value === "string" || typeof current.value === "boolean") continue;
    if (typeof current.value === "number") {
      if (!Number.isFinite(current.value)) return invalid("catalog", "MCP tool input schema must be valid JSON");
      continue;
    }
    if (typeof current.value !== "object" || visited.has(current.value)) {
      return invalid("catalog", "MCP tool input schema must be valid JSON");
    }
    visited.add(current.value);
    if (Array.isArray(current.value)) {
      pending.push(...current.value.map((item) => ({ value: item, depth: current.depth + 1 })));
    } else if (isPlainRecord(current.value)) {
      pending.push(...Object.values(current.value).map((item) => ({ value: item, depth: current.depth + 1 })));
    } else {
      return invalid("catalog", "MCP tool input schema must be valid JSON");
    }
  }
  return { success: true };
}

function serializedBytes(
  value: unknown,
  field: McpValidationField,
  message: string,
): { success: true; byteLength: number } | Exclude<McpValidationResult, { success: true }> {
  try {
    return { success: true, byteLength: encoder.encode(JSON.stringify(value)).byteLength };
  } catch {
    return invalid(field, message);
  }
}

function isPlainRecord(value: unknown): value is Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  const prototype = Object.getPrototypeOf(value) as object | null;
  return prototype === Object.prototype || prototype === null;
}

function invalid(field: McpValidationField, message: string): Exclude<McpValidationResult, { success: true }> {
  return { success: false, errorCode: "validation", field, message };
}

function limitExceeded(
  field: McpValidationField,
  message: string,
): Exclude<McpValidationResult, { success: true }> {
  return { success: false, errorCode: "limit_exceeded", field, message };
}
