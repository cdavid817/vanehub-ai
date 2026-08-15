import type { GoalService } from "./goal-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriGoalClient } from "./tauri-goal-client";
import { webGoalClient } from "./web-goal-client";

export function createGoalService(): GoalService {
  return createRuntimeAdapter({ tauri: tauriGoalClient, webMock: webGoalClient });
}

export const goalService = createGoalService();
