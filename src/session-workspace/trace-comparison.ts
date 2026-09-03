import type {
  ExecutionSpanKind,
  ExecutionSpanSummary,
  ExecutionStatus,
  ExecutionTimeline,
} from "../types/execution-observability";

/**
 * Two runs, compared by what was counted rather than by what was said.
 *
 * Never a content diff. Comparing messages or attribute values would put two runs' redacted text
 * side by side and invite a reader to read the difference — which is the one operation this whole
 * change has been avoiding, because the material either run carries is exactly what redaction
 * exists for. Counts, statuses and classifications compare fine and give up nothing.
 *
 * The harder half is knowing when a comparison is not available. Two runs are only comparable on a
 * dimension when both were observed well enough to have a number for it, and the failure mode is
 * specific: a run whose spans are half opaque reports fewer tool calls than it made, and set next
 * to a fully observed run it looks like it did less work. So every count carries whether it is a
 * floor, and a difference between two floors is not a difference.
 */

/** A number from each run, and whether the pair means anything. */
export interface ComparedCount {
  left: number;
  right: number;
  /**
   * False when either side is a lower bound rather than a measurement.
   *
   * A reader shown "12 vs 8" concludes the second run did less. If the second run's spans were
   * half opaque, what they should conclude is that it was watched less closely.
   */
  comparable: boolean;
}

export interface ComparedDuration {
  left: number | null;
  right: number | null;
  /** False while either run is unfinished: a run still going has no duration to compare. */
  comparable: boolean;
}

export interface ComparedStatus {
  left: ExecutionStatus;
  right: ExecutionStatus;
  same: boolean;
}

/** How much of a run was observed natively, as counts per fidelity. */
export interface FidelityMix {
  native: number;
  proxied: number;
  inferred: number;
  opaque: number;
}

export interface RunComparison {
  status: ComparedStatus;
  duration: ComparedDuration;
  spans: ComparedCount;
  failures: ComparedCount;
  /** Spans that touched a file, by kind and by link. Never which files. */
  changes: ComparedCount;
  toolCounts: { kind: ExecutionSpanKind; left: number; right: number; comparable: boolean }[];
  usageQuality: { left: FidelityMix; right: FidelityMix };
  /**
   * Whether either run has an observation gap.
   *
   * Surfaced on its own because it qualifies every count below it at once, and a reader who
   * misses it will read the whole comparison as a set of measurements.
   */
  observationDiffers: boolean;
}

/** File relationships a link can name. Counted, never followed. */
const FILE_RELATIONSHIPS = new Set(["file", "file-change"]);

export function compareRuns(left: ExecutionTimeline, right: ExecutionTimeline): RunComparison {
  const leftMix = fidelityMix(left.spans);
  const rightMix = fidelityMix(right.spans);
  // A run with any span the runtime could not see inside, or any span whose end was never
  // observed, produces counts that are floors rather than totals.
  const leftBounded = isLowerBound(left.spans);
  const rightBounded = isLowerBound(right.spans);
  const comparable = !leftBounded && !rightBounded;

  const kinds = [...new Set([...left.spans, ...right.spans].map((span) => span.kind))]
    .filter((kind) => kind !== "unknown")
    .sort();

  return {
    status: {
      left: left.run.status,
      right: right.run.status,
      same: left.run.status === right.run.status,
    },
    duration: {
      left: left.run.durationMs ?? null,
      right: right.run.durationMs ?? null,
      // A run still going has no duration. Comparing elapsed-so-far against a finished run's total
      // would say the running one is faster, which is not a fact about either.
      comparable: left.run.durationMs !== null && left.run.durationMs !== undefined
        && right.run.durationMs !== null && right.run.durationMs !== undefined,
    },
    spans: { left: left.spans.length, right: right.spans.length, comparable },
    failures: {
      left: left.spans.filter(isFailure).length,
      right: right.spans.filter(isFailure).length,
      comparable,
    },
    changes: {
      left: changeCount(left.spans),
      right: changeCount(right.spans),
      comparable,
    },
    toolCounts: kinds.map((kind) => ({
      kind,
      left: left.spans.filter((span) => span.kind === kind).length,
      right: right.spans.filter((span) => span.kind === kind).length,
      comparable,
    })),
    usageQuality: { left: leftMix, right: rightMix },
    observationDiffers: leftBounded !== rightBounded,
  };
}

function isFailure(span: ExecutionSpanSummary): boolean {
  return span.status === "failed" || span.status === "cancelled" || span.status === "incomplete";
}

/**
 * Whether this run's counts are floors.
 *
 * An opaque span is work the runtime could not see inside, so anything it did is uncounted. An
 * incomplete one is work whose end was never observed, so anything after that point is uncounted
 * too. Either makes every count in the run a lower bound.
 */
function isLowerBound(spans: readonly ExecutionSpanSummary[]): boolean {
  return spans.some((span) => span.fidelity === "opaque" || span.status === "incomplete");
}

/**
 * How many spans touched a file.
 *
 * The count and nothing else. Which files a run touched is a path, and two runs' paths side by
 * side is a content comparison wearing a different name.
 */
function changeCount(spans: readonly ExecutionSpanSummary[]): number {
  return spans.filter(
    (span) =>
      span.kind === "file" ||
      span.links.some((link) => FILE_RELATIONSHIPS.has(link.relationship)),
  ).length;
}

function fidelityMix(spans: readonly ExecutionSpanSummary[]): FidelityMix {
  const mix: FidelityMix = { native: 0, proxied: 0, inferred: 0, opaque: 0 };
  for (const span of spans) mix[span.fidelity] += 1;
  return mix;
}

/**
 * The delta, or nothing when the pair does not support one.
 *
 * Returning `null` rather than a number is what stops a view from rendering "+4" next to two
 * figures that were never comparable in the first place.
 */
export function comparedDelta(count: ComparedCount | ComparedDuration): number | null {
  if (!count.comparable) return null;
  if (count.left === null || count.right === null) return null;
  return count.right - count.left;
}
