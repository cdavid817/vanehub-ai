import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import type { PromptHook } from "../types/prompt-hook";
import {
  expandedCategoriesFor,
  flattenPromptHookGroups,
  groupPromptHooks,
} from "../settings/pages/prompt-hooks/prompt-hook-view-model";
import {
  promptHookVirtualizationThreshold,
  shouldVirtualizePromptHooks,
} from "./virtual-list";

describe("Prompt Hook virtualization", () => {
  it("switches to windowing only above 500 items", () => {
    expect(promptHookVirtualizationThreshold).toBe(500);
    expect(shouldVirtualizePromptHooks(500)).toBe(false);
    expect(shouldVirtualizePromptHooks(501)).toBe(true);
  });

  it("flattens category headings and 501 ordered hooks without losing stable keys", () => {
    const hooks = Array.from({ length: 501 }, (_, index) => promptHook(index));
    const groups = groupPromptHooks(hooks);
    const rows = flattenPromptHookGroups(groups, expandedCategoriesFor(groups));

    expect(rows).toHaveLength(503);
    expect(rows.filter((row) => row.kind === "category")).toHaveLength(2);
    expect(rows.filter((row) => row.kind === "hook").map((row) => row.hook.id)).toEqual(
      hooks.filter((hook) => hook.category === "bootstrap").map((hook) => hook.id)
        .concat(hooks.filter((hook) => hook.category === "dynamic").map((hook) => hook.id)),
    );
    expect(new Set(rows.map((row) => row.key)).size).toBe(rows.length);
  });

  it("keeps operations, bounded overscan, and collection metadata on the shared card", () => {
    const source = readFileSync(
      new URL("../settings/pages/prompt-hooks/prompt-hook-card-list.tsx", import.meta.url),
      "utf8",
    );

    expect(source).toContain("overscan={4}");
    expect(source).toContain('testId="prompt-hook-virtual-list"');
    expect(source).toContain("aria-posinset");
    expect(source).toContain("aria-setsize");
    expect(source).toContain("onToggleEnabled");
    expect(source).toContain("onPreview");
    expect(source).toContain("onOpen");
    expect(source).toContain("onDelete");
  });
});

function promptHook(index: number): PromptHook {
  return {
    id: `hook-${index.toString().padStart(3, "0")}`,
    name: `Hook ${index}`,
    description: "Virtualized Hook",
    category: index % 2 === 0 ? "bootstrap" : "dynamic",
    stage: "per-turn",
    order: index,
    version: 1,
    source: "user",
    enabled: true,
    disableable: true,
    cliBindings: ["codex-cli"],
    governance: { safetyTier: "editable", transparencyTier: "opt-in-view", governanceTier: "human-gated" },
    createdAt: "2026-08-21T00:00:00.000Z",
    updatedAt: "2026-08-21T00:00:00.000Z",
  };
}
