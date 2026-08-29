import type { Goal, GoalLink, GoalStatus } from "../../contracts/goal";
import { goalLinkProgressStates, goalLinkTargets } from "../../contracts/goal";
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

const STATUS_WEIGHTS: ReadonlyArray<readonly [GoalStatus, number]> = [
  ["draft", 15], ["active", 45], ["achieved", 30], ["abandoned", 10],
];

/** `awaiting_acceptance` only ever appears as a derived value, never a stored one -- see `contracts/goal.ts`. */
function deriveStatus(rng: SeededRandom, status: GoalStatus): Goal["derivedStatus"] {
  if (status === "active") return chance(rng, 0.35) ? "awaiting_acceptance" : "active";
  return status;
}

function buildLink(rng: SeededRandom, index: number): GoalLink {
  return {
    targetKind: pick(rng, goalLinkTargets),
    targetId: `link-target-${index}`,
    progress: pick(rng, goalLinkProgressStates),
  };
}

/**
 * `count` deterministic goals. `counted`/`terminal`/`unresolvable` are always recomputed from the
 * goal's own `links` array (never independently randomised), matching the production invariant
 * documented on `Goal`: those three counts describe the links, they are not a separate fact.
 */
export function generateGoals(count: number, seed: number = DEFAULT_SEED): Goal[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("goal");
  const goals: Goal[] = [];

  for (let index = 0; index < count; index += 1) {
    const status = pickWeighted(rng, STATUS_WEIGHTS);
    const createdAt = isoTimestamp(rng, FIXTURE_RANGE_START_MS, FIXTURE_RANGE_END_MS);
    const updatedAt = offsetTimestamp(createdAt, 0, 1000 * 60 * 60 * 24 * 90, rng);
    const links = Array.from({ length: nextInt(rng, 0, 6) }, (_unused, linkIndex) => buildLink(rng, index * 10 + linkIndex));
    const counted = links.filter((link) => link.targetKind !== "session" && link.progress !== "unresolvable").length;
    const terminal = links.filter((link) => link.progress === "terminal").length;
    const unresolvable = links.filter((link) => link.progress === "unresolvable").length;

    goals.push({
      id: nextId(),
      title: maybeLong(rng, () => title(rng, 2, 7), () => title(rng, 220, 380)),
      description: words(rng, 10, 60),
      acceptanceNotes: words(rng, 5, 30),
      status,
      derivedStatus: deriveStatus(rng, status),
      projectPath: chance(rng, 0.6) ? `D:/workspace/${title(rng, 1, 2)}` : null,
      createdAt,
      updatedAt,
      counted,
      terminal,
      unresolvable,
      links,
    });
  }

  return goals;
}
