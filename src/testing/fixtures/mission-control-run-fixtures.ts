import type { AgentRunRunner, AgentRunState } from "../../types/agent-run";
import type { MissionControlAction, MissionControlAttention, MissionControlRunSummary } from "../../types/mission-control";
import {
  DEFAULT_SEED,
  FIXTURE_RANGE_END_MS,
  FIXTURE_RANGE_START_MS,
  type SeededRandom,
  chance,
  createIdFactory,
  createSeededRandom,
  isoTimestamp,
  maybeLong,
  nextInt,
  offsetTimestamp,
  pick,
  pickWeighted,
  title,
} from "./seeded-random";

const STATE_WEIGHTS: ReadonlyArray<readonly [AgentRunState, number]> = [
  ["completed", 38], ["running", 15], ["failed", 10], ["cancelled", 8], ["waiting_approval", 6],
  ["waiting_user", 5], ["blocked", 4], ["stuck", 3], ["retrying", 4], ["paused", 3],
  ["verifying", 2], ["preparing", 1], ["created", 1],
];

const TERMINAL_STATES: ReadonlySet<AgentRunState> = new Set(["completed", "failed", "cancelled"]);

/** No literal `blocked` attention bucket exists; `stuck` is the closest fit for a run that cannot proceed. */
function attentionFor(rng: SeededRandom, state: AgentRunState): MissionControlAttention | null {
  switch (state) {
    case "waiting_approval": return "approval";
    case "waiting_user": return "user";
    case "stuck":
    case "blocked": return "stuck";
    case "failed": return "failed";
    case "completed": return chance(rng, 0.08) ? "review" : null;
    default: return null;
  }
}

function verificationFor(rng: SeededRandom, state: AgentRunState): MissionControlRunSummary["verification"] {
  if (TERMINAL_STATES.has(state)) return pickWeighted(rng, [["passed", 55], ["failed", 20], ["unavailable", 25]] as const);
  if (state === "verifying") return "running";
  if (state === "created" || state === "preparing") return "pending";
  return pickWeighted(rng, [["pending", 40], ["running", 30], ["unavailable", 30]] as const);
}

function actionsFor(state: AgentRunState): MissionControlAction[] {
  if (state === "waiting_approval") return ["open", "approval", "cancel"];
  if (state === "waiting_user" || state === "blocked" || state === "stuck") return ["open", "resume", "cancel"];
  if (TERMINAL_STATES.has(state)) return ["open", "review"];
  return ["open", "cancel"];
}

function buildRunner(rng: SeededRandom, index: number): AgentRunRunner {
  const kind = pick(rng, ["local", "ssh"] as const);
  return {
    kind,
    targetId: `${kind}-target-${index % 25}`,
    targetRevision: nextInt(rng, 1, 12),
    label: `${kind === "ssh" ? "Remote" : "Local"} runner ${index % 25}`,
    hostLabel: kind === "ssh" ? `host-${index % 25}.internal` : null,
    recovery: pick(rng, ["none", "inspect_only", "reattach"] as const),
    capabilityWitness: `capability-${index}`,
    authorityWitness: `authority-${index}`,
    recoveryReference: chance(rng, 0.3) ? `recovery-${index}` : null,
  };
}

/**
 * `count` deterministic `MissionControlRunSummary` rows -- the flat, UI-ready shape actually
 * rendered by `src/mission-control/mission-control.tsx`'s run cards, including the `attention`
 * taxonomy an attention-first redesign needs to exercise (approval/user/stuck/failed/review) and
 * every `AgentRunState` the domain defines. See the fixture generator's report for why this type
 * was chosen over the lower-level `AgentRun` entity.
 */
export function generateMissionControlRuns(
  count: number,
  sessionIds: readonly string[],
  seed: number = DEFAULT_SEED,
): MissionControlRunSummary[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("run");
  const runs: MissionControlRunSummary[] = [];

  for (let index = 0; index < count; index += 1) {
    const state = pickWeighted(rng, STATE_WEIGHTS);
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const updatedAt = offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 3, rng);
    const terminal = TERMINAL_STATES.has(state);
    const sessionId = sessionIds.length > 0 ? pick(rng, sessionIds) : null;

    runs.push({
      runId: nextId(),
      version: nextInt(rng, 1, 16),
      ownerType: "session",
      ownerId: sessionId ?? `session-unknown-${index}`,
      agentId: pick(rng, ["claude-code", "codex-cli", "opencode", "antigravity-cli", "gemini-cli", "onepiece"] as const),
      title: maybeLong(rng, () => title(rng, 3, 8), () => title(rng, 220, 380)),
      state,
      createdAt,
      updatedAt,
      endedAt: terminal ? offsetTimestamp(updatedAt, 0, 1000 * 60 * 30, rng) : null,
      projectId: chance(rng, 0.7) ? `project-${index % 60}` : null,
      workspace: chance(rng, 0.5) ? title(rng, 1, 2) : null,
      phase: chance(rng, 0.6) ? pick(rng, ["planning", "implementing", "verifying", "reviewing"] as const) : null,
      attention: attentionFor(rng, state),
      reasonCode: state === "failed" || state === "blocked" || state === "stuck" ? `reason-${state}` : null,
      verification: verificationFor(rng, state),
      tokens: chance(rng, 0.85) ? nextInt(rng, 500, 250_000) : null,
      cost: chance(rng, 0.6) ? Number((rng() * 12).toFixed(4)) : null,
      actions: actionsFor(state),
      navigation: sessionId ? { kind: "session", id: sessionId, sessionId } : null,
      runner: chance(rng, 0.5) ? buildRunner(rng, index) : null,
    });
  }

  return runs;
}
