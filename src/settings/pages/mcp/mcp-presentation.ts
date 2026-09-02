import type { TFunction } from "i18next";
import { MCP_ERROR_CODES, type McpConnectionStatus, type McpErrorCode, type McpTransportType } from "../../../types/mcp";
import type { StatusTone } from "../../../ui/status/StatusBadge";

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

/** Shared by every `useMutation`'s own `onError` projection into `MutationState` (task 12.18) --
 *  unwraps a caught, unknown rejection through the same safe errorCode allowlist `formatMcpFailure`
 *  itself relies on, so a per-card Toggle/Delete/Test failure is never rendered with raw,
 *  unclassified error text either. */
export function mcpMutationErrorMessage(t: TFunction, error: unknown): string {
  const failure = mcpErrorFromUnknown(error);
  return formatMcpFailure(t, failure.errorCode, failure.message);
}

/** Shared by the page's own per-card status filter and `McpServerCard`'s `StatusBadge` so both
 *  translate the same `connectionStatus` axis the same way. "disconnected" and "no status data
 *  yet" intentionally collapse to the same "not tested" copy, matching this page's own
 *  pre-existing behavior. */
export function mcpConnectionStatusKey(connectionStatus: McpConnectionStatus | undefined): string {
  if (connectionStatus === "disabled") return "mcp.status.disabled";
  if (connectionStatus === "connected") return "mcp.status.connected";
  if (connectionStatus === "error") return "mcp.status.error";
  return "mcp.status.notTested";
}

export function mcpConnectionStatusTone(connectionStatus: McpConnectionStatus | undefined): StatusTone {
  if (connectionStatus === "connected") return "success";
  if (connectionStatus === "error") return "danger";
  return "neutral";
}
