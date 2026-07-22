import type { SshConnectionService } from "./ssh-connection-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriSshConnectionClient } from "./tauri-ssh-connection-client";
import { webSshConnectionClient } from "./web-ssh-connection-client";

export function createSshConnectionService(): SshConnectionService {
  return createRuntimeAdapter({
    tauri: tauriSshConnectionClient,
    webMock: webSshConnectionClient,
  });
}

export const sshConnectionService = createSshConnectionService();
