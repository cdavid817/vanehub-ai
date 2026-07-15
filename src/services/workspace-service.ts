import type { WorkspaceSnapshot } from "../types/workspace";

export interface WorkspaceService {
  getWorkspaceSnapshot(): Promise<WorkspaceSnapshot>;
}
