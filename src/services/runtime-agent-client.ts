import type { AgentService } from "./agent-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";

export function createAgentService(): AgentService {
  return createRuntimeAdapter({
    tauri: tauriAgentClient,
    webMock: webAgentClient,
  });
}

export const agentService = createAgentService();
