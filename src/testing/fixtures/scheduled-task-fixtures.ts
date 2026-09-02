import type { ScheduledTask, ScheduledTaskFrequency, ScheduledTaskLatestStatus } from "../../types/agent";
import { managedCliAgentIds } from "../../types/agent";
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
  words,
} from "./seeded-random";

const STATUS_WEIGHTS: ReadonlyArray<readonly [ScheduledTaskLatestStatus, number]> = [
  ["succeeded", 45], ["never-run", 20], ["failed", 15], ["running", 10], ["skipped", 10],
];

function buildFrequency(rng: SeededRandom): ScheduledTaskFrequency {
  const kind = pick(rng, ["minutes", "hours", "daily", "weekly", "monthly"] as const);
  const timeOfDay = `${nextInt(rng, 0, 24).toString().padStart(2, "0")}:${nextInt(rng, 0, 60).toString().padStart(2, "0")}`;
  switch (kind) {
    case "minutes": return { kind, interval: pick(rng, [5, 15, 30, 45] as const) };
    case "hours": return { kind, interval: nextInt(rng, 1, 12) };
    case "daily": return { kind, timeOfDay };
    case "weekly": return { kind, weekday: nextInt(rng, 0, 7), timeOfDay };
    case "monthly": return { kind, dayOfMonth: nextInt(rng, 1, 29), timeOfDay };
  }
}

/**
 * `count` deterministic scheduled tasks covering every `ScheduledTaskLatestStatus` and every
 * `ScheduledTaskFrequency` variant, with `latestRunAt`/`latestError` kept consistent with
 * `latestStatus` (null unless the task has actually run / actually failed).
 */
export function generateScheduledTasks(count: number, seed: number = DEFAULT_SEED): ScheduledTask[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("scheduled-task");
  const agentIds = [...managedCliAgentIds] as const;
  const tasks: ScheduledTask[] = [];

  for (let index = 0; index < count; index += 1) {
    const latestStatus = pickWeighted(rng, STATUS_WEIGHTS);
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const updatedAt = offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 20, rng);
    const hasRun = latestStatus !== "never-run";

    tasks.push({
      id: nextId(),
      name: maybeLong(rng, () => title(rng, 2, 6), () => title(rng, 200, 320)),
      content: words(rng, 15, 60),
      agentId: pick(rng, agentIds),
      frequency: buildFrequency(rng),
      enabled: chance(rng, 0.85),
      nextRunAt: offsetTimestamp(updatedAt, 1000 * 60, 1000 * 60 * 60 * 24 * 14, rng),
      latestStatus,
      latestRunAt: hasRun ? offsetTimestamp(updatedAt, -1000 * 60 * 60 * 24, 0, rng) : null,
      latestRunSessionId: hasRun ? `session-for-task-${index}` : null,
      latestError: latestStatus === "failed" ? words(rng, 6, 20) : null,
      createdAt,
      updatedAt,
      version: 1,
    });
  }

  return tasks;
}
