/** What a goal stores. `awaiting_acceptance` is deliberately not here. */
export const goalStatuses = ["draft", "active", "achieved", "abandoned"] as const;
export type GoalStatus = (typeof goalStatuses)[number];

/**
 * What a goal presents. `awaiting_acceptance` exists only as a derived value,
 * recomputed from the goal's children on every read, so reopening a child pulls
 * the goal back out of it without any repair step.
 */
export const derivedGoalStatuses = [
  "draft",
  "active",
  "awaiting_acceptance",
  "achieved",
  "abandoned",
] as const;
export type DerivedGoalStatus = (typeof derivedGoalStatuses)[number];

export const goalLinkTargets = ["plan", "loop", "work_item", "session"] as const;
export type GoalLinkTarget = (typeof goalLinkTargets)[number];

/** `unresolvable` means the target was deleted or could not be read. */
export const goalLinkProgressStates = ["terminal", "active", "unresolvable"] as const;
export type GoalLinkProgress = (typeof goalLinkProgressStates)[number];

export interface GoalLink {
  targetKind: GoalLinkTarget;
  targetId: string;
  progress: GoalLinkProgress;
}

export interface Goal {
  id: string;
  title: string;
  description: string;
  acceptanceNotes: string;
  status: GoalStatus;
  derivedStatus: DerivedGoalStatus;
  projectPath: string | null;
  createdAt: string;
  updatedAt: string;
  /** Links that count toward acceptance: neither sessions nor unresolvable. */
  counted: number;
  terminal: number;
  unresolvable: number;
  links: GoalLink[];
}

export interface GoalInput {
  title: string;
  description?: string;
  acceptanceNotes?: string;
  projectPath?: string | null;
}
