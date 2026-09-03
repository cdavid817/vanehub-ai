import { linkableGoalTargets } from "../contracts/goal";
import { lifecycleLabelKey } from "../lib/session-lifecycle";
import { agentService } from "../services/runtime-agent-client";
import { workBoardService } from "../services/runtime-work-board-client";
import type { AgentRunState } from "../types/agent-run";
import type { SessionLifecycleState } from "../types/agent";
import type { WorkItemStage } from "../types/work-board";
import type { StatusTone } from "../ui/status/StatusBadge";

/**
 * design.md Decision 12's `ExecutionTargetPicker`: per-kind search providers for Session, Run,
 * Loop, and Work Item, each returning a normalized "safe summary" (title/project/status) plus a
 * stable id -- never a prompt, response, or other content field. Deliberately plain async
 * functions with their own narrow return shape, not `WorkbenchSearchProvider`
 * (command-center/command-center-types.ts): that type's `route` is a full `WorkbenchLocation`
 * (linking a target to a goal has no navigation destination to populate), and its
 * `scopes`/`cursor` fields exist for command-center's own cross-domain merged search, which this
 * single-kind picker has no use for either.
 */
export type ExecutionTargetKind = (typeof linkableGoalTargets)[number];

export interface ExecutionTargetOption {
  id: string;
  title: string;
  projectPath: string | null;
  /** A ready-to-use i18n key, reused verbatim from each domain's own existing label namespace
   *  (mission-control-run-card.tsx's `missionControl.state.*`, lib/session-lifecycle.ts's
   *  `layout.lifecycle.*`, work-item-stage-menu.tsx's `todoBoard.stage.*`,
   *  loop-definition-overview.tsx's `loops.definition.enabled`/`disabled`) rather than inventing a
   *  parallel status vocabulary this picker would own alone. */
  statusKey: string;
  statusTone: StatusTone;
}

export type ExecutionTargetSearch = (query: string) => Promise<ExecutionTargetOption[]>;
export type ExecutionTargetProviders = Record<ExecutionTargetKind, ExecutionTargetSearch>;

const RESULT_LIMIT = 20;

function matchesQuery(haystack: string, query: string): boolean {
  return haystack.toLowerCase().includes(query.trim().toLowerCase());
}

const SESSION_TONE: Record<SessionLifecycleState, StatusTone> = {
  idle: "neutral", starting: "running", running: "running", failed: "danger", stopped: "neutral",
};

/** `searchSessions` itself returns `[]` for an empty query (web-session-query-client.ts) -- there
 *  is no "browse all sessions" search mode, so an empty picker query honestly shows no results
 *  here rather than this provider inventing one by calling `listSessions` instead. */
async function searchSessionTargets(query: string): Promise<ExecutionTargetOption[]> {
  const results = await agentService.searchSessions({ query, limit: RESULT_LIMIT });
  return results.map(({ session }) => ({
    id: session.id,
    title: session.title,
    projectPath: session.projectPath,
    statusKey: lifecycleLabelKey(session.lifecycleState),
    statusTone: SESSION_TONE[session.lifecycleState],
  }));
}

const RUN_TONE: Record<AgentRunState, StatusTone> = {
  created: "neutral", preparing: "running", running: "running", verifying: "running", retrying: "running",
  waiting_approval: "attention", waiting_user: "attention", paused: "attention", blocked: "blocked", stuck: "blocked",
  completed: "success", failed: "danger", cancelled: "neutral",
};

/** `getMissionControlOverview` has no free-text query -- mirrors run-search-provider.ts's own
 *  approach exactly: fetch a generous page per section, dedupe by runId across
 *  attention/active/recent, then filter client-side. */
async function searchRunTargets(query: string): Promise<ExecutionTargetOption[]> {
  const overview = await agentService.getMissionControlOverview({ limit: 50, sort: "newest" });
  const seen = new Set<string>();
  const candidates = [];
  for (const run of [...overview.attention.items, ...overview.active.items, ...overview.recent.items]) {
    if (seen.has(run.runId)) continue;
    seen.add(run.runId);
    candidates.push(run);
  }
  const filtered = query.trim() ? candidates.filter((run) => matchesQuery(run.title, query)) : candidates;
  return filtered.slice(0, RESULT_LIMIT).map((run) => ({
    id: run.runId,
    title: run.title,
    // `workspace` is this type's own nearest equivalent to `projectPath` (types/mission-control.ts)
    // -- normalized to the same option field name the other three kinds use, not a second concept.
    // It is `null` in every Web/mock run today (web-mission-control-client.ts), which is honest,
    // not a bug this picker should paper over: the browser build has no real workspace to report.
    projectPath: run.workspace,
    statusKey: `missionControl.state.${run.state}`,
    statusTone: RUN_TONE[run.state],
  }));
}

/** `listLoopDefinitions` has no query parameter either -- filtered client-side, same shape as the
 *  run provider above. A "loop" `GoalLink.targetId` is a loop *definition* id, not a run id --
 *  confirmed by reading `LoopProgressProbe::resolve` (src-tauri/.../progress_probes.rs): it looks
 *  up `loop_definitions WHERE id = ?1` first, then separately reads that definition's *latest* run
 *  only to derive terminal/active. The identity anchor is the definition. */
async function searchLoopTargets(query: string): Promise<ExecutionTargetOption[]> {
  const definitions = await agentService.listLoopDefinitions();
  const filtered = query.trim() ? definitions.filter((definition) => matchesQuery(definition.name, query)) : definitions;
  return filtered.slice(0, RESULT_LIMIT).map((definition) => ({
    id: definition.id,
    title: definition.name,
    projectPath: definition.projectPath,
    statusKey: definition.enabled ? "loops.definition.enabled" : "loops.definition.disabled",
    statusTone: definition.enabled ? "success" : "neutral",
  }));
}

const WORK_ITEM_TONE: Record<WorkItemStage, StatusTone> = {
  inbox: "neutral", planned: "neutral", in_progress: "running", review: "attention", done: "success",
};

/** `archived: false` is explicit, not relied on as the filter's own default: a picker should not
 *  offer an archived item as a fresh link target, and spelling this out means that choice survives
 *  even if `listWorkItems`'s own default semantics ever change. */
async function searchWorkItemTargets(query: string): Promise<ExecutionTargetOption[]> {
  const items = await workBoardService.listWorkItems({ archived: false, query });
  return items.slice(0, RESULT_LIMIT).map((item) => ({
    id: item.id,
    title: item.title,
    projectPath: item.projectPath,
    statusKey: `todoBoard.stage.${item.stage}`,
    statusTone: WORK_ITEM_TONE[item.stage],
  }));
}

export const executionTargetSearchProviders: ExecutionTargetProviders = {
  loop: searchLoopTargets,
  work_item: searchWorkItemTargets,
  session: searchSessionTargets,
  run: searchRunTargets,
};
