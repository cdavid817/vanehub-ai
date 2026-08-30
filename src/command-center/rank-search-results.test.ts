import { describe, expect, it } from "vitest";
import { rankSearchResults } from "./rank-search-results";
import type { WorkbenchSearchResult } from "./command-center-types";

const NOW = Date.parse("2026-08-31T12:00:00.000Z");

function result(overrides: Partial<WorkbenchSearchResult> = {}): WorkbenchSearchResult {
  return {
    key: "k",
    kind: "session",
    title: "Untitled",
    route: { destination: "sessions", sessionId: "s", creatingSession: false },
    ...overrides,
  };
}

describe("rankSearchResults", () => {
  it("ranks an exact title match above a prefix match, above a plain substring match", () => {
    const exact = result({ key: "exact", title: "auth" });
    const prefix = result({ key: "prefix", title: "auth token fix" });
    const substring = result({ key: "substring", title: "fix null auth" });
    const ranked = rankSearchResults([substring, prefix, exact], "auth", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["exact", "prefix", "substring"]);
  });

  it("drops a result that does not even substring-match a non-empty query", () => {
    const exact = result({ key: "exact", title: "auth" });
    const unrelated = result({ key: "unrelated", title: "totally different" });
    const ranked = rankSearchResults([unrelated, exact], "auth", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["exact"]);
  });

  it("ranks a needs-attention result above an otherwise-identical neutral one", () => {
    const attention = result({ key: "attention", title: "fix null auth", status: "attention" });
    const neutral = result({ key: "neutral", title: "fix null auth", status: "neutral" });
    const ranked = rankSearchResults([neutral, attention], "auth", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["attention", "neutral"]);
  });

  it("ranks an error-status result above neutral too", () => {
    const error = result({ key: "error", title: "fix null auth", status: "error" });
    const neutral = result({ key: "neutral", title: "fix null auth", status: "neutral" });
    const ranked = rankSearchResults([neutral, error], "auth", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["error", "neutral"]);
  });

  it("ranks a current-project result above one from elsewhere", () => {
    const here = result({ key: "here", title: "fix bug", subtitle: "D:\\code\\vanehub" });
    const elsewhere = result({ key: "elsewhere", title: "fix bug", subtitle: "D:\\code\\other" });
    const ranked = rankSearchResults([elsewhere, here], "fix", { now: NOW, currentProjectPath: "D:\\code\\vanehub" });
    expect(ranked.map((entry) => entry.key)).toEqual(["here", "elsewhere"]);
  });

  it("does not apply the current-project boost when no current project is given", () => {
    const a = result({ key: "a", title: "fix bug", subtitle: "D:\\code\\vanehub" });
    const b = result({ key: "b", title: "fix bug", subtitle: "D:\\code\\other", updatedAt: "2026-08-31T11:59:00.000Z" });
    const ranked = rankSearchResults([a, b], "fix", { now: NOW });
    // No project signal in play, so the only remaining differentiator is recency — b is newer.
    expect(ranked.map((entry) => entry.key)).toEqual(["b", "a"]);
  });

  it("breaks ties by recency", () => {
    const older = result({ key: "older", title: "fix bug", updatedAt: "2026-08-01T00:00:00.000Z" });
    const newer = result({ key: "newer", title: "fix bug", updatedAt: "2026-08-30T00:00:00.000Z" });
    const ranked = rankSearchResults([older, newer], "fix", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["newer", "older"]);
  });

  it("preserves input order for a full tie (stable sort)", () => {
    const a = result({ key: "a", title: "same" });
    const b = result({ key: "b", title: "same" });
    const c = result({ key: "c", title: "same" });
    const ranked = rankSearchResults([a, b, c], "", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["a", "b", "c"]);
  });

  it("does not let recency alone outrank an exact title match", () => {
    const staleExact = result({ key: "stale-exact", title: "auth", updatedAt: "2020-01-01T00:00:00.000Z" });
    const freshSubstring = result({ key: "fresh-substring", title: "fix null auth", updatedAt: "2026-08-31T11:59:59.000Z" });
    const ranked = rankSearchResults([freshSubstring, staleExact], "auth", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["stale-exact", "fresh-substring"]);
  });

  it("treats an empty query as no title-match signal, ranking only on attention/project/recency", () => {
    const attention = result({ key: "attention", title: "anything", status: "attention", updatedAt: "2020-01-01T00:00:00.000Z" });
    const recent = result({ key: "recent", title: "anything else", updatedAt: "2026-08-31T11:59:59.000Z" });
    const ranked = rankSearchResults([recent, attention], "", { now: NOW });
    expect(ranked.map((entry) => entry.key)).toEqual(["attention", "recent"]);
  });
});
