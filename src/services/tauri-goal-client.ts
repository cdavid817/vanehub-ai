import { invoke } from "@tauri-apps/api/core";
import type { Goal } from "../contracts/goal";
import type { GoalService } from "./goal-service";

export const tauriGoalClient: GoalService = {
  listGoals() { return invoke<Goal[]>("list_goals"); },
  getGoal(goalId) { return invoke<Goal>("get_goal", { goalId }); },
  createGoal(input) { return invoke<Goal>("create_goal", { input }); },
  updateGoal(goalId, input) { return invoke<Goal>("update_goal", { goalId, input }); },
  deleteGoal(goalId) { return invoke<void>("delete_goal", { goalId }); },
  linkGoalTarget(goalId, targetKind, targetId) {
    return invoke<Goal>("link_goal_target", { goalId, targetKind, targetId });
  },
  unlinkGoalTarget(goalId, targetKind, targetId) {
    return invoke<Goal>("unlink_goal_target", { goalId, targetKind, targetId });
  },
  activateGoal(goalId) { return invoke<Goal>("activate_goal", { goalId }); },
  acceptGoal(goalId) { return invoke<Goal>("accept_goal", { goalId }); },
  reopenGoal(goalId) { return invoke<Goal>("reopen_goal", { goalId }); },
  abandonGoal(goalId) { return invoke<Goal>("abandon_goal", { goalId }); },
};
