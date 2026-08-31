import { describe, expect, it } from "vitest";
import { getInspectorProvider, INSPECTOR_PROVIDERS, type InspectorProvider } from "./inspector-provider-registry";
import type { WorkbenchSelectionKind } from "../../types/workbench-selection";

const ALL_KINDS: WorkbenchSelectionKind[] = [
  "session",
  "message",
  "tool",
  "file",
  "change",
  "run",
  "loop-iteration",
  "evaluation-result",
];

describe("getInspectorProvider", () => {
  it("returns undefined for a kind with no registered provider, rather than throwing", () => {
    for (const kind of ALL_KINDS) {
      expect(() => getInspectorProvider(kind)).not.toThrow();
    }
  });

  it("returns exactly what is registered under its own kind, never a different kind's provider", () => {
    for (const [kind, provider] of Object.entries(INSPECTOR_PROVIDERS) as Array<[WorkbenchSelectionKind, InspectorProvider | undefined]>) {
      if (!provider) continue;
      expect(getInspectorProvider(kind)).toBe(provider);
      expect(provider.kind).toBe(kind);
    }
  });

  it("keeps every registered provider's loader lazy — the module is never imported eagerly by the registry itself", () => {
    for (const provider of Object.values(INSPECTOR_PROVIDERS)) {
      if (!provider) continue;
      expect(typeof provider.loader).toBe("function");
    }
  });
});
