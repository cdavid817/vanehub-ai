import type { SettingsPageDefinition, SettingsPageId } from "./settings-page-types";

/**
 * One flat, searchable entry per page and per registered field (task 12.1/12.4). Built once from
 * static registry metadata -- design.md Decision 17: "不通过挂载所有页面抓 DOM" (never by mounting
 * every page to scrape its DOM). i18n keys stay unresolved here; resolving them against the
 * *current* locale is `searchSettingsIndex`'s job, done at query time, since the index itself must
 * stay locale-independent (the user can switch language without the index going stale).
 */
export interface SettingsSearchEntry {
  pageId: SettingsPageId;
  kind: "page" | "field";
  labelKey: string;
  descriptionKey?: string;
  keywords: string[];
  /** Present only for `kind: "field"` -- what `/settings/:page#anchorId` scrolls/focuses to. */
  anchorId?: string;
}

export function buildSettingsSearchIndex(pages: SettingsPageDefinition[]): SettingsSearchEntry[] {
  const entries: SettingsSearchEntry[] = [];
  for (const page of pages) {
    entries.push({
      descriptionKey: page.descriptionKey,
      keywords: page.keywords,
      kind: "page",
      labelKey: page.labelKey,
      pageId: page.id,
    });
    for (const field of page.fields) {
      entries.push({
        anchorId: field.anchorId,
        keywords: field.keywords ?? [],
        kind: "field",
        labelKey: field.labelKey,
        pageId: page.id,
      });
    }
  }
  return entries;
}

export interface SettingsSearchResult {
  entry: SettingsSearchEntry;
  page: SettingsPageDefinition;
}

/**
 * `translate` is injected rather than imported (`useTranslation`'s `t`) so this stays a pure
 * function testable without React or an i18n provider -- the same reasoning as
 * `create-session-draft-model.ts`'s own reducer split (task 11.1).
 */
export function searchSettingsIndex(
  index: SettingsSearchEntry[],
  pages: SettingsPageDefinition[],
  query: string,
  translate: (key: string) => string,
): SettingsSearchResult[] {
  const normalized = query.trim().toLowerCase();
  if (!normalized) return [];
  const pagesById = new Map(pages.map((page) => [page.id, page]));
  const results: SettingsSearchResult[] = [];
  for (const entry of index) {
    if (!entryMatches(entry, normalized, translate)) continue;
    const page = pagesById.get(entry.pageId);
    if (page) results.push({ entry, page });
  }
  return results;
}

function entryMatches(entry: SettingsSearchEntry, normalizedQuery: string, translate: (key: string) => string): boolean {
  if (translate(entry.labelKey).toLowerCase().includes(normalizedQuery)) return true;
  if (entry.descriptionKey && translate(entry.descriptionKey).toLowerCase().includes(normalizedQuery)) return true;
  return entry.keywords.some((keyword) => keyword.toLowerCase().includes(normalizedQuery));
}

/**
 * Task 12.1/12.3's "Detect duplicate metadata" scenario: two fields (possibly on different pages)
 * sharing the same `anchorId` would make `/settings/:page#anchorId` navigation ambiguous. Returns
 * every anchor id used more than once, empty when the registry is clean.
 */
export function findDuplicateFieldAnchors(pages: SettingsPageDefinition[]): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const page of pages) {
    for (const field of page.fields) {
      if (seen.has(field.anchorId)) duplicates.add(field.anchorId);
      seen.add(field.anchorId);
    }
  }
  return [...duplicates];
}

/** Same shape of check for field *ids* (distinct from `anchorId` -- an id is the field's own
 *  stable identity within its page, an anchor is where it scrolls to; nothing requires them equal). */
export function findDuplicateFieldIdsWithinPage(page: SettingsPageDefinition): string[] {
  const seen = new Set<string>();
  const duplicates = new Set<string>();
  for (const field of page.fields) {
    if (seen.has(field.id)) duplicates.add(field.id);
    seen.add(field.id);
  }
  return [...duplicates];
}
