import type {
  CliEnvironmentSnapshot,
  CliInstallation,
} from "../../../types/cli-environment-snapshot";

/**
 * Reading the backend's decisions off a snapshot.
 *
 * Nothing here decides anything. There is no version comparison, no upgrade-versus-downgrade
 * derivation, no capability inference, and no command construction: the backend made all four, and
 * the previous implementation making them again on this side is what let a page offer an "upgrade"
 * to the version already installed and then install a different one.
 *
 * What these do is read `allowedActions`, `sources`, `conflicts` and `installations` -- which is
 * the whole reason the backend sends them.
 */

/** Actions that change the machine. Anything else is a read. */
const MUTATING_ACTIONS = ["install", "upgrade", "downgrade", "reinstall"];

/** The five buckets the summary bar counts. A tool lands in exactly one. */
export type CliSummaryBucket = "ready" | "needsLogin" | "updates" | "conflicts" | "broken";

export type CliSummaryCounts = Record<CliSummaryBucket, number>;

/**
 * The bucket for each overall state the backend can report.
 *
 * `missing` and `unknown` are deliberately absent: nothing installed and nothing wrong with it is
 * not a problem, and not ready either, so it belongs in no count.
 */
const BUCKET_BY_OVERALL_STATE: Readonly<Record<string, CliSummaryBucket>> = {
  broken: "broken",
  conflict: "conflicts",
  "needs-auth": "needsLogin",
  "update-available": "updates",
  ready: "ready",
};

/**
 * Which bucket a tool belongs to.
 *
 * A lookup on `overallState`, not a second precedence ladder. The backend already ordered these
 * (broken > conflict > needs-auth > update-available > ready), and a tool lands in exactly one, so
 * the counts always add up to at most the number of tools.
 */
export function summaryBucket(snapshot: CliEnvironmentSnapshot): CliSummaryBucket | null {
  return BUCKET_BY_OVERALL_STATE[snapshot.overallState] ?? null;
}

export function summaryCounts(snapshots: readonly CliEnvironmentSnapshot[]): CliSummaryCounts {
  const counts: CliSummaryCounts = { ready: 0, needsLogin: 0, updates: 0, conflicts: 0, broken: 0 };
  for (const snapshot of snapshots) {
    const bucket = summaryBucket(snapshot);
    if (bucket) counts[bucket] += 1;
  }
  return counts;
}

/** Whether this tool is something the user has to act on. Drives the "needs attention" filter. */
export function needsAttention(snapshot: CliEnvironmentSnapshot): boolean {
  const bucket = summaryBucket(snapshot);
  return bucket === "conflicts" || bucket === "broken" || bucket === "needsLogin"
    || bucket === "updates";
}

/** The source that owns the recommended installation, or the only source there is. */
export function recommendedSourceId(snapshot: CliEnvironmentSnapshot): string | null {
  return recommendedInstallation(snapshot)?.sourceId ?? snapshot.sources[0]?.sourceId ?? null;
}

export function recommendedInstallation(
  snapshot: CliEnvironmentSnapshot,
): CliInstallation | undefined {
  return snapshot.installations.find(
    (installation) => installation.id === snapshot.recommendedInstallationId,
  );
}

export function pathSelectedInstallation(
  snapshot: CliEnvironmentSnapshot,
): CliInstallation | undefined {
  return snapshot.installations.find(
    (installation) => installation.id === snapshot.pathSelectedInstallationId,
  );
}

/**
 * Whether what PATH runs differs from what the backend would act on.
 *
 * The one case worth surfacing on a collapsed card: the version shown is not the version a command
 * line would run.
 */
export function pathDivergesFromRecommended(snapshot: CliEnvironmentSnapshot): boolean {
  return (
    snapshot.pathSelectedInstallationId !== null
    && snapshot.recommendedInstallationId !== null
    && snapshot.pathSelectedInstallationId !== snapshot.recommendedInstallationId
  );
}

/** What the target selector shows: one source's catalog, never a merge of several. */
export function targetVersionOptions(snapshot: CliEnvironmentSnapshot): string[] {
  const owning = recommendedSourceId(snapshot);
  const source = snapshot.sources.find((candidate) => candidate.sourceId === owning)
    ?? snapshot.sources[0];
  return source ? [...source.availableVersions] : [];
}

/** The version the recommended installation reports, or `null` when nothing was observed. */
export function installedVersion(snapshot: CliEnvironmentSnapshot): string | null {
  return recommendedInstallation(snapshot)?.reportedVersion ?? null;
}

/** Whether any conflict the backend reported withholds machine changes for this tool. */
export function mutationBlocked(snapshot: CliEnvironmentSnapshot): boolean {
  return snapshot.conflicts.some((conflict) => conflict.blocksMutation);
}

/** The one conflict to explain first. Its code is localized; nothing parses a message. */
export function blockingConflict(snapshot: CliEnvironmentSnapshot) {
  return snapshot.conflicts.find((conflict) => conflict.blocksMutation) ?? snapshot.conflicts[0];
}

/**
 * Whether a change to `targetVersion` is on offer at all.
 *
 * Two reasons it is not: the backend offered no mutating action for the owning source, or the
 * target is what is already installed. The second is a string equality on the version the backend
 * itself reported -- there is nothing to order and no direction to infer.
 */
export function canRequestChange(
  snapshot: CliEnvironmentSnapshot,
  targetVersion: string | null,
): boolean {
  if (mutationBlocked(snapshot)) return false;
  if (targetVersion !== null && targetVersion === installedVersion(snapshot)) return false;
  const sourceId = recommendedSourceId(snapshot);
  return snapshot.allowedActions.some(
    (action) => MUTATING_ACTIONS.includes(action.action) && action.sourceId === sourceId,
  );
}

/** Tools a bulk upgrade would act on, as the backend's own action list reports them. */
export function bulkUpgradeEligible(snapshot: CliEnvironmentSnapshot): boolean {
  return !mutationBlocked(snapshot)
    && snapshot.allowedActions.some((action) => action.action === "upgrade");
}

/** The source summary for a source id, so a card can show its management state and guidance. */
export function sourceSummary(snapshot: CliEnvironmentSnapshot, sourceId: string | null) {
  return snapshot.sources.find((source) => source.sourceId === sourceId);
}

/** Filters a list by the toolbar's controls. Pure, so the page keeps no derived copy of it. */
export function filterSnapshots(
  snapshots: readonly CliEnvironmentSnapshot[],
  filters: {
    search: string;
    state: string;
    source: string;
    attentionOnly: boolean;
  },
): CliEnvironmentSnapshot[] {
  const query = filters.search.trim().toLowerCase();
  return snapshots.filter((snapshot) => {
    if (filters.attentionOnly && !needsAttention(snapshot)) return false;
    if (filters.state !== "all" && summaryBucket(snapshot) !== filters.state) return false;
    if (
      filters.source !== "all"
      && !snapshot.sources.some((source) => source.sourceId === filters.source)
    ) {
      return false;
    }
    if (!query) return true;
    return [snapshot.displayName, snapshot.provider, ...snapshot.executableNames].some((value) =>
      value?.toLowerCase().includes(query),
    );
  });
}

/** Every source id present across the list, for the source filter. */
export function availableSourceIds(snapshots: readonly CliEnvironmentSnapshot[]): string[] {
  return [...new Set(snapshots.flatMap((snapshot) => snapshot.sources.map((s) => s.sourceId)))]
    .sort();
}
