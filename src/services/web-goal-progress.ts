import type {
  DerivedGoalStatus,
  GoalLink,
  GoalLinkProgress,
  GoalLinkTarget,
  GoalStatus,
} from "../contracts/goal";

/**
 * Which targets exist in the Web runtime, and what each one reports.
 *
 * The set is fixture data, but the values encode the asymmetry the desktop
 * runtime has to honour: a failed plan run is retryable and therefore still
 * active, while a failed loop run is terminal. Anything outside this catalog is
 * unresolvable, which is what the desktop runtime reports for a deleted target.
 */
const CATALOG: ReadonlyMap<string, GoalLinkProgress> = new Map([
  ["plan:web-plan-completed", "terminal"],
  ["plan:web-plan-cancelled", "terminal"],
  ["plan:web-plan-archived", "terminal"],
  // Retryable: the plan run state machine allows failed -> running.
  ["plan:web-plan-failed", "active"],
  // Waiting on a human of its own, so it cannot promote its goal.
  ["plan:web-plan-awaiting", "active"],
  ["plan:web-plan-running", "active"],
  // Loops have no edge out of failed, so failure ends the loop.
  ["loop:web-loop-failed", "terminal"],
  ["loop:web-loop-succeeded", "terminal"],
  ["loop:web-loop-cancelled", "terminal"],
  ["loop:web-loop-awaiting", "active"],
  ["loop:web-loop-running", "active"],
  ["work_item:web-work-item-done", "terminal"],
  ["work_item:web-work-item-archived", "terminal"],
  ["work_item:web-work-item-doing", "active"],
  ["session:web-session-open", "active"],
]);

export function probeTarget(targetKind: GoalLinkTarget, targetId: string): GoalLinkProgress {
  return CATALOG.get(`${targetKind}:${targetId}`) ?? "unresolvable";
}

export function participatesInDerivation(targetKind: GoalLinkTarget): boolean {
  return targetKind !== "session" && targetKind !== "run";
}

export interface GoalProgressTotals {
  derivedStatus: DerivedGoalStatus;
  counted: number;
  terminal: number;
  unresolvable: number;
}

/**
 * Mirrors `contexts/goals/application/progress.rs`. Sessions never count, and
 * unresolvable children leave the denominator so a deleted plan cannot strand a
 * goal short of acceptance.
 */
export function deriveProgress(status: GoalStatus, links: GoalLink[]): GoalProgressTotals {
  let counted = 0;
  let terminal = 0;
  let unresolvable = 0;

  for (const link of links) {
    if (!participatesInDerivation(link.targetKind)) continue;
    if (link.progress === "unresolvable") {
      unresolvable += 1;
      continue;
    }
    counted += 1;
    if (link.progress === "terminal") terminal += 1;
  }

  const awaiting = status === "active" && counted > 0 && terminal === counted;
  const derivedStatus: DerivedGoalStatus = awaiting ? "awaiting_acceptance" : status;

  return { derivedStatus, counted, terminal, unresolvable };
}

/** Mirrors `GoalStatus::can_transition_to` in the domain layer. */
export function canTransitionTo(from: GoalStatus, to: GoalStatus): boolean {
  if (to === "abandoned") return from !== "abandoned";
  if (to === "active") return from !== "active";
  return false;
}
