import { mockWorkspaceSnapshot } from "./mock-workspace-data";
import type { WorkspaceService } from "./workspace-service";

export const tauriWorkspaceClient: WorkspaceService = {
  async getWorkspaceSnapshot() {
    return mockWorkspaceSnapshot;
  },
};
