import type { CliEnvironmentSnapshot } from "../../types/cli-environment-snapshot";

/**
 * Reading the backend's decisions off a snapshot.
 *
 * Nothing here decides anything. There is no version comparison, no upgrade-versus-downgrade
 * derivation, and no capability inference: the backend made all three, and the previous
 * implementation making them again on this side is what let a page offer an "upgrade" to the
 * version already installed and then install a different one.
 *
 * When the user picks a target, the page sends it with no action at all and the backend derives the
 * direction. These functions only answer "is there any machine change on offer here", which is a
 * question the backend already answered in `allowedActions`.
 */

/** Actions that change the machine. Anything else is a read. */
const MUTATING_ACTIONS = ["install", "upgrade", "downgrade", "reinstall"];

/** The source that owns the recommended installation, or the only source there is. */
export function recommendedSourceId(snapshot: CliEnvironmentSnapshot): string | null {
  const recommended = snapshot.installations.find(
    (installation) => installation.id === snapshot.recommendedInstallationId,
  );
  return recommended?.sourceId ?? snapshot.sources[0]?.sourceId ?? null;
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
  const recommended = snapshot.installations.find(
    (installation) => installation.id === snapshot.recommendedInstallationId,
  );
  return recommended?.reportedVersion ?? null;
}

/** Whether any conflict the backend reported withholds machine changes for this tool. */
export function mutationBlocked(snapshot: CliEnvironmentSnapshot): boolean {
  return snapshot.conflicts.some((conflict) => conflict.blocksMutation);
}

/**
 * Whether a change to `targetVersion` is on offer at all.
 *
 * Two reasons it is not: the backend offered no mutating action for the owning source, or the
 * target is what is already installed. The second is an equality check on the string the backend
 * itself reported, not a version comparison -- there is nothing to order and no direction to infer.
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
