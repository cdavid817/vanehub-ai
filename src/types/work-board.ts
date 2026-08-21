export const workItemStages = ["inbox", "planned", "in_progress", "review", "done"] as const;
export type WorkItemStage = (typeof workItemStages)[number];

export const workItemPriorities = ["none", "low", "medium", "high", "urgent"] as const;
export type WorkItemPriority = (typeof workItemPriorities)[number];

export const workItemSourceKinds = ["session", "scheduled_task"] as const;
export type WorkItemSourceKind = (typeof workItemSourceKinds)[number];
export type WorkItemLinkRelation = "primary" | "execution" | "automation" | "supporting";

export interface WorkItemSourceLink {
  sourceKind: WorkItemSourceKind;
  sourceId: string;
  relation: WorkItemLinkRelation;
  title: string;
  status: string;
  available: boolean;
  projectPath: string | null;
  updatedAt: string | null;
}

export interface WorkItem {
  id: string;
  title: string;
  description: string;
  stage: WorkItemStage;
  priority: WorkItemPriority;
  rank: number;
  projectPath: string | null;
  dueAt: string | null;
  archived: boolean;
  createdAt: string;
  updatedAt: string;
  sources: WorkItemSourceLink[];
}

export interface WorkItemFilters {
  archived?: boolean;
  query?: string;
  sourceKinds?: WorkItemSourceKind[];
  stages?: WorkItemStage[];
  priorities?: WorkItemPriority[];
  projectPaths?: string[];
}

export interface CreateWorkItemInput {
  title: string;
  description?: string;
  stage?: WorkItemStage;
  priority?: WorkItemPriority;
  projectPath?: string | null;
  dueAt?: string | null;
}

export interface UpdateWorkItemInput {
  workItemId: string;
  title?: string;
  description?: string;
  priority?: WorkItemPriority;
  projectPath?: string | null;
  dueAt?: string | null;
}

export interface MoveWorkItemInput {
  workItemId: string;
  stage: WorkItemStage;
  beforeWorkItemId?: string | null;
}

export interface LinkWorkItemSourceInput {
  workItemId: string;
  sourceKind: WorkItemSourceKind;
  sourceId: string;
  relation: WorkItemLinkRelation;
}
