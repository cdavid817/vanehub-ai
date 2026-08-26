/**
 * What the Logs panel is handed, stated apart from the code that produces it.
 *
 * Split out because the two change for different reasons and at different rates: the contract moves
 * when the panel needs to know something new, the hook moves whenever a read does. Together they
 * had reached ten lines short of the file-size rule, and a split made under that pressure is one
 * made in a hurry.
 *
 * Every field's comment came with it. They explain distinctions a caller has to honour —
 * `initialError` blocks while `pageError` does not, `coverage` is undefined rather than complete
 * until a page answers — and a contract whose reasons stayed behind in the implementation is one
 * nobody can implement twice.
 */
import type {
  SessionLogCorrelationFilters,
  SessionLogCoverage,
  SessionLogEntry,
  SessionLogLevel,
} from "../types/session-workspace";
import type { SessionLogNotice } from "../types/session-log-notice";
import type { LiveNoticeDecision } from "./log-live-policy";
import type { WorkspaceErrorKey } from "./workspace-error";

export type SessionLogSeekStatus = "continue" | "invalid" | "not-found" | null;

export interface SessionLogsScope {
  sessionId: string | null;
  /**
   * Every correlation narrowing the read, as one value.
   *
   * One object rather than a parameter each, because the set grows: a new correlation added as its
   * own parameter is one the callers can forget to pass, and forgetting it widens the query
   * silently — the list gets bigger and nothing says why.
   */
  scope: SessionLogCorrelationFilters;
  levels: SessionLogLevel[];
  search: string;
  /** False while the panel stays mounted behind another tab. Defers reads, keeps rows. */
  isVisible?: boolean;
}

/**
 * Log page state, kept out of the view so a failure can be attributed to the request that failed.
 *
 * `initialError` blocks, because there is nothing to look at yet. `pageError` does not: a page
 * append or refresh that fails must leave the rows the user is already reading on screen, which
 * the previous single-error state could not express — one failed Load more replaced the whole
 * list with an error panel.
 */
export interface SessionLogsState {
  entries: SessionLogEntry[];
  /**
   * What the index was willing to claim about the rows below, as of the read that produced them.
   *
   * Kept beside the entries rather than fetched on its own, so a reader can never be shown rows
   * from one moment under a coverage claim from another. `undefined` until a page has answered,
   * which the view renders as `unavailable` — a coverage nobody reported is not a complete one.
   */
  coverage: SessionLogCoverage | undefined;
  hasMore: boolean;
  initialError: WorkspaceErrorKey | null;
  loading: boolean;
  pageError: WorkspaceErrorKey | null;
  pendingFocusId: string | null;
  seekStatus: SessionLogSeekStatus;
  seeking: boolean;
  stale: boolean;
  /**
   * Set when a live notice arrived that the current filters could not be judged against.
   *
   * Not an error, and not a stale marker: the rows on screen are correct, and something happened
   * that this view cannot place among them. Refreshing resolves it; guessing would not.
   */
  firstPageInvalidated: boolean;
  clearPendingFocus: () => void;
  clearSeekStatus: () => void;
  /**
   * Feeds one live notice through the insertion policy.
   *
   * Returns what was done, so a caller can count what it is withholding without re-deriving the
   * decision — two answers to "was this row added" is exactly the drift the shared policy exists to
   * prevent.
   */
  applyLiveNotice: (notice: SessionLogNotice) => Promise<LiveNoticeDecision>;
  loadMore: () => Promise<void>;
  locateTimestamp: (draft: string) => Promise<void>;
  refresh: () => Promise<void>;
  retryInitial: () => Promise<void>;
}
