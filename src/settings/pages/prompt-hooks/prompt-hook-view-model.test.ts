import { describe, expect, it } from "vitest";
import type { PromptHook } from "../../../types/prompt-hook";
import {
  activeAdditionalFilterCount,
  defaultPromptHookFilters,
  expandedCategoriesFor,
  filterPromptHooks,
  flattenPromptHookGroups,
  groupPromptHooks,
} from "./prompt-hook-view-model";

describe("Prompt Hook management view model", () => {
  it("combines primary and additional filters with settings search", () => {
    const visible = filterPromptHooks(hooks, {
      ...defaultPromptHookFilters,
      agent: "codex-cli",
      enabled: "enabled",
      source: "user",
      stage: "per-turn",
    }, "review");

    expect(visible.map((hook) => hook.id)).toEqual(["dynamic-review"]);
  });

  it("groups by stable category and execution order", () => {
    const groups = groupPromptHooks(hooks);

    expect(groups.map((group) => [group.category, group.hooks.length])).toEqual([
      ["bootstrap", 1],
      ["dynamic", 2],
    ]);
    expect(groups[1].hooks.map((hook) => hook.id)).toEqual(["dynamic-review", "dynamic-disabled"]);
  });

  it("resets expansion to every matching group and produces stable flattened keys", () => {
    const groups = groupPromptHooks(hooks);
    const expanded = expandedCategoriesFor(groups);
    const rows = flattenPromptHookGroups(groups, expanded);

    expect([...expanded]).toEqual(["bootstrap", "dynamic"]);
    expect(rows.map((row) => row.key)).toEqual([
      "category:bootstrap",
      "hook:bootstrap-context",
      "category:dynamic",
      "hook:dynamic-review",
      "hook:dynamic-disabled",
    ]);
    expect(rows.at(-1)).toMatchObject({ kind: "hook", position: 3, total: 3 });
  });

  it("counts only source, stage, and category as additional filters", () => {
    expect(activeAdditionalFilterCount({
      ...defaultPromptHookFilters,
      agent: "codex-cli",
      enabled: "enabled",
      category: "dynamic",
      stage: "per-turn",
    })).toBe(2);
  });
});

const hooks: PromptHook[] = [
  hook("dynamic-disabled", "dynamic", 20, { enabled: false }),
  hook("bootstrap-context", "bootstrap", 10, { source: "builtin" }),
  hook("dynamic-review", "dynamic", 10),
];

function hook(
  id: string,
  category: PromptHook["category"],
  order: number,
  overrides: Partial<PromptHook> = {},
): PromptHook {
  return {
    id,
    name: id.includes("review") ? "Review Focus" : id,
    description: "Prompt Hook fixture",
    category,
    stage: "per-turn",
    order,
    version: 1,
    source: "user",
    enabled: true,
    disableable: true,
    cliBindings: ["codex-cli"],
    governance: {
      safetyTier: "editable",
      transparencyTier: "opt-in-view",
      governanceTier: "human-gated",
    },
    createdAt: "2026-08-21T00:00:00.000Z",
    updatedAt: "2026-08-21T00:00:00.000Z",
    ...overrides,
  };
}
