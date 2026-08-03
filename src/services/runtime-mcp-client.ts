import type { McpService } from "./mcp-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriMcpClient } from "./tauri-mcp-client";
import { webMcpClient } from "./web-mcp-client";
import { withMcpServiceValidation } from "./mcp-validation";

export function createMcpService(): McpService {
  return withMcpServiceValidation(
    createRuntimeAdapter({
      tauri: tauriMcpClient,
      webMock: webMcpClient,
    }),
  );
}

export const mcpService = createMcpService();
