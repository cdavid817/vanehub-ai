import type { WorkbenchCommand } from "./command-center-types";

/**
 * 6.7. One command per genuinely distinct deep-link target, not one per name a reader might use —
 * Board is Plan's own default section and Mission Control is Runs' own default section, so those
 * two live as searchable `keywords` on their parent's command (also satisfying design.md's "Command
 * Center indexes old keywords" migration note) rather than as separate, behaviorally-identical
 * commands. Goals/Loops/Schedules each route somewhere the parent's default does not, so each gets
 * its own entry.
 *
 * 22.2: `matchedCommands` (command-center.tsx) only ever tests `keyword.includes(query)` — a
 * *longer* pre-redesign term than its current synonym is not found by that check even though the
 * shorter, current term is already listed (e.g. "evaluation" being present does not make
 * "evaluations" match). Verified against the deleted `workspace-route.ts`'s six flat destinations
 * (`git show b3ba029a^`) and each one's real old `layout.activityBar.*` label text, not guessed:
 * three of the five old labels/paths ("Loops", "Goal Center"/"goals", "Mission Control" as a label)
 * already round-trip through an existing keyword unchanged. "work-board"/"evaluations"/
 * "mission-control" are the exact old `LEGACY_DESTINATION_REDIRECTS` path segments (workbench-
 * route.ts), and "任务看板"/"agent 评测" are the exact old zh-CN activity-bar labels — none of the
 * six were a substring of any keyword already present.
 */
export const DESTINATION_COMMANDS: WorkbenchCommand[] = [
  {
    id: "goto-sessions",
    labelKey: "commandCenter.command.goToSessions",
    keywords: ["session", "sessions", "会话"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "sessions", sessionId: null, creatingSession: false }),
  },
  {
    id: "goto-projects",
    labelKey: "commandCenter.command.goToProjects",
    keywords: ["project", "projects", "workspace", "workspaces", "项目", "工作区"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "projects", projectId: undefined }),
  },
  {
    id: "goto-runs",
    labelKey: "commandCenter.command.goToRuns",
    keywords: ["runs", "mission control", "mission-control", "attention inbox", "任务控制台", "运行"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "runs", section: "attention", runId: undefined }),
  },
  {
    id: "goto-loops",
    labelKey: "commandCenter.command.goToLoops",
    keywords: ["loops", "loop engineering", "循环", "循环工程"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined }),
  },
  {
    id: "goto-schedules",
    labelKey: "commandCenter.command.goToSchedules",
    keywords: ["scheduled tasks", "schedule", "定时任务"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "runs", section: "schedules", scheduleId: undefined }),
  },
  {
    id: "goto-plan",
    labelKey: "commandCenter.command.goToPlan",
    keywords: ["plan", "board", "todo board", "work-board", "看板", "任务看板", "计划"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "plan", section: "board", viewId: undefined, workItemId: undefined }),
  },
  {
    id: "goto-goals",
    labelKey: "commandCenter.command.goToGoals",
    keywords: ["goals", "goal center", "目标", "目标中心"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "plan", section: "goals", goalId: undefined }),
  },
  {
    id: "goto-quality",
    labelKey: "commandCenter.command.goToQuality",
    keywords: ["quality", "evaluation", "evaluations", "agent evaluation", "评测", "agent 评测", "质量"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined }),
  },
];
