import type { Goal, GoalInput, GoalLinkTarget } from "../contracts/goal";

export interface GoalService {
  listGoals(): Promise<Goal[]>;
  getGoal(goalId: string): Promise<Goal>;
  createGoal(input: GoalInput): Promise<Goal>;
  updateGoal(goalId: string, input: GoalInput): Promise<Goal>;
  deleteGoal(goalId: string): Promise<void>;
  linkGoalTarget(goalId: string, targetKind: GoalLinkTarget, targetId: string): Promise<Goal>;
  unlinkGoalTarget(goalId: string, targetKind: GoalLinkTarget, targetId: string): Promise<Goal>;
  /** Draft or abandoned into active work. */
  activateGoal(goalId: string): Promise<Goal>;
  /** Rejected unless the goal currently derives to `awaiting_acceptance`. */
  acceptGoal(goalId: string): Promise<Goal>;
  reopenGoal(goalId: string): Promise<Goal>;
  abandonGoal(goalId: string): Promise<Goal>;
}
