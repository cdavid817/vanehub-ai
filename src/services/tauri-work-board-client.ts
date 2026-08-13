import { invoke } from "@tauri-apps/api/core";
import type { WorkItem } from "../types/work-board";
import type { WorkBoardService } from "./work-board-service";

export const tauriWorkBoardClient: WorkBoardService = {
  listWorkItems(filters) { return invoke<WorkItem[]>("list_work_items", { filters: filters ?? {} }); },
  createWorkItem(input) { return invoke<WorkItem>("create_work_item", { input }); },
  updateWorkItem(input) { return invoke<WorkItem>("update_work_item", { input }); },
  moveWorkItem(input) { return invoke<WorkItem>("move_work_item", { input }); },
  linkWorkItemSource(input) { return invoke<WorkItem>("link_work_item_source", { input }); },
  archiveWorkItem(workItemId) { return invoke<WorkItem>("archive_work_item", { workItemId }); },
  restoreWorkItem(workItemId) { return invoke<WorkItem>("restore_work_item", { workItemId }); },
  deleteWorkItem(workItemId) { return invoke<void>("delete_work_item", { workItemId }); },
};
