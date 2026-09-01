// Deliberately no React/testing-library import, and deliberately synthetic fixture pages rather
// than the real registry -- this is a pure-function test of the index/search mechanism itself,
// independent of exactly which 20 pages are currently registered or how they are classified.
import { describe, expect, it } from "vitest";
import {
  buildSettingsSearchIndex,
  findDuplicateFieldAnchors,
  findDuplicateFieldIdsWithinPage,
  searchSettingsIndex,
} from "./settings-search-index";
import type { SettingsPageDefinition } from "./settings-page-types";

const labels: Record<string, string> = {
  "page.alpha": "Alpha Settings",
  "page.alpha.description": "Configures the alpha subsystem.",
  "page.bravo": "Bravo Settings",
  "page.bravo.description": "Configures the bravo subsystem.",
  "field.alpha.timeout": "Request timeout",
  "field.bravo.host": "Remote host",
};

function translate(key: string): string {
  return labels[key] ?? key;
}

function page(overrides: Partial<SettingsPageDefinition>): SettingsPageDefinition {
  return {
    id: "basic",
    labelKey: "page.alpha",
    crumbKey: "page.alpha",
    group: "general",
    icon: (() => null) as unknown as SettingsPageDefinition["icon"],
    searchPlaceholderKey: "search.alpha",
    descriptionKey: "page.alpha.description",
    keywords: [],
    fields: [],
    saveMode: "immediate",
    risk: "normal",
    loader: () => Promise.resolve({ default: () => null }),
    ...overrides,
  };
}

const alpha = page({
  id: "basic",
  labelKey: "page.alpha",
  descriptionKey: "page.alpha.description",
  keywords: ["speed"],
  fields: [{ id: "timeout", labelKey: "field.alpha.timeout", anchorId: "alpha-timeout", keywords: ["latency"] }],
});
const bravo = page({
  id: "providers",
  labelKey: "page.bravo",
  descriptionKey: "page.bravo.description",
  keywords: [],
  fields: [{ id: "host", labelKey: "field.bravo.host", anchorId: "bravo-host" }],
});

describe("buildSettingsSearchIndex", () => {
  it("emits one page entry and one entry per registered field", () => {
    const index = buildSettingsSearchIndex([alpha, bravo]);
    expect(index).toHaveLength(4);
    expect(index.filter((entry) => entry.kind === "page")).toHaveLength(2);
    expect(index.filter((entry) => entry.kind === "field")).toHaveLength(2);
  });

  it("carries the anchor id through for field entries only", () => {
    const index = buildSettingsSearchIndex([alpha]);
    const fieldEntry = index.find((entry) => entry.kind === "field");
    expect(fieldEntry?.anchorId).toBe("alpha-timeout");
    const pageEntry = index.find((entry) => entry.kind === "page");
    expect(pageEntry?.anchorId).toBeUndefined();
  });
});

describe("searchSettingsIndex", () => {
  const index = buildSettingsSearchIndex([alpha, bravo]);

  it("matches a page by its localized label", () => {
    const results = searchSettingsIndex(index, [alpha, bravo], "Bravo", translate);
    expect(results).toHaveLength(1);
    expect(results[0].page.id).toBe("providers");
  });

  it("matches a field on another page and resolves its owning page", () => {
    const results = searchSettingsIndex(index, [alpha, bravo], "Remote host", translate);
    expect(results).toHaveLength(1);
    expect(results[0].entry.kind).toBe("field");
    expect(results[0].entry.anchorId).toBe("bravo-host");
    expect(results[0].page.id).toBe("providers");
  });

  it("matches a synonym keyword that does not appear in the visible label", () => {
    const results = searchSettingsIndex(index, [alpha, bravo], "latency", translate);
    expect(results).toHaveLength(1);
    expect(results[0].entry.anchorId).toBe("alpha-timeout");
  });

  it("matches a page-level keyword", () => {
    const results = searchSettingsIndex(index, [alpha, bravo], "speed", translate);
    expect(results.some((result) => result.entry.kind === "page" && result.page.id === "basic")).toBe(true);
  });

  it("returns no results for an empty or unmatched query", () => {
    expect(searchSettingsIndex(index, [alpha, bravo], "", translate)).toHaveLength(0);
    expect(searchSettingsIndex(index, [alpha, bravo], "   ", translate)).toHaveLength(0);
    expect(searchSettingsIndex(index, [alpha, bravo], "nonexistent-term-xyz", translate)).toHaveLength(0);
  });

  it("matches case-insensitively", () => {
    expect(searchSettingsIndex(index, [alpha, bravo], "ALPHA SETTINGS", translate)).toHaveLength(1);
  });
});

describe("findDuplicateFieldAnchors", () => {
  it("returns nothing when every anchor id is unique", () => {
    expect(findDuplicateFieldAnchors([alpha, bravo])).toEqual([]);
  });

  it("reports an anchor id reused across two different pages", () => {
    const collidingBravo = page({
      ...bravo,
      fields: [{ id: "host", labelKey: "field.bravo.host", anchorId: "alpha-timeout" }],
    });
    expect(findDuplicateFieldAnchors([alpha, collidingBravo])).toEqual(["alpha-timeout"]);
  });
});

describe("findDuplicateFieldIdsWithinPage", () => {
  it("returns nothing when field ids are unique within a page", () => {
    expect(findDuplicateFieldIdsWithinPage(alpha)).toEqual([]);
  });

  it("reports a field id reused within the same page", () => {
    const collidingAlpha = page({
      ...alpha,
      fields: [
        { id: "timeout", labelKey: "field.alpha.timeout", anchorId: "a" },
        { id: "timeout", labelKey: "field.alpha.timeout", anchorId: "b" },
      ],
    });
    expect(findDuplicateFieldIdsWithinPage(collidingAlpha)).toEqual(["timeout"]);
  });
});
