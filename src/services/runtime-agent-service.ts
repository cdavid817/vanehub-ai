import type { AgentService } from "./agent-service";
import type { LoopWorkbenchService } from "./loop-service";

export type RuntimeAgentService = AgentService & LoopWorkbenchService;
