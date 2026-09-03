import bulk from "./web-cli-bulk-fixtures.json";
import snapshots from "./web-cli-environment-snapshots.json";
import {
  CLI_BULK_SKIP_REASONS,
  CLI_MUTATION_OUTCOMES,
  type CliBulkItemResult,
} from "../types/cli-environment";
import type {
  CliActionPlan,
  CliBulkActionPlan,
  CliEnvironmentSnapshot,
} from "../types/cli-environment-snapshot";

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
export const WEB_CLI_OUTCOME_TARGETS = Object.freeze(bulk.outcomeTargets);

/**
 * Targets that make `prepareCliAction` hand back an already-unusable plan.
 *
 * A refusal is otherwise unreachable from a browser: a freshly prepared plan is always a usable
 * draft, and no amount of clicking produces one that expired or was revised underneath. Selecting
 * these is how the review dialog's refusal path gets exercised at all.
 */
export const WEB_CLI_REFUSAL_TARGETS = Object.freeze(bulk.refusalTargets);

/** Plan ids the mock always answers for, one per refusal the real backend can produce. */
export const WEB_CLI_FIXED_PLAN_IDS = Object.freeze(bulk.fixedPlanIds);

interface WebCliOutcome {
  status: "succeeded" | "failed";
  outcome: string;
  error: string | null;
}

/** Which terminal outcome a chosen target version drives. Unlisted targets verify. */
export function webCliOutcomeFor(targetVersion: string | null): WebCliOutcome {
  const listed: Record<string, { status: string; outcome: string; error: string | null }> = bulk.outcomes;
  const found = listed[targetVersion ?? ""];
  if (!found) return { status: "succeeded", outcome: "verified", error: null };
  return { ...found, status: found.status === "failed" ? "failed" : "succeeded" };
}

export function webCliEnvironmentSnapshots(): CliEnvironmentSnapshot[] {
  // Copied per call: a caller that mutated what it was handed would change every later call's
  // answer, and a mock whose data drifts is worse than no mock.
  return snapshots.map((snapshot) => ({
    ...snapshot,
    installations: snapshot.installations.map((installation) => ({ ...installation })),
    conflicts: snapshot.conflicts.map((conflict) => ({ ...conflict })),
    // JSON widens the literal union to `string`; narrowing here keeps the fixture data honest
    // without an `as` cast that would let a typo through.
    sources: snapshot.sources.map((source) => ({
      ...source,
      management: source.management === "managed" ? ("managed" as const) : ("detect-only" as const),
    })),
    allowedActions: snapshot.allowedActions.map((action) => ({ ...action })),
  }));
}

/** The batch preview, and the per-item results running it produces. Data, so it lives in JSON. */
export function webCliBulkActionPlan(planId: string): CliBulkActionPlan {
  return { ...bulk.plan, id: planId };
}

export function webCliBulkItemResults(): CliBulkItemResult[] {
  // JSON widens every literal to `string`. Each arm is rebuilt against the union rather than cast,
  // so a typo in the fixture fails here instead of reaching a UI as a code nothing can localize.
  return bulk.items.map((item): CliBulkItemResult => {
    const base = {
      agentId: item.agentId,
      planId: item.planId,
      sourceId: item.sourceId,
      targetVersion: item.targetVersion,
    };
    if (item.status === "completed") {
      const outcome = CLI_MUTATION_OUTCOMES.find((value) => value === item.outcome);
      if (!outcome) throw new Error(`unknown bulk outcome: ${item.outcome}`);
      return { ...base, status: "completed", outcome, reason: null };
    }
    const reason = CLI_BULK_SKIP_REASONS.find((value) => value === item.reason);
    if (!reason) throw new Error(`unknown bulk skip reason: ${item.reason}`);
    return { ...base, status: "skipped", outcome: null, reason };
  });
}

export function webCliActionPlan(overrides: Partial<CliActionPlan> & { id: string }): CliActionPlan {
  return { ...bulk.actionPlan, ...overrides };
}
