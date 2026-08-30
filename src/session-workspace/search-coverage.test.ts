import { describe, expect, it } from "vitest";
import { runWebPathSearch } from "../services/web-workspace-path-search-mock";
import { runWebWorkspaceSearch } from "../services/web-workspace-search-mock";
import {
  configureWebWorkspaceSearch,
  resetWebWorkspaceSearch,
} from "../services/web-workspace-search-registry";
import {
  emptyResultKey,
  searchReasonKey,
  workspaceSearchReasonCodes,
} from "./search-coverage";

const twoFiles = { "a.ts": "const needle = 1;\n", "b.ts": "const needle = 2;\n" };

/**
 * The distinction the whole coverage contract exists for. A search that examined the workspace and
 * found nothing means the text is not there; a search that stopped early found nothing *so far*.
 * Saying "no matches" for the second tells the user a fact nobody established, and the way they act
 * on it is to conclude the string does not exist and move on.
 */
describe("empty result message", () => {
  it("only claims there are no matches when the whole workspace was searched", () => {
    expect(emptyResultKey({ state: "complete" })).toBe("sessionTabs.files.contentSearch.empty");
    // No coverage at all is the pre-search state, where the plain message is still the honest one.
    expect(emptyResultKey(undefined)).toBe("sessionTabs.files.contentSearch.empty");
    expect(emptyResultKey(null)).toBe("sessionTabs.files.contentSearch.empty");
  });

  it("does not claim certainty when the search stopped early", () => {
    expect(emptyResultKey({ state: "partial", reasonCode: "byte_budget_exhausted" })).toBe(
      "sessionTabs.files.contentSearch.emptyPartial",
    );
    // Including cancellation: the user stopping the search is the clearest case of all, and it was
    // previously reported to them as "no matches".
    expect(emptyResultKey({ state: "partial", reasonCode: "cancelled" })).toBe(
      "sessionTabs.files.contentSearch.emptyPartial",
    );
  });

  it("separates a workspace that could not be searched from one that was", () => {
    // "No matches in what was searched" would describe an empty set as a result; nothing was
    // examined at all here.
    expect(emptyResultKey({ state: "unavailable", reasonCode: "provider_unavailable" })).toBe(
      "sessionTabs.files.contentSearch.emptyUnavailable",
    );
  });
});

describe("coverage reason", () => {
  it("has a key for every code the native budget can stop on", () => {
    // The matching Rust list is `InspectionStopReason::code`. A code added on one side without the
    // other degrades to the state sentence rather than to a raw token, but the point of pinning
    // this is that the degradation is a deliberate act rather than an oversight.
    expect(workspaceSearchReasonCodes).toHaveLength(18);
    for (const code of workspaceSearchReasonCodes) {
      expect(searchReasonKey(code)).toBe(`sessionTabs.files.searchReason.${code}`);
    }
  });

  it("words every stop the Web adapter can produce, not only the native ones", async () => {
    // Parity from the other direction. The pinned list is written against the Rust enum, so a code
    // the browser build invents on its own would render as a raw token there and nowhere else —
    // which is the shape of bug that only reproduces on the adapter nobody is running.
    const observed = new Set<string>();

    configureWebWorkspaceSearch({ maxFiles: 1 });
    observed.add(
      (await runWebWorkspaceSearch({ query: "needle", searchId: "s" }, twoFiles)).coverage
        .reasonCode ?? "",
    );
    resetWebWorkspaceSearch();

    configureWebWorkspaceSearch({ maxBytes: 4 });
    observed.add(
      (await runWebWorkspaceSearch({ query: "needle", searchId: "s" }, twoFiles)).coverage
        .reasonCode ?? "",
    );
    resetWebWorkspaceSearch();

    configureWebWorkspaceSearch({ maxResults: 1 });
    observed.add(
      (await runWebWorkspaceSearch({ query: "needle", searchId: "s" }, twoFiles)).coverage
        .reasonCode ?? "",
    );
    resetWebWorkspaceSearch();

    const stale = runWebWorkspaceSearch({ query: "needle", searchId: "s" }, twoFiles);
    const fresh = runWebWorkspaceSearch({ query: "other", searchId: "s" }, twoFiles);
    observed.add((await stale).coverage.reasonCode ?? "");
    await fresh;
    resetWebWorkspaceSearch();

    observed.add(
      (await runWebPathSearch({ query: "a", searchId: "s", cursor: "not-a-cursor" }, [])).coverage
        .reasonCode ?? "",
    );
    resetWebWorkspaceSearch();

    observed.delete("");
    // Named rather than counted. A drive that stopped producing codes would still satisfy a "every
    // observed code has a key" loop, by observing none.
    expect([...observed].sort()).toEqual([
      "byte_budget_exhausted",
      "file_budget_exhausted",
      "invalid_cursor",
      "result_budget_exhausted",
      "superseded",
    ]);
    for (const code of observed) {
      expect(searchReasonKey(code)).toBe(`sessionTabs.files.searchReason.${code}`);
    }
  });

  it("falls back rather than showing a token this build cannot word", () => {
    // A native build newer than this frontend. `byte_budget_exhausted` in front of a user is worse
    // than the vaguer sentence it would have replaced.
    expect(searchReasonKey("some_future_reason")).toBeNull();
    expect(searchReasonKey(undefined)).toBeNull();
    expect(searchReasonKey("")).toBeNull();
  });
});
