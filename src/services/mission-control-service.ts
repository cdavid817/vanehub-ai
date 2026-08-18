import type { AgentRun, AgentRunEvent, AgentRunFilter, AgentRunPage } from "../types/agent-run";
import type {
  MissionControlActionInput,
  MissionControlActionReceipt,
  MissionControlOverview,
  MissionControlQuery,
  MissionControlRunDetail,
} from "../types/mission-control";

export interface MissionControlService {
  getAgentRun(runId: string): Promise<AgentRun>;
  listAgentRuns(offset?: number, limit?: number, filter?: AgentRunFilter): Promise<AgentRunPage>;
  listAgentRunEvents(runId: string, offset?: number, limit?: number): Promise<AgentRunEvent[]>;
  cancelAgentRun(runId: string, version: number): Promise<AgentRun>;
  resumeAgentRun(runId: string, version: number): Promise<AgentRun>;
  getMissionControlOverview(query?: MissionControlQuery): Promise<MissionControlOverview>;
  getMissionControlRun(runId: string): Promise<MissionControlRunDetail>;
  performMissionControlAction(input: MissionControlActionInput): Promise<MissionControlActionReceipt>;
}
