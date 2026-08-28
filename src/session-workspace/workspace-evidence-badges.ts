import type {
  EvidenceCoverageState,
  WorkspaceEvidenceSummary,
} from "../types/session-workspace-evidence";
import type { SessionTabId } from "./session-tab-bar";

export type WorkspaceBadgeTone = "neutral" | "danger";

/**
 * What a tab badge is allowed to say.
 *
 * The third case is the one that matters. A summary that cannot answer — the index is still
 * building, the retained window does not reach back far enough, the capability is not wired on
 * this runtime — must not render as `0`, because `0` is a claim: it says the session has no live
 * shells, no unviewed changes, no failed verifications. A reader cannot tell that claim apart from
 * "nobody has counted yet", and acting on it is exactly the mistake this console exists to
 * prevent.
 */
export type WorkspaceTabBadge =
  | { kind: "none" }
  | { kind: "count"; count: number; tone: WorkspaceBadgeTone; atLeast: boolean }
  | { kind: "unknown"; reason: EvidenceCoverageState | "loading" };

/** How far the single summary query has got. Anything but `ready` produces placeholders. */
export type WorkspaceSummaryState = "loading" | "ready" | "unavailable";

const NONE: WorkspaceTabBadge = { kind: "none" };

/**
 * One badge, decided from one count and the coverage that count was measured under.
 *
 * `partial` with a zero count is not a zero: partial coverage means the answer is a floor, and a
 * floor of zero carries no information at all. A positive count under partial coverage is honest
 * as "at least this many", which is what `atLeast` renders.
 */
function badgeFor(
  count: number,
  tone: WorkspaceBadgeTone,
  coverage: EvidenceCoverageState,
): WorkspaceTabBadge {
  if (coverage === "unavailable" || coverage === "indexing") {
    return { kind: "unknown", reason: coverage };
  }
  if (coverage === "partial") {
    return count > 0
      ? { kind: "count", count, tone, atLeast: true }
      : { kind: "unknown", reason: "partial" };
  }
  return count > 0 ? { kind: "count", count, tone, atLeast: false } : NONE;
}

/**
 * A failure count outranks a running count, and the tone says which one is on screen.
 *
 * Showing `running` alone would hide a tab whose work has already failed; showing both would need
 * two numbers in a badge that has room for one.
 */
function attentionBadge(
  running: number,
  failed: number,
  coverage: EvidenceCoverageState,
): WorkspaceTabBadge {
  return failed > 0
    ? badgeFor(failed, "danger", coverage)
    : badgeFor(running, "neutral", coverage);
}

/**
 * Every tab badge, from the one bounded summary.
 *
 * Mounting each panel's query to count its own rows was the alternative, and it costs one request
 * per tab on every session open for numbers that fit in a single answer.
 *
 * Shell, Logs, Traces, and Report are backed by capabilities that land in later task groups. Until
 * then their runtime reports coverage rather than fabricated totals, and these badges render as
 * placeholders — which is why the mapper reads `coverage` before it reads any count.
 */
export function workspaceTabBadges(
  summary: WorkspaceEvidenceSummary | undefined,
  state: WorkspaceSummaryState,
): Partial<Record<SessionTabId, WorkspaceTabBadge>> {
  // No summary at all means no badges at all. A runtime that cannot answer the summary query has
  // no evidence surface to be uncertain about, and stamping six placeholders across the bar would
  // report a limitation of the console as a property of the session. A summary that *does* answer
  // and then admits limited coverage is a different statement, and that one is worth showing.
  if (state !== "ready" || summary === undefined) return {};

  const coverage = summary.coverage.state;
  return {
    changes: badgeFor(summary.changes.unviewedFiles, "neutral", coverage),
    terminal: attentionBadge(
      summary.executionRecords.running,
      summary.executionRecords.failed,
      coverage,
    ),
    shell: badgeFor(summary.shells.live, "neutral", coverage),
    logs: badgeFor(summary.logs.newErrors, "danger", coverage),
    traces: attentionBadge(summary.traces.running, summary.traces.failed, coverage),
    report: badgeFor(summary.verification.failed, "danger", coverage),
  };
}

/** The i18n key describing what a tab's badge counts, used as the badge's accessible name. */
export function workspaceBadgeLabelKey(tab: SessionTabId): string {
  return `workspaceBadge.label.${tab}`;
}
