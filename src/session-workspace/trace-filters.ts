import type {
  ExecutionFidelity,
  ExecutionSpanSummary,
} from "../types/execution-observability";

/**
 * What a reader can narrow a waterfall to, and what narrowing costs them.
 *
 * Every filter here answers a question somebody actually arrives with — what failed, what was
 * retried, what the run waited on, what we could not see. The cost is the same one every filter
 * has: a hidden span is indistinguishable from a span that never happened, and a waterfall showing
 * three rows out of two hundred looks exactly like a run that did almost nothing. So the filter
 * state travels with a count of what it excluded, and the view is expected to say so.
 */

/** Toggles that narrow to spans having a property. All off means no narrowing. */
export interface TraceFilters {
  criticalPath: boolean;
  retried: boolean;
  delegated: boolean;
  failed: boolean;
  /** Spans whose observation is known to be incomplete. */
  gap: boolean;
  /** Empty means every fidelity. A concrete set matches only those. */
  fidelities: ExecutionFidelity[];
}

export const NO_TRACE_FILTERS: TraceFilters = {
  criticalPath: false,
  retried: false,
  delegated: false,
  failed: false,
  gap: false,
  fidelities: [],
};

export type TraceFilterToggle = "criticalPath" | "retried" | "delegated" | "failed" | "gap";

export const TRACE_FILTER_TOGGLES: TraceFilterToggle[] = [
  "failed",
  "criticalPath",
  "retried",
  "delegated",
  "gap",
];

export const TRACE_FIDELITIES: ExecutionFidelity[] = [
  "native",
  "proxied",
  "inferred",
  "opaque",
];

/** Whether any filter is doing anything. */
export function hasActiveTraceFilters(filters: TraceFilters): boolean {
  return (
    filters.criticalPath ||
    filters.retried ||
    filters.delegated ||
    filters.failed ||
    filters.gap ||
    filters.fidelities.length > 0
  );
}

/**
 * A span is shown when it satisfies every active toggle.
 *
 * Conjunctive rather than disjunctive: "failed and on the critical path" is the question somebody
 * debugging a slow failure actually has, and a union would answer a different one — every failure
 * plus every critical-path span — while looking like it had answered theirs.
 */
export function matchesTraceFilters(
  span: ExecutionSpanSummary,
  filters: TraceFilters,
): boolean {
  if (filters.criticalPath && !span.criticalPath) return false;
  // A span nobody counted an attempt for is not a retry. Treating absent as 1 and then as "not
  // retried" happens to give the right answer here, but only by accident — so it is written as
  // the explicit test that it is.
  if (filters.retried && !(span.attempt !== undefined && span.attempt > 1)) return false;
  if (filters.delegated && !span.delegated) return false;
  if (filters.failed && !isFailure(span)) return false;
  if (filters.gap && !hasObservationGap(span)) return false;
  if (filters.fidelities.length > 0 && !filters.fidelities.includes(span.fidelity)) return false;
  return true;
}

/**
 * What counts as a failure for the purposes of looking for one.
 *
 * `incomplete` is included because a span whose end was never observed is a span that may have
 * failed — and somebody scanning for failures needs to see it rather than have it quietly sorted
 * into the successes.
 */
function isFailure(span: ExecutionSpanSummary): boolean {
  return span.status === "failed" || span.status === "cancelled" || span.status === "incomplete";
}

/**
 * Whether this span's observation is known to be incomplete.
 *
 * Two different shapes of the same problem: `opaque` means the runtime could not see inside the
 * work, and `incomplete` means it saw the start and never the end. Both leave a hole, and a reader
 * auditing what the traces can be trusted about wants them together.
 */
function hasObservationGap(span: ExecutionSpanSummary): boolean {
  return span.fidelity === "opaque" || span.status === "incomplete";
}

export interface FilteredSpans {
  spans: ExecutionSpanSummary[];
  /** How many the filters removed. Zero when nothing is filtering. */
  hiddenCount: number;
}

/**
 * Applies the filters and reports what they cost.
 *
 * The count is not decoration. A waterfall showing three rows looks the same whether the run had
 * three spans or two hundred, and the difference between "this run did almost nothing" and "I am
 * looking at almost none of it" is the entire reason to return it.
 */
export function filterTraceSpans(
  spans: readonly ExecutionSpanSummary[],
  filters: TraceFilters,
): FilteredSpans {
  if (!hasActiveTraceFilters(filters)) {
    return { spans: [...spans], hiddenCount: 0 };
  }
  const kept = spans.filter((span) => matchesTraceFilters(span, filters));
  return { spans: kept, hiddenCount: spans.length - kept.length };
}
