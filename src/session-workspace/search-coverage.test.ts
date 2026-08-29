import { describe, expect, it } from "vitest";
import {
  emptyResultKey,
  searchReasonKey,
  workspaceSearchReasonCodes,
} from "./search-coverage";

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

  it("falls back rather than showing a token this build cannot word", () => {
    // A native build newer than this frontend. `byte_budget_exhausted` in front of a user is worse
    // than the vaguer sentence it would have replaced.
    expect(searchReasonKey("some_future_reason")).toBeNull();
    expect(searchReasonKey(undefined)).toBeNull();
    expect(searchReasonKey("")).toBeNull();
  });
});
