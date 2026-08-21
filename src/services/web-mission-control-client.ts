import type { AgentRun } from "../types/agent-run";
import type {
  MissionControlActionReceipt,
  MissionControlOverview,
  MissionControlQuery,
  MissionControlRunDetail,
  MissionControlRunSummary,
} from "../types/mission-control";
import type { AgentService } from "./agent-service";
import type { MissionControlService } from "./mission-control-service";
import {
  isActiveWebRunState,
  isTerminalWebRunState,
  listWebAgentRunEvents,
  listWebAgentRuns,
  updateWebAgentRun,
} from "./web-agent-run-state";

function webMissionSummary(run: AgentRun): MissionControlRunSummary {
  const session = run.links.find((link) => link.linkType === "session");
  const review = run.links.find((link) => link.linkType === "review");
  const attention = run.state === "waiting_approval" ? "approval" : run.state === "waiting_user" ? "user"
    : ["blocked", "stuck"].includes(run.state) ? "stuck" : run.state === "failed" ? "failed" : review ? "review" : null;
  const actions: MissionControlRunSummary["actions"] = ["open"];
  if (!isTerminalWebRunState(run.state)) actions.push("cancel");
  if (["paused", "blocked", "stuck"].includes(run.state)) actions.push("resume");
  if (run.state === "waiting_approval") actions.push("approval");
  if (review) actions.push("review");
  return {
    runId: run.id, version: run.version, ownerType: run.owner.ownerType, ownerId: run.owner.ownerId,
    agentId: run.owner.ownerType === "agent" ? run.owner.ownerId : null, title: `Run ${run.owner.ownerId}`,
    state: run.state, createdAt: run.createdAt, updatedAt: run.updatedAt,
    endedAt: isTerminalWebRunState(run.state) ? run.updatedAt : null, projectId: null, workspace: null,
    phase: run.state, attention, reasonCode: run.reasonCode,
    verification: run.state === "verifying" ? "running" : run.state === "completed" ? "passed" : run.state === "failed" ? "failed" : "unavailable",
    tokens: null, cost: null, actions,
    navigation: review ? { kind: "review", id: review.linkId, sessionId: session?.linkId } : session ? { kind: "session", id: session.linkId } : null,
    runner: run.runner ?? null,
  };
}

function webMissionOverview(query: MissionControlQuery): MissionControlOverview {
  const limit = Math.max(1, Math.min(query.limit ?? 20, 50));
  const offset = query.cursor ? Number(query.cursor) : 0;
  if (!Number.isSafeInteger(offset) || offset < 0) throw new Error("invalid mission control cursor");
  let runs = listWebAgentRuns().map(webMissionSummary).filter((run) =>
    (!query.states?.length || query.states.includes(run.state))
    && (!query.agentId || run.agentId === query.agentId)
    && (!query.projectId || run.projectId === query.projectId)
    && (!query.runner || run.runner?.kind === query.runner));
  const priority = (run: MissionControlRunSummary) => run.attention ? 0 : 1;
  runs = runs.sort((left, right) => query.sort === "oldest"
    ? left.createdAt.localeCompare(right.createdAt)
    : query.sort === "attention" ? priority(left) - priority(right) || right.createdAt.localeCompare(left.createdAt)
      : right.createdAt.localeCompare(left.createdAt));
  const page = (items: MissionControlRunSummary[]) => ({ items: items.slice(offset, offset + limit), nextCursor: offset + limit < items.length ? String(offset + limit) : null });
  const count = (state: AgentRun["state"]) => listWebAgentRuns().filter((run) => run.state === state).length;
  return {
    counts: { running: count("running"), waitingApproval: count("waiting_approval"), waitingUser: count("waiting_user"), retrying: count("retrying"), blocked: count("blocked") + count("stuck"), failed: count("failed"), completedRecently: count("completed") },
    attention: page(runs.filter((run) => run.attention)), active: page(runs.filter((run) => isActiveWebRunState(run.state))),
    recent: page(runs.filter((run) => isTerminalWebRunState(run.state))),
  };
}

export const webMissionControlClient: MissionControlService = {
  async getAgentRun(runId) {
    const run = listWebAgentRuns().find((item) => item.id === runId);
    if (!run) throw new Error(`run not found: ${runId}`);
    return run;
  },
  async listAgentRuns(offset = 0, limit = 50, filter) {
    const bounded = Math.max(1, Math.min(limit, 100));
    const filtered = listWebAgentRuns().filter((run) =>
      (!filter?.ownerType || run.owner.ownerType === filter.ownerType)
      && (!filter?.ownerId || run.owner.ownerId === filter.ownerId)
      && (!filter?.parentRunId || run.parentRunId === filter.parentRunId)
      && (!filter?.state || run.state === filter.state));
    return { items: filtered.slice(offset, offset + bounded), offset, limit: bounded };
  },
  async listAgentRunEvents(runId, offset = 0, limit = 50) {
    const bounded = Math.max(1, Math.min(limit, 100));
    return listWebAgentRunEvents(runId).slice(offset, offset + bounded);
  },
  async cancelAgentRun(runId, version) {
    const cancelled = updateWebAgentRun(runId, version, "cancelled");
    for (const child of listWebAgentRuns().filter((run) => run.parentRunId === runId)) {
      if (!["completed", "failed", "cancelled"].includes(child.state)) {
        updateWebAgentRun(child.id, child.version, "cancelled");
      }
    }
    return cancelled;
  },
  async resumeAgentRun(runId, version) {
    const run = listWebAgentRuns().find((item) => item.id === runId);
    if (!run || !["paused", "blocked", "stuck"].includes(run.state)) throw new Error("run cannot be resumed");
    return updateWebAgentRun(runId, version, "running");
  },
  async getMissionControlOverview(query = {}) { return structuredClone(webMissionOverview(query)); },
  async getMissionControlRun(runId): Promise<MissionControlRunDetail> {
    const run = listWebAgentRuns().find((item) => item.id === runId);
    if (!run) throw new Error(`run not found: ${runId}`);
    const linked = new Set(run.links.map((link) => link.linkType));
    const facets: MissionControlRunDetail["facets"] = ["overview", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"].map((facet) => ({
      facet: facet as MissionControlRunDetail["facets"][number]["facet"],
      state: facet === "overview" || facet === "timeline" || facet === "logs" || linked.has(facet) ? "available" : "unavailable",
    }));
    return structuredClone({ run: webMissionSummary(run), facets });
  },
  async performMissionControlAction(this: AgentService, input): Promise<MissionControlActionReceipt> {
    const current = listWebAgentRuns().find((run) => run.id === input.runId);
    if (!current) throw new Error(`run not found: ${input.runId}`);
    if (current.version !== input.version) throw new Error("run version conflict");
    if (input.action === "cancel") return { run: webMissionSummary(await this.cancelAgentRun(input.runId, input.version)), operationId: null };
    if (input.action === "resume") return { run: webMissionSummary(await this.resumeAgentRun(input.runId, input.version)), operationId: null };
    throw new Error("mission control action is unsupported");
  },
};
