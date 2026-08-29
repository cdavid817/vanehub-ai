import type {
  ImSessionConnector,
  RemoteWorkspace,
  Session,
  SessionExecutionOrigin,
  SessionLifecycleState,
  SessionRecoveryStatus,
  SessionSourceMetadata,
} from "../../types/agent";
import { managedCliAgentIds } from "../../types/agent";
import type { SessionPersonalizationMode } from "../../types/personalization";
import {
  DEFAULT_SEED,
  FIXTURE_RANGE_END_MS,
  FIXTURE_RANGE_START_MS,
  type SeededRandom,
  chance,
  createIdFactory,
  createSeededRandom,
  fixturePath,
  isoTimestamp,
  maybeLong,
  nextInt,
  offsetTimestamp,
  pick,
  pickWeighted,
  title,
} from "./seeded-random";

const AGENT_IDS = [...managedCliAgentIds, "onepiece"] as const;
const INTERACTION_MODES = ["browser", "native-desktop", "cli", "api"] as const;
const PERSONALIZATION_MODES: readonly SessionPersonalizationMode[] = ["standard", "project-only", "temporary"];
const IM_CONNECTORS: readonly ImSessionConnector[] = ["feishu", "telegram", "dingtalk", "wecom", "weixin"];

const LIFECYCLE_WEIGHTS: ReadonlyArray<readonly [SessionLifecycleState, number]> = [
  ["idle", 55], ["running", 20], ["stopped", 12], ["starting", 8], ["failed", 5],
];
const RECOVERY_WEIGHTS: ReadonlyArray<readonly [SessionRecoveryStatus, number]> = [
  ["clean", 85], ["reconciling", 7], ["action_required", 5], ["quarantined", 3],
];

function buildRemoteWorkspace(rng: SeededRandom, index: number): RemoteWorkspace {
  const host = `build-host-${index % 40}.internal`;
  const path = fixturePath(rng);
  return {
    host,
    port: chance(rng, 0.5) ? 22 : null,
    user: chance(rng, 0.7) ? "vane" : null,
    path,
    displayName: `${host} workspace`,
    uri: `ssh://${host}${path}`,
  };
}

function buildSource(rng: SeededRandom): SessionSourceMetadata {
  return chance(rng, 0.18)
    ? { kind: "im", connector: pick(rng, IM_CONNECTORS) }
    : { kind: "desktop", connector: null };
}

function buildExecutionOrigin(rng: SeededRandom): SessionExecutionOrigin {
  const kind = pickWeighted(rng, [["user", 70], ["scheduled_task", 20], ["plan_attempt", 10]] as const);
  return { kind, id: kind === "user" ? null : `${kind}-${nextInt(rng, 1, 999)}` };
}

/**
 * `count` deterministic sessions covering every lifecycle/recovery state, a mix of local and
 * remote/worktree layouts, and a small fraction of very long titles/paths so later truncation
 * tests have something real to truncate.
 */
export function generateSessions(count: number, seed: number = DEFAULT_SEED): Session[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("session");
  const sessions: Session[] = [];

  for (let index = 0; index < count; index += 1) {
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const updatedAt = offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 30, rng);
    const lifecycleState = pickWeighted(rng, LIFECYCLE_WEIGHTS);
    const hasWorktree = chance(rng, 0.4);
    const hasRemote = chance(rng, 0.08);
    const worktreeSuffix = `wt-${index}`;

    sessions.push({
      id: nextId(),
      personalizationMode: pick(rng, PERSONALIZATION_MODES),
      title: maybeLong(rng, () => title(rng, 2, 7), () => title(rng, 220, 400)),
      agentId: pick(rng, AGENT_IDS),
      interactionMode: pick(rng, INTERACTION_MODES),
      lifecycleState,
      recoveryStatus: pickWeighted(rng, RECOVERY_WEIGHTS),
      recoveryRevision: nextInt(rng, 0, 5),
      stateRevision: nextInt(rng, 1, 200),
      historyRevision: nextInt(rng, 1, 500),
      activeExecutionRunId: lifecycleState === "running" ? `active-run-${index}` : null,
      folder: chance(rng, 0.3) ? title(rng, 1, 2) : null,
      projectPath: chance(rng, 0.85) ? maybeLong(rng, () => fixturePath(rng), () => fixturePath(rng, true)) : null,
      worktreePath: hasWorktree ? `D:/worktrees/${worktreeSuffix}` : null,
      worktreeName: hasWorktree ? worktreeSuffix : null,
      worktreeBranch: hasWorktree ? `vanehub/${worktreeSuffix}` : null,
      remoteWorkspace: hasRemote ? buildRemoteWorkspace(rng, index) : null,
      remoteSshConnectionId: hasRemote ? `ssh-${index}` : null,
      remoteSshConnectionRevision: hasRemote ? nextInt(rng, 1, 10) : null,
      runtimeSessionId: lifecycleState === "idle" || lifecycleState === "stopped" ? null : `runtime-${index}`,
      categoryId: chance(rng, 0.4) ? `category-${nextInt(rng, 0, 8)}` : null,
      source: chance(rng, 0.5) ? buildSource(rng) : undefined,
      executionOrigin: chance(rng, 0.6) ? buildExecutionOrigin(rng) : undefined,
      pinned: chance(rng, 0.06),
      archived: chance(rng, 0.15),
      createdAt,
      updatedAt,
    });
  }

  return sessions;
}
