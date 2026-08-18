import type { AgentRun, AgentRunEvent } from "../types/agent-run";

// Owned here and never exported; the loop, chat and session contexts all project run state, and
// they do it through the accessors below rather than through a shared binding.
const WEB_RUN_TIME = "2026-08-16T00:00:00.000Z";
let webAgentRuns: AgentRun[] = [{
  id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
  owner: { ownerType: "web_demo", ownerId: "web-session-open" },
  links: [{ linkType: "session", linkId: "web-session-open" }],
  parentRunId: null,
  state: "paused",
  recoveryPolicy: "not_recoverable",
  runner: {
    kind: "local", targetId: "local", targetRevision: null, label: "Local", hostLabel: "This device",
    recovery: "none", capabilityWitness: "web-demo-local", authorityWitness: "web-demo-local", recoveryReference: null,
  },
  retryCount: 1,
  maxRetries: 2,
  reasonCode: "web_demo_paused",
  createdAt: WEB_RUN_TIME,
  updatedAt: WEB_RUN_TIME,
  version: 4,
  lastWitness: "web-demo-pause",
}, ...([
  ["waiting_approval", "approval_required"], ["waiting_user", "user_question"],
  ["retrying", "provider_backoff"], ["stuck", "runner_disconnected"],
  ["failed", "runner_interrupted"], ["completed", null], ["running", null],
] as const).map(([state, reasonCode], index): AgentRun => ({
  id: `018f0f17-4d6a-7e20-b41d-66c5271a29${index}`,
  owner: { ownerType: index === 5 ? "evaluation" : "agent", ownerId: `web-owner-${index}` },
  links: [{ linkType: "session", linkId: `web-session-${index}` }, ...(index === 4 ? [{ linkType: "review", linkId: "web-review-1" }] : [])],
  parentRunId: null, state, recoveryPolicy: "owner_reconciles", retryCount: state === "retrying" ? 1 : 0,
  maxRetries: 2, reasonCode, createdAt: `2026-08-16T00:0${index + 1}:00.000Z`,
  updatedAt: `2026-08-16T00:0${index + 1}:30.000Z`, version: 2, lastWitness: `web-${state}`,
  runner: index === 3 || index === 4 || index === 6 ? {
    kind: "ssh", targetId: "web-demo-ssh", targetRevision: 1, label: "Build host", hostLabel: "build.example.test",
    recovery: "inspect_only", capabilityWitness: "web-demo-ssh", authorityWitness: "web-demo-ssh-v1", recoveryReference: null,
  } : undefined,
}))];
const defaultWebAgentRuns = structuredClone(webAgentRuns);
const webAgentRunEvents = new Map<string, AgentRunEvent[]>([[webAgentRuns[0].id, [{
  sequence: 4,
  state: "paused",
  trigger: "pause",
  timestamp: WEB_RUN_TIME,
  reasonCode: "web_demo_paused",
  witness: "web-demo-pause",
}]]]);

export function seedWebMissionControlRunsForTest(count: 100 | 1_000): void {
  const states: AgentRun["state"][] = [
    "running", "waiting_approval", "waiting_user", "retrying", "blocked", "failed", "completed",
  ];
  webAgentRuns = Array.from({ length: count }, (_, index): AgentRun => {
    const state = states[index % states.length];
    const timestamp = new Date(Date.parse(WEB_RUN_TIME) + index * 1_000).toISOString();
    return {
      id: `018f0f17-4d6a-7e20-b41d-${String(index).padStart(12, "0")}`,
      owner: { ownerType: "agent", ownerId: `performance-agent-${index % 10}` },
      links: [{ linkType: "session", linkId: `performance-session-${index}` }],
      parentRunId: null,
      state,
      recoveryPolicy: "owner_reconciles",
      retryCount: state === "retrying" ? 1 : 0,
      maxRetries: 2,
      reasonCode: ["blocked", "failed"].includes(state) ? `performance_${state}` : null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 1,
      lastWitness: `performance-fixture:${index}`,
    };
  });
  webAgentRunEvents.clear();
}

export function resetWebMissionControlRunsForTest(): void {
  webAgentRuns = structuredClone(defaultWebAgentRuns);
  webAgentRunEvents.clear();
  webAgentRunEvents.set(webAgentRuns[0].id, [{
    sequence: 4,
    state: "paused",
    trigger: "pause",
    timestamp: WEB_RUN_TIME,
    reasonCode: "web_demo_paused",
    witness: "web-demo-pause",
  }]);
}

export function updateWebAgentRun(runId: string, version: number, state: AgentRun["state"]): AgentRun {
  const current = webAgentRuns.find((run) => run.id === runId);
  if (!current) throw new Error(`run not found: ${runId}`);
  if (["completed", "failed", "cancelled"].includes(current.state)) return current;
  if (current.version !== version) throw new Error("run version conflict");
  const nextVersion = version + 1;
  const updatedAt = `2026-08-16T00:00:${String(nextVersion).padStart(2, "0")}.000Z`;
  const updated = { ...current, state, reasonCode: null, version: nextVersion, updatedAt };
  webAgentRuns = webAgentRuns.map((run) => run.id === runId ? updated : run);
  const events = webAgentRunEvents.get(runId) ?? [];
  events.push({
    sequence: nextVersion,
    state,
    trigger: state === "cancelled" ? "cancel_user" : "resume",
    timestamp: updatedAt,
    reasonCode: null,
    witness: `web-${state}:${runId}:${version}`,
  });
  webAgentRunEvents.set(runId, events);
  return updated;
}

export function projectWebOwnerRun(ownerId: string, state: AgentRun["state"]): void {
  const run = webAgentRuns.find((item) => item.owner.ownerId === ownerId);
  if (run && run.state !== state && !["completed", "failed", "cancelled"].includes(run.state)) {
    updateWebAgentRun(run.id, run.version, state);
  }
}

const terminalRunStates = new Set<AgentRun["state"]>(["completed", "failed", "cancelled"]);
const activeRunStates = new Set<AgentRun["state"]>(["created", "preparing", "running", "waiting_approval", "waiting_user", "paused", "retrying", "blocked", "stuck", "verifying"]);
export function listWebAgentRuns(): AgentRun[] {
  return webAgentRuns;
}

export function findWebAgentRun(runId: string): AgentRun | undefined {
  return webAgentRuns.find((run) => run.id === runId);
}

export function prependWebAgentRun(run: AgentRun): void {
  webAgentRuns = [run, ...webAgentRuns];
}

export function listWebAgentRunEvents(runId: string): AgentRunEvent[] {
  return webAgentRunEvents.get(runId) ?? [];
}

export function setWebAgentRunEvents(runId: string, events: AgentRunEvent[]): void {
  webAgentRunEvents.set(runId, events);
}

export function isTerminalWebRunState(state: AgentRun["state"]): boolean {
  return terminalRunStates.has(state);
}

export function isActiveWebRunState(state: AgentRun["state"]): boolean {
  return activeRunStates.has(state);
}
