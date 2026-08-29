import type { WorkspaceSearchCoverage } from "../types/session-workspace-inspection";

/**
 * The reason codes the native inspection budget can stop on.
 *
 * Pinned here rather than accepted as an open string, because the whole point of a code is that the
 * frontend owns the wording — and a code with no wording renders as a raw token, which is worse
 * than the generic sentence it would have replaced. The matching Rust list is
 * `InspectionStopReason::code`.
 */
export const workspaceSearchReasonCodes = [
  "cancelled",
  "superseded",
  "owner_dropped",
  "inspection_busy",
  "directory_budget_exhausted",
  "entry_budget_exhausted",
  "file_budget_exhausted",
  "byte_budget_exhausted",
  "metadata_budget_exhausted",
  "candidate_budget_exhausted",
  "result_budget_exhausted",
  "depth_budget_exhausted",
  "deadline_exceeded",
  "unreadable_entries",
  "provider_unavailable",
  "provider_failed",
  "invalid_cursor",
  "stale_cursor",
] as const;

export type WorkspaceSearchReasonCode = (typeof workspaceSearchReasonCodes)[number];

/**
 * The i18n key for a coverage reason, or `null` when this build has no wording for it.
 *
 * A native build newer than this frontend can send a code that is not in the list. Returning `null`
 * makes the caller fall back to the state-level sentence, which is vaguer but still true — whereas
 * interpolating the raw token would put `byte_budget_exhausted` in front of a user.
 */
export function searchReasonKey(reasonCode: string | undefined): string | null {
  if (!reasonCode) return null;
  return (workspaceSearchReasonCodes as readonly string[]).includes(reasonCode)
    ? `sessionTabs.files.searchReason.${reasonCode}`
    : null;
}

/**
 * Which message an empty result should use.
 *
 * This is the distinction the whole coverage contract exists for. A search that examined the whole
 * workspace and found nothing means the text is not there. A search that stopped early — budget
 * exhausted, cancelled, deadline reached — found nothing *so far*, and saying "no matches" there
 * tells the user a fact nobody established. They are most likely to act on it by concluding the
 * string does not exist and moving on.
 *
 * `unavailable` is separated again because nothing was searched at all: "no matches in what was
 * searched" would describe an empty set as a result.
 */
export function emptyResultKey(coverage: WorkspaceSearchCoverage | null | undefined): string {
  if (!coverage || coverage.state === "complete") {
    return "sessionTabs.files.contentSearch.empty";
  }
  return coverage.state === "unavailable"
    ? "sessionTabs.files.contentSearch.emptyUnavailable"
    : "sessionTabs.files.contentSearch.emptyPartial";
}
