import type { DerivedGoalStatus, Goal, GoalLink, GoalLinkTarget } from "../contracts/goal";
import { goalLinkTargets } from "../contracts/goal";

export function groupLinks(links: GoalLink[]): { kind: GoalLinkTarget; links: GoalLink[] }[] {
  return goalLinkTargets
    .map((kind) => ({ kind, links: links.filter((link) => link.targetKind === kind) }))
    .filter((group) => group.links.length > 0);
}

/** Children that count toward acceptance and have not finished. */
export function blockingLinks(goal: Goal): GoalLink[] {
  return goal.links.filter((link) => !["session", "run"].includes(link.targetKind) && link.progress === "active");
}

export function unresolvableLinks(goal: Goal): GoalLink[] {
  return goal.links.filter((link) => link.progress === "unresolvable");
}

export function canAccept(goal: Goal): boolean {
  return goal.derivedStatus === "awaiting_acceptance";
}

/**
 * Why a goal is not ready, so the detail view can say it instead of showing a
 * progress bar that simply stops moving.
 */
export type BlockingReason = "none" | "not-active" | "no-children" | "children-running";

export function blockingReason(goal: Goal): BlockingReason {
  if (canAccept(goal)) return "none";
  if (goal.status !== "active") return "not-active";
  if (goal.counted === 0) return "no-children";
  return "children-running";
}

const TONES: Record<DerivedGoalStatus, string> = {
  draft: "bg-muted text-muted-foreground",
  active: "bg-primary/10 text-primary",
  awaiting_acceptance: "bg-amber-500/15 text-amber-600 dark:text-amber-400",
  achieved: "bg-emerald-500/15 text-emerald-600 dark:text-emerald-400",
  abandoned: "bg-muted text-muted-foreground line-through",
};

export function statusTone(status: DerivedGoalStatus): string {
  return TONES[status];
}

export function progressLabel(goal: Goal): string {
  return `${goal.terminal}/${goal.counted}`;
}

/** 0..1 for the list's progress meter. A goal with nothing counted reads as empty, not as done. */
export function progressRatio(goal: Goal): number {
  if (goal.counted <= 0) return 0;
  return Math.min(1, Math.max(0, goal.terminal / goal.counted));
}
