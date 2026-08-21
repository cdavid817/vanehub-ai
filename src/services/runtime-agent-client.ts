import { createRuntimeAdapter } from "./runtime-adapter";
import type { RuntimeAgentService } from "./runtime-agent-service";
import { tauriAgentClient } from "./tauri-agent-client";
import { tauriLoopClient } from "./tauri-loop-client";
import { webAgentClient } from "./web-agent-client";

export const tauriRuntimeAgentClient: RuntimeAgentService = { ...tauriAgentClient, ...tauriLoopClient };
// The Web adapter already composes webLoopClient; retain its identity so runtime spies and subscriptions stay attached.
const webRuntimeAgentClient = webAgentClient as RuntimeAgentService;

export function createAgentService(): RuntimeAgentService {
  return createRuntimeAdapter({
    tauri: tauriRuntimeAgentClient,
    webMock: webRuntimeAgentClient,
  });
}

export const agentService = createAgentService();
