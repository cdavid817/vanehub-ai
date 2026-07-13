import type { AgentService } from "./agent-service";
import { tauriAgentClient } from "./tauri-agent-client";
import { webAgentClient } from "./web-agent-client";

declare global {
  interface Window {
    __TAURI_INTERNALS__?: unknown;
  }
}

export function createAgentService(): AgentService {
  if (typeof window !== "undefined" && window.__TAURI_INTERNALS__) {
    return tauriAgentClient;
  }

  return webAgentClient;
}

export const agentService = createAgentService();
