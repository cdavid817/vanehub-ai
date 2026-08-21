import type {
  CreateWorkItemInput,
  LinkWorkItemSourceInput,
  MoveWorkItemInput,
  UpdateWorkItemInput,
  WorkItem,
  WorkItemFilters,
  WorkItemSourceKind,
  WorkItemSourceLink,
} from "../types/work-board";
import { workItemPriorities, workItemStages } from "../types/work-board";
import { webAgentClient } from "./web-agent-client";
import type { WorkBoardService } from "./work-board-service";

const items = new Map<string, WorkItem>();
let sequence = 0;
const clone = <T>(value: T): T => structuredClone(value);
const now = () => new Date().toISOString();
const nextId = () => `work-item-web-${++sequence}`;

function requireItem(id: string): WorkItem {
  const item = items.get(id);
  if (!item) throw new Error("Work item not found.");
  return item;
}

function validateTitle(title: string): string {
  const value = title.trim();
  if (!value || value.length > 200) throw new Error("Work item title must contain 1-200 characters.");
  return value;
}

function nextRank(stage: WorkItem["stage"]): number {
  return Math.max(0, ...[...items.values()].filter((item) => item.stage === stage).map((item) => item.rank)) + 1_000;
}

function matches(item: WorkItem, filters: WorkItemFilters): boolean {
  if (item.archived !== Boolean(filters.archived)) return false;
  const query = filters.query?.trim().toLocaleLowerCase();
  if (query && !`${item.title} ${item.description} ${item.projectPath ?? ""}`.toLocaleLowerCase().includes(query)) return false;
  if (filters.sourceKinds?.length && !item.sources.some((source) => filters.sourceKinds?.includes(source.sourceKind))) return false;
  if (filters.stages?.length && !filters.stages.includes(item.stage)) return false;
  if (filters.priorities?.length && !filters.priorities.includes(item.priority)) return false;
  return !filters.projectPaths?.length || Boolean(item.projectPath && filters.projectPaths.includes(item.projectPath));
}

function sourceOwned(kind: WorkItemSourceKind, id: string): boolean {
  return [...items.values()].some((item) => item.sources.some((source) => source.sourceKind === kind && source.sourceId === id));
}

function addImported(title: string, stage: WorkItem["stage"], projectPath: string | null, source: WorkItemSourceLink): void {
  if (sourceOwned(source.sourceKind, source.sourceId)) return;
  const timestamp = now();
  const item: WorkItem = { id: nextId(), title, description: "", stage, priority: "none", rank: nextRank(stage), projectPath, dueAt: null, archived: false, createdAt: timestamp, updatedAt: timestamp, sources: [source] };
  items.set(item.id, item);
}

async function reconcile(): Promise<void> {
  const [sessions, scheduledTasks] = await Promise.all([
    webAgentClient.listSessions(), webAgentClient.listScheduledTasks(),
  ]);
  sessions.filter((session) => !session.executionOrigin || session.executionOrigin.kind === "user").forEach((session) => addImported(session.title, "inbox", session.projectPath, { sourceKind: "session", sourceId: session.id, relation: "primary", title: session.title, status: session.lifecycleState, available: true, projectPath: session.projectPath, updatedAt: session.updatedAt }));
  scheduledTasks.forEach((task) => addImported(task.name, "planned", null, { sourceKind: "scheduled_task", sourceId: task.id, relation: "primary", title: task.name, status: task.enabled ? task.latestStatus : "disabled", available: true, projectPath: null, updatedAt: task.updatedAt }));
}

function mutate(id: string, update: (item: WorkItem) => void): WorkItem {
  const item = requireItem(id);
  update(item);
  item.updatedAt = now();
  return clone(item);
}

export const webWorkBoardClient: WorkBoardService = {
  async listWorkItems(filters = {}) {
    await reconcile();
    return [...items.values()].filter((item) => matches(item, filters)).sort((left, right) => left.stage.localeCompare(right.stage) || left.rank - right.rank).map(clone);
  },
  async createWorkItem(input: CreateWorkItemInput) {
    const stage = input.stage ?? "inbox";
    if (!workItemStages.includes(stage)) throw new Error("Unknown work item stage.");
    const priority = input.priority ?? "none";
    if (!workItemPriorities.includes(priority)) throw new Error("Unknown work item priority.");
    const timestamp = now();
    const item: WorkItem = { id: nextId(), title: validateTitle(input.title), description: input.description?.trim() ?? "", stage, priority, rank: nextRank(stage), projectPath: input.projectPath?.trim() || null, dueAt: input.dueAt ?? null, archived: false, createdAt: timestamp, updatedAt: timestamp, sources: [] };
    items.set(item.id, item);
    return clone(item);
  },
  async updateWorkItem(input: UpdateWorkItemInput) {
    return mutate(input.workItemId, (item) => {
      if (input.title !== undefined) item.title = validateTitle(input.title);
      if (input.description !== undefined) item.description = input.description.trim();
      if (input.priority !== undefined) item.priority = input.priority;
      if (input.projectPath !== undefined) item.projectPath = input.projectPath?.trim() || null;
      if (input.dueAt !== undefined) item.dueAt = input.dueAt;
    });
  },
  async moveWorkItem(input: MoveWorkItemInput) {
    return mutate(input.workItemId, (item) => {
      item.stage = input.stage;
      const before = input.beforeWorkItemId ? requireItem(input.beforeWorkItemId) : null;
      item.rank = before?.stage === input.stage ? before.rank - 1 : nextRank(input.stage);
    });
  },
  async linkWorkItemSource(input: LinkWorkItemSourceInput) {
    if (sourceOwned(input.sourceKind, input.sourceId)) throw new Error("Source is already linked to a work item.");
    return mutate(input.workItemId, (item) => item.sources.push({ sourceKind: input.sourceKind, sourceId: input.sourceId, relation: input.relation, title: input.sourceId, status: "linked", available: true, projectPath: null, updatedAt: null }));
  },
  async archiveWorkItem(workItemId) { return mutate(workItemId, (item) => { item.archived = true; }); },
  async restoreWorkItem(workItemId) { return mutate(workItemId, (item) => { item.archived = false; }); },
  async deleteWorkItem(workItemId) {
    const item = requireItem(workItemId);
    if (!item.archived) throw new Error("Archive a work item before permanently deleting it.");
    items.delete(workItemId);
  },
};

export function resetWebWorkBoardForTest(): void {
  items.clear();
  sequence = 0;
}
