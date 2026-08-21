import type { ManagedCliAgentId } from "../../../types/agent";
import type {
  PromptHook,
  PromptHookCategory,
  PromptHookSource,
  PromptHookStage,
} from "../../../types/prompt-hook";

export type PromptHookEnabledFilter = "__all__" | "enabled" | "disabled";
export type PromptHookFilterValue<T extends string> = "__all__" | T;

export interface PromptHookFilters {
  agent: PromptHookFilterValue<ManagedCliAgentId>;
  category: PromptHookFilterValue<PromptHookCategory>;
  enabled: PromptHookEnabledFilter;
  query: string;
  source: PromptHookFilterValue<PromptHookSource>;
  stage: PromptHookFilterValue<PromptHookStage>;
}

export interface PromptHookGroup {
  category: PromptHookCategory;
  hooks: PromptHook[];
}

export type PromptHookInventoryRow =
  | { key: string; kind: "category"; category: PromptHookCategory; count: number }
  | { key: string; kind: "hook"; hook: PromptHook; position: number; total: number };

export const promptHookCategoryOrder: PromptHookCategory[] = [
  "bootstrap",
  "callback",
  "dynamic",
  "law",
  "navigation",
  "routing",
  "static",
];

export const defaultPromptHookFilters: PromptHookFilters = {
  agent: "__all__",
  category: "__all__",
  enabled: "__all__",
  query: "",
  source: "__all__",
  stage: "__all__",
};

export function filterPromptHooks(
  hooks: readonly PromptHook[],
  filters: PromptHookFilters,
  settingsSearchTerm = "",
) {
  const needle = `${filters.query} ${settingsSearchTerm}`.trim().toLowerCase();
  return hooks.filter((hook) => {
    if (filters.category !== "__all__" && hook.category !== filters.category) return false;
    if (filters.source !== "__all__" && hook.source !== filters.source) return false;
    if (filters.stage !== "__all__" && hook.stage !== filters.stage) return false;
    if (filters.enabled === "enabled" && !hook.enabled) return false;
    if (filters.enabled === "disabled" && hook.enabled) return false;
    if (filters.agent !== "__all__" && !hook.cliBindings.includes(filters.agent)) return false;
    if (!needle) return true;
    return searchableHookText(hook).includes(needle);
  });
}

export function groupPromptHooks(hooks: readonly PromptHook[]): PromptHookGroup[] {
  return promptHookCategoryOrder.flatMap((category) => {
    const categoryHooks = [...hooks.filter((hook) => hook.category === category)]
      .sort((left, right) => left.order - right.order || left.name.localeCompare(right.name));
    return categoryHooks.length > 0 ? [{ category, hooks: categoryHooks }] : [];
  });
}

export function expandedCategoriesFor(groups: readonly PromptHookGroup[]) {
  return new Set(groups.map((group) => group.category));
}

export function flattenPromptHookGroups(
  groups: readonly PromptHookGroup[],
  expandedCategories: ReadonlySet<PromptHookCategory>,
): PromptHookInventoryRow[] {
  const total = groups.reduce((count, group) => count + group.hooks.length, 0);
  let position = 0;
  return groups.flatMap((group) => {
    const heading: PromptHookInventoryRow = {
      key: `category:${group.category}`,
      kind: "category",
      category: group.category,
      count: group.hooks.length,
    };
    if (!expandedCategories.has(group.category)) return [heading];
    return [
      heading,
      ...group.hooks.map((hook): PromptHookInventoryRow => {
        position += 1;
        return { key: `hook:${hook.id}`, kind: "hook", hook, position, total };
      }),
    ];
  });
}

export function activeAdditionalFilterCount(filters: PromptHookFilters) {
  return [filters.category, filters.source, filters.stage]
    .filter((value) => value !== "__all__").length;
}

function searchableHookText(hook: PromptHook) {
  return [hook.id, hook.name, hook.description, hook.category, hook.stage, hook.source]
    .join(" ")
    .toLowerCase();
}
