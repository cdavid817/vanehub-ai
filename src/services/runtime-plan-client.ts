import type { PlanService } from "./plan-service";
import { createRuntimeAdapter } from "./runtime-adapter";
import { tauriPlanClient } from "./tauri-plan-client";
import { webPlanClient } from "./web-plan-client";

export function createPlanService(): PlanService {
  return createRuntimeAdapter({ tauri: tauriPlanClient, webMock: webPlanClient });
}

export const planService = createPlanService();
