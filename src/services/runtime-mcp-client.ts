import type { McpService } from "./mcp-service";
import { tauriMcpClient } from "./tauri-mcp-client";
import { webMcpClient } from "./web-mcp-client";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function createMcpService(): McpService {
  if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
    return tauriMcpClient;
  }

  return webMcpClient;
}

export const mcpService = createMcpService();
