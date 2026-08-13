import type {
  CreateWorkItemInput,
  LinkWorkItemSourceInput,
  MoveWorkItemInput,
  UpdateWorkItemInput,
  WorkItem,
  WorkItemFilters,
} from "../types/work-board";

export interface WorkBoardService {
  listWorkItems(filters?: WorkItemFilters): Promise<WorkItem[]>;
  createWorkItem(input: CreateWorkItemInput): Promise<WorkItem>;
  updateWorkItem(input: UpdateWorkItemInput): Promise<WorkItem>;
  moveWorkItem(input: MoveWorkItemInput): Promise<WorkItem>;
  linkWorkItemSource(input: LinkWorkItemSourceInput): Promise<WorkItem>;
  archiveWorkItem(workItemId: string): Promise<WorkItem>;
  restoreWorkItem(workItemId: string): Promise<WorkItem>;
  deleteWorkItem(workItemId: string): Promise<void>;
}
