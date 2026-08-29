import type { ScheduledTask, Session } from "../../types/agent";
import type { ChatMessage } from "../../types/chat";
import type { Goal } from "../../contracts/goal";
import type { EvaluationArena } from "../../types/evaluation";
import type { LoopRun } from "../../types/loop";
import type { MissionControlRunSummary } from "../../types/mission-control";
import type { WorkItem } from "../../types/work-board";
import { type EvaluationResultRow, generateEvaluationFixtures } from "./evaluation-fixtures";
import { generateGoals } from "./goal-fixtures";
import { generateLoopRuns } from "./loop-run-fixtures";
import { generateMessages } from "./message-fixtures";
import { generateMissionControlRuns } from "./mission-control-run-fixtures";
import { generateScheduledTasks } from "./scheduled-task-fixtures";
import { DEFAULT_SEED } from "./seeded-random";
import { generateSessions } from "./session-fixtures";
import { generateWorkItems } from "./work-item-fixtures";

/**
 * Deterministic large-scale fixture counts for task 0.9 of `redesign-unified-workbench-ui`.
 *
 * Every value is the literal number the OpenSpec task asks for; a later structural-performance
 * test should import `FIXTURE_COUNTS` instead of restating the numbers.
 */
export const FIXTURE_COUNTS = {
  sessions: 1000,
  messages: 5000,
  missionControlRuns: 1000,
  workItems: 1000,
  goals: 500,
  loopRuns: 200,
  scheduledTasks: 100,
  evaluationResultRows: 10_000,
} as const;

export interface LargeScaleFixtureSet {
  sessions: Session[];
  messages: ChatMessage[];
  missionControlRuns: MissionControlRunSummary[];
  workItems: WorkItem[];
  goals: Goal[];
  loopRuns: LoopRun[];
  scheduledTasks: ScheduledTask[];
  evaluationArenas: EvaluationArena[];
  evaluationResultRows: EvaluationResultRow[];
}

/**
 * Builds the full fixture set described by `FIXTURE_COUNTS` from one seed.
 *
 * Each domain generator advances the seed by a fixed offset rather than sharing one RNG instance,
 * so the domains stay independent: calling this with a smaller `FIXTURE_COUNTS.sessions` for a
 * lighter benchmark would not perturb the runs, goals, or evaluation rows generated after it.
 * Every domain generator is also directly importable on its own from its own module in this
 * directory, for a test that only needs one domain at a non-default scale.
 */
export function generateLargeScaleFixtures(seed: number = DEFAULT_SEED): LargeScaleFixtureSet {
  const sessions = generateSessions(FIXTURE_COUNTS.sessions, seed + 1);
  const sessionIds = sessions.map((session) => session.id);
  const messages = generateMessages(sessions, FIXTURE_COUNTS.messages, seed + 2);
  const missionControlRuns = generateMissionControlRuns(FIXTURE_COUNTS.missionControlRuns, sessionIds, seed + 3);
  const workItems = generateWorkItems(FIXTURE_COUNTS.workItems, sessionIds, seed + 4);
  const goals = generateGoals(FIXTURE_COUNTS.goals, seed + 5);
  const loopRuns = generateLoopRuns(FIXTURE_COUNTS.loopRuns, seed + 6);
  const scheduledTasks = generateScheduledTasks(FIXTURE_COUNTS.scheduledTasks, seed + 7);
  const { arenas, resultRows } = generateEvaluationFixtures(FIXTURE_COUNTS.evaluationResultRows, seed + 8);

  return {
    sessions,
    messages,
    missionControlRuns,
    workItems,
    goals,
    loopRuns,
    scheduledTasks,
    evaluationArenas: arenas,
    evaluationResultRows: resultRows,
  };
}
