import type { TFunction } from "i18next";
import { MCP_ERROR_CODES, type McpErrorCode, type McpTransportType } from "../../../types/mcp";

export function mcpTransportTranslationKey(
  transportType: McpTransportType,
): "mcp.transport.stdio" | "mcp.transport.sse" | "mcp.transport.streamableHttp" {
  switch (transportType) {
    case "stdio":
      return "mcp.transport.stdio";
    case "sse":
      return "mcp.transport.sse";
    case "streamable_http":
      return "mcp.transport.streamableHttp";
  }
}

export function mcpErrorCodeTranslationKey(errorCode: McpErrorCode): string {
  return `mcp.error.code.${errorCode}`;
}

export function mcpErrorCodeFromUnknown(value: unknown): McpErrorCode | null {
  return typeof value === "string" && (MCP_ERROR_CODES as readonly string[]).includes(value)
    ? (value as McpErrorCode)
    : null;
}

export function formatMcpFailure(
  t: TFunction,
  errorCode: McpErrorCode | null | undefined,
  safeMessage?: string | null,
): string {
  if (!errorCode) return t("mcp.error.unknown");
  const label = t(mcpErrorCodeTranslationKey(errorCode));
  const detail = safeMessage?.trim();
  return detail ? `${label} [${errorCode}]: ${detail}` : `${label} [${errorCode}]`;
}

export function mcpErrorFromUnknown(error: unknown): { errorCode: McpErrorCode | null; message: string | null } {
  if (!error || typeof error !== "object") return { errorCode: null, message: null };
  const errorCode = mcpErrorCodeFromUnknown("errorCode" in error ? error.errorCode : null);
  if (!errorCode) return { errorCode: null, message: null };
  return {
    errorCode,
    message: error instanceof Error && error.message.trim() ? error.message.trim() : null,
  };
}
