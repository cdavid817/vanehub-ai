import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriWorkspaceClient } from "./tauri-workspace-client";
import { webWorkspaceClient } from "./web-workspace-client";
import type { WorkspaceService } from "./workspace-service";

export function createWorkspaceService(): WorkspaceService {
  return createRuntimeAdapter({
    tauri: tauriWorkspaceClient,
    webMock: webWorkspaceClient,
  });
}

export const workspaceService = createWorkspaceService();
