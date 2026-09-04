import { goalService } from "../services/runtime-goal-client";
import type { DerivedGoalStatus, Goal } from "../contracts/goal";
import type { SemanticStatus, WorkbenchSearchProvider, WorkbenchSearchRequest, WorkbenchSearchResult } from "./command-center-types";

/**
 * Mirrors `goal-presentation.ts`'s own `TONES` color intent (draft/abandoned muted,
 * active primary, awaiting_acceptance amber, achieved emerald) rather than inventing a second,
 * possibly-drifting mapping from the same source status.
 */
function toStatus(status: DerivedGoalStatus): SemanticStatus {
  if (status === "awaiting_acceptance") return "attention";
  if (status === "active") return "active";
  if (status === "achieved") return "success";
  return "neutral";
}

function toSearchResult(goal: Goal): WorkbenchSearchResult {
  return {
    key: goal.id,
    kind: "goal",
    title: goal.title,
    status: toStatus(goal.derivedStatus),
    route: { destination: "plan", section: "goals", goalId: goal.id },
    updatedAt: goal.updatedAt,
  };
}

/**
 * design.md Decision 4 privacy rule: only local safe summaries. `description`/`acceptanceNotes`/
 * `links` are deliberately excluded from `toSearchResult` above -- never read here either, to keep
 * this provider's safety auditable by field name alone, the same discipline
 * `run-search-provider.ts` already follows for `reasonCode`.
 */
export const goalSearchProvider: WorkbenchSearchProvider = {
  id: "goals",
  supports: (scope) => scope === "goal",
  // request.signal is intentionally unused: listGoals isn't abortable, and a shared orchestrator
  // discards stale results centrally rather than each provider doing it itself -- same reasoning
  // as run-search-provider.ts.
  async search(request: WorkbenchSearchRequest) {
    const goals = await goalService.listGoals();
    const needle = request.query.trim().toLowerCase();
    const matched = needle ? goals.filter((goal) => goal.title.toLowerCase().includes(needle)) : goals;
    return { items: matched.slice(0, request.limit).map(toSearchResult), nextCursor: null };
  },
};
