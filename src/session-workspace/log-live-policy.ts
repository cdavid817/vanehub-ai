import type { SessionLogNotice } from "../types/session-log-notice";
import type {
  SessionLogCorrelationFilters,
  SessionLogLevel,
} from "../types/session-workspace";

/**
 * What a view may do with a live notice, given the filters it is currently showing.
 *
 * Three outcomes rather than two, and the third is the one that matters. A notice carries
 * identifiers and never the log line — deliberately, so the event channel does not carry the
 * corpus — which means some filters can be decided from it and some cannot. Guessing on the ones
 * that cannot is what produces the two failures this exists to prevent: a row appearing that the
 * active search excludes, or a row silently withheld that it admits. Both look like the filter is
 * broken, and neither is recoverable by looking harder at the list.
 */
export type LiveNoticeDecision = "insert" | "ignore" | "invalidate";

export interface LiveNoticeScope {
  levels: SessionLogLevel[];
  search: string;
  correlation: SessionLogCorrelationFilters;
  sessionId: string | null;
}

/** Which notice field answers each correlation filter. */
const CORRELATION_FIELDS = {
  seatId: "seatId",
  runId: "runId",
  traceId: "traceId",
  spanId: "spanId",
  operationId: "operationId",
  agentId: "agentId",
} as const;

function selected(value: string | null | undefined): string | null {
  return typeof value === "string" && value.trim().length > 0 ? value : null;
}

/**
 * Decides from the notice alone.
 *
 * `invalidate` is returned for a text search because the notice has no message, category or
 * context to match against — and a search is precisely the filter under which a reader is most
 * confident that what they see is everything that matched.
 *
 * A gap notice always invalidates. It says records were lost, which changes what the current page
 * is missing rather than adding to it; inserting anything for it would be inventing a row, and
 * ignoring it would leave the page claiming a completeness it no longer has.
 */
export function decideLiveNotice(
  notice: SessionLogNotice,
  scope: LiveNoticeScope,
): LiveNoticeDecision {
  if (notice.noticeKind === "gap") return "invalidate";

  // A notice for another session is not this list's business at all, and that is decidable without
  // any of the filters below.
  if (scope.sessionId && notice.sessionId && notice.sessionId !== scope.sessionId) return "ignore";
  if (scope.sessionId && !notice.sessionId) return "ignore";

  for (const [filterKey, noticeKey] of Object.entries(CORRELATION_FIELDS)) {
    const wanted = selected(scope.correlation[filterKey as keyof SessionLogCorrelationFilters]);
    if (!wanted) continue;
    // A record that carries no such correlation is out of scope rather than in it. Admitting it
    // would attribute work to a run that did not do it — the same rule the query itself follows.
    if (notice[noticeKey] !== wanted) return "ignore";
  }

  if (scope.levels.length > 0 && !scope.levels.includes(notice.level)) return "ignore";

  // Everything decidable said yes. The search is the one filter left, and it cannot be answered
  // from identifiers — so the page is invalidated rather than guessed at.
  if (scope.search.trim().length > 0) return "invalidate";

  return "insert";
}
