import { mockWorkspaceSnapshot } from "./mock-workspace-data";
import type { WorkspaceService } from "./workspace-service";

export const webWorkspaceClient: WorkspaceService = {
  async getWorkspaceSnapshot() {
    return mockWorkspaceSnapshot;
  },
};
