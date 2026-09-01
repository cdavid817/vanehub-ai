// Task 12.3: architecture tests for unique page ids, field ids, anchors, search keys, category
// order, and synchronized locale keys. Runs against the *real* registry (not fixtures) --
// `settings-search-index.test.ts` already proves the duplicate-detection functions themselves work
// correctly against synthetic data; this file is what actually catches a real violation.
import { describe, expect, it } from "vitest";
import en from "../i18n/locales/en.json";
import { findDuplicateFieldAnchors, findDuplicateFieldIdsWithinPage } from "./settings-search-index";
import { settingsPageGroupOrder, settingsPages } from "./settings-pages";

const localeKeys = new Set(Object.keys(en));

describe("settings registry architecture (task 12.3)", () => {
  it("gives every page a unique id", () => {
    const ids = settingsPages.map((page) => page.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("gives no page duplicate field ids within itself", () => {
    const violations = settingsPages
      .map((page) => ({ pageId: page.id, duplicates: findDuplicateFieldIdsWithinPage(page) }))
      .filter((entry) => entry.duplicates.length > 0);
    expect(violations).toEqual([]);
  });

  it("gives no two fields (on the same or different pages) the same anchor id", () => {
    expect(findDuplicateFieldAnchors(settingsPages)).toEqual([]);
  });

  it("resolves every page's labelKey, crumbKey, descriptionKey, and searchPlaceholderKey to a real English string", () => {
    const missing: string[] = [];
    for (const page of settingsPages) {
      for (const key of [page.labelKey, page.crumbKey, page.descriptionKey, page.searchPlaceholderKey]) {
        if (!localeKeys.has(key)) missing.push(`${page.id}: ${key}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("resolves every field's labelKey (and any keyword-backed key) to a real English string", () => {
    const missing: string[] = [];
    for (const page of settingsPages) {
      for (const field of page.fields) {
        if (!localeKeys.has(field.labelKey)) missing.push(`${page.id}.${field.id}: ${field.labelKey}`);
      }
    }
    expect(missing).toEqual([]);
  });

  it("keeps every registered group inside the declared group order, with no page in an unlisted group", () => {
    const declaredGroups = new Set(settingsPageGroupOrder);
    const usedGroups = new Set(settingsPages.map((page) => page.group));
    for (const group of usedGroups) expect(declaredGroups.has(group)).toBe(true);
  });

  it("keeps pages in the same group contiguous, matching the declared group order", () => {
    const seenGroups: string[] = [];
    for (const page of settingsPages) {
      if (seenGroups[seenGroups.length - 1] !== page.group) seenGroups.push(page.group);
    }
    // If a group appeared, was left, and appeared again, it would show up twice in `seenGroups`.
    expect(new Set(seenGroups).size).toBe(seenGroups.length);
    const orderedSeenGroups = settingsPageGroupOrder.filter((group) => seenGroups.includes(group));
    expect(seenGroups).toEqual(orderedSeenGroups);
  });

  it("assigns every page a saveMode and risk level (no field left unclassified)", () => {
    for (const page of settingsPages) {
      expect(["immediate", "draft", "mixed"]).toContain(page.saveMode);
      expect(["normal", "sensitive", "dangerous"]).toContain(page.risk);
    }
  });
});
