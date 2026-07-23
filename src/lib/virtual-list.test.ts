import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  chunkItems,
  promptHookVirtualizationThreshold,
  shouldVirtualizePromptHooks,
} from "./virtual-list";

describe("Prompt Hook virtualization", () => {
  it("switches to windowing only above 500 items", () => {
    expect(promptHookVirtualizationThreshold).toBe(500);
    expect(shouldVirtualizePromptHooks(500)).toBe(false);
    expect(shouldVirtualizePromptHooks(501)).toBe(true);
  });

  it("regroups responsive rows without losing or reordering hooks", () => {
    const hooks = Array.from({ length: 501 }, (_, index) => `hook-${index}`);
    const singleColumn = chunkItems(hooks, 1);
    const doubleColumn = chunkItems(hooks, 2);

    expect(singleColumn).toHaveLength(501);
    expect(doubleColumn).toHaveLength(251);
    expect(doubleColumn.at(-1)).toEqual(["hook-500"]);
    expect(doubleColumn.flat()).toEqual(hooks);
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
    expect(source).toContain("onToggleAgent");
    expect(source).toContain("onPreview");
    expect(source).toContain("onEdit");
    expect(source).toContain("onDelete");
  });
});
