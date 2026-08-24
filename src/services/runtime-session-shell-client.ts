import { createRuntimeAdapter } from "./runtime-adapter";
import type { SessionShellService } from "./session-shell-service";
import { tauriSessionShellClient } from "./tauri-session-shell-client";
import { webSessionShellClient } from "./web-session-shell-client";

export function createSessionShellService(): SessionShellService {
  return createRuntimeAdapter({
    tauri: tauriSessionShellClient,
    webMock: webSessionShellClient,
  });
}

export const sessionShellService = createSessionShellService();
