import type { WorkbenchCommand } from "./command-center-types";

/**
 * 6.7. One command per genuinely distinct deep-link target, not one per name a reader might use —
 * Board is Plan's own default section and Mission Control is Runs' own default section, so those
 * two live as searchable `keywords` on their parent's command (also satisfying design.md's "Command
 * Center indexes old keywords" migration note) rather than as separate, behaviorally-identical
 * commands. Goals/Loops/Schedules each route somewhere the parent's default does not, so each gets
 * its own entry.
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
    keywords: ["runs", "mission control", "attention inbox", "任务控制台", "运行"],
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
    keywords: ["plan", "board", "todo board", "看板", "计划"],
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
    keywords: ["quality", "evaluation", "agent evaluation", "评测", "质量"],
    isAvailable: () => true,
    run: (context) => context.navigate({ destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined }),
  },
];
