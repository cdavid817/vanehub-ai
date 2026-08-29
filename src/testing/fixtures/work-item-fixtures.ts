import type { WorkItem, WorkItemSourceLink, WorkItemStage } from "../../types/work-board";
import { workItemPriorities } from "../../types/work-board";
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

const STAGE_WEIGHTS: ReadonlyArray<readonly [WorkItemStage, number]> = [
  ["inbox", 25], ["planned", 25], ["in_progress", 20], ["review", 15], ["done", 15],
];

function buildSources(rng: SeededRandom, sessionIds: readonly string[], index: number): WorkItemSourceLink[] {
  const sourceCount = nextInt(rng, 0, 4);
  return Array.from({ length: sourceCount }, (_unused, sourceIndex) => {
    const useSession = sessionIds.length > 0 && chance(rng, 0.7);
    return {
      sourceKind: useSession ? "session" : "scheduled_task",
      sourceId: useSession ? pick(rng, sessionIds) : `scheduled-task-${(index + sourceIndex) % 100}`,
      relation: pick(rng, ["primary", "execution", "automation", "supporting"] as const),
      title: title(rng, 2, 6),
      status: pick(rng, ["open", "active", "completed", "unavailable"] as const),
      available: chance(rng, 0.85),
      projectPath: chance(rng, 0.5) ? `D:/workspace/${title(rng, 1, 2)}` : null,
      updatedAt: chance(rng, 0.8) ? isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS) : null,
    } satisfies WorkItemSourceLink;
  });
}

/**
 * `count` deterministic work items spread across every board stage/priority, each carrying zero
 * to three source links (a mix of session- and scheduled-task-originated), with a small fraction
 * of long titles/paths for truncation stress tests.
 */
export function generateWorkItems(count: number, sessionIds: readonly string[], seed: number = DEFAULT_SEED): WorkItem[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("work-item");
  const stageRank = new Map<WorkItemStage, number>();
  const items: WorkItem[] = [];

  for (let index = 0; index < count; index += 1) {
    const stage = pickWeighted(rng, STAGE_WEIGHTS);
    const rank = stageRank.get(stage) ?? 0;
    stageRank.set(stage, rank + 1);
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const updatedAt = offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 60, rng);

    items.push({
      id: nextId(),
      title: maybeLong(rng, () => title(rng, 2, 7), () => title(rng, 220, 380)),
      description: words(rng, 10, 80),
      stage,
      priority: pick(rng, workItemPriorities),
      rank,
      projectPath: chance(rng, 0.7)
        ? maybeLong(rng, () => `D:/workspace/${title(rng, 1, 2)}`, () => `D:/workspace/${words(rng, 16, 24).replace(/ /g, "/")}`)
        : null,
      dueAt: chance(rng, 0.4) ? offsetTimestamp(updatedAt, 0, 1000 * 60 * 60 * 24 * 30, rng) : null,
      archived: chance(rng, 0.12),
      createdAt,
      updatedAt,
      sources: buildSources(rng, sessionIds, index),
    });
  }

  return items;
}
