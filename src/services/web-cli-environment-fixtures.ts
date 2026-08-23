import snapshots from "./web-cli-environment-snapshots.json";
import type { CliActionPlan, CliEnvironmentSnapshot } from "../types/cli-environment-snapshot";

/**
 * Deterministic CLI environment data for the Web/mock runtime.
 *
 * The snapshots themselves live in `web-cli-environment-snapshots.json`, beside this file, for the
 * same reason `src/config/onepiece-provider-catalog.json` does: they are data, and a couple of
 * hundred lines of object literals in a `.ts` file are data pretending to be code.
 *
 * Every value in them is invented. Nothing here reads the host's PATH, spawns a process, reaches a
 * package manager or the network, touches a credential store, or writes a log -- a browser session
 * has none of those, and pretending otherwise would put a machine's real layout on a page that
 * cannot have looked at one. Paths are obvious placeholders under `/mock`, because a realistic home
 * directory would be read as a real finding.
 *
 * The five tools match the backend registry, and between them cover the cases a UI has to render:
 * an ordinary update, nothing installed, already current, a blocking conflict, and a
 * vendor-installed tool with no catalog at all.
 */

/** Selecting one of these as the target drives the mock to that terminal outcome. */
export const WEB_CLI_OUTCOME_TARGETS = Object.freeze({
  verified: "1.3.0",
  appliedUnverified: "1.3.0-unverified",
  changedButFailed: "1.3.0-changed",
  noChangeFailed: "1.3.0-fails",
  cancelled: "1.3.0-cancels",
});

/** Plan ids the mock always answers for, one per refusal the real backend can produce. */
export const WEB_CLI_FIXED_PLAN_IDS = Object.freeze({
  draft: "web-plan-draft",
  expired: "web-plan-expired",
  consumed: "web-plan-consumed",
  stale: "web-plan-stale",
});

export function webCliEnvironmentSnapshots(): CliEnvironmentSnapshot[] {
  // Copied per call: a caller that mutated what it was handed would change every later call's
  // answer, and a mock whose data drifts is worse than no mock.
  return snapshots.map((snapshot) => ({
    ...snapshot,
    installations: snapshot.installations.map((installation) => ({ ...installation })),
    conflicts: snapshot.conflicts.map((conflict) => ({ ...conflict })),
    sources: snapshot.sources.map((source) => ({ ...source })),
    allowedActions: snapshot.allowedActions.map((action) => ({ ...action })),
  }));
}

export function webCliActionPlan(overrides: Partial<CliActionPlan> & { id: string }): CliActionPlan {
  return {
    revision: 1,
    agentId: "claude-code",
    action: "upgrade",
    sourceId: "npm",
    installationId: "claude",
    currentVersion: "1.2.0",
    targetVersion: "1.3.0",
    channel: "stable",
    commandPreview: {
      program: "npm",
      args: ["install", "--global", "@anthropic-ai/claude-code@1.3.0"],
    },
    preconditions: ["source-executable-available"],
    warnings: [],
    requiresElevation: false,
    requiresNetwork: true,
    state: "draft",
    createdAt: "2026-01-01T00:00:00+00:00",
    expiresAt: "2026-01-01T00:10:00+00:00",
    ...overrides,
  };
}
