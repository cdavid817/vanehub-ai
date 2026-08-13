import type { WorkItem, WorkItemFilters } from "../types/work-board";

export function filterWorkItems(items: WorkItem[], filters: WorkItemFilters): WorkItem[] {
  const query = filters.query?.trim().toLocaleLowerCase();
  return items.filter((item) => {
    if (item.archived !== Boolean(filters.archived)) return false;
    if (query && !`${item.title} ${item.description} ${item.projectPath ?? ""}`.toLocaleLowerCase().includes(query)) return false;
    if (filters.sourceKinds?.length && !item.sources.some((source) => filters.sourceKinds?.includes(source.sourceKind))) return false;
    if (filters.stages?.length && !filters.stages.includes(item.stage)) return false;
    if (filters.priorities?.length && !filters.priorities.includes(item.priority)) return false;
    return !filters.projectPaths?.length || Boolean(item.projectPath && filters.projectPaths.includes(item.projectPath));
  });
}
