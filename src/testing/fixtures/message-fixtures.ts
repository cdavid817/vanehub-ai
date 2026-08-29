import type { Session } from "../../types/agent";
import type { ChatMessage, MessageFeedback, MessageRole, MessageStatus } from "../../types/chat";
import {
  DEFAULT_SEED,
  type SeededRandom,
  chance,
  createIdFactory,
  createSeededRandom,
  nextInt,
  pickWeighted,
  words,
} from "./seeded-random";

interface MessageTier { readonly name: string; readonly weight: number; readonly min: number; readonly max: number }

/**
 * Most sessions are quiet; a handful ran long conversations. This is the long tail, not a flat
 * average -- the weights and ranges are chosen so the *expected* total lands close to
 * `totalCount` for a 0.7 chatty fraction (roughly 7 messages per chatty session at the default
 * 5,000-over-1,000 scale); the sweep below then closes the small remaining gap exactly.
 */
const CHATTY_FRACTION = 0.7;
const TIERS: readonly MessageTier[] = [
  { name: "epic", weight: 2, min: 60, max: 140 },
  { name: "long", weight: 6, min: 20, max: 45 },
  { name: "medium", weight: 25, min: 6, max: 14 },
  { name: "short", weight: 67, min: 1, max: 3 },
];

/**
 * Assigns a message count per session id. Sessions past `CHATTY_FRACTION` get zero (never
 * chatted); the rest draw a tier and a count within it. Counts are then nudged, one session at a
 * time and always within that session's own tier bounds, until the grand total is exactly
 * `totalCount` -- so "5,000 messages" stays exact without collapsing into a uniform per-session
 * average.
 */
function assignSessionMessageCounts(rng: SeededRandom, sessions: readonly Session[], totalCount: number): Map<string, number> {
  const chattyCount = Math.round(sessions.length * CHATTY_FRACTION);
  const chattySessions = sessions.slice(0, chattyCount);
  const tierByIndex: MessageTier[] = [];
  const counts = chattySessions.map(() => {
    const tier = pickWeighted(rng, TIERS.map((candidate) => [candidate, candidate.weight] as const));
    tierByIndex.push(tier);
    return nextInt(rng, tier.min, tier.max + 1);
  });

  let diff = totalCount - counts.reduce((sum, value) => sum + value, 0);
  // Deterministic full sweeps (see `distributeExact` in `seeded-random.ts` for the same pattern):
  // guaranteed to close `diff` to zero as long as it is feasible within each session's own tier
  // bounds, rather than hoping a bounded number of random probes happens to land on the right ones.
  let progressed = true;
  while (diff !== 0 && progressed && counts.length > 0) {
    progressed = false;
    for (let index = 0; index < counts.length && diff !== 0; index += 1) {
      const tier = tierByIndex[index];
      if (diff > 0 && counts[index] < tier.max) {
        counts[index] += 1;
        diff -= 1;
        progressed = true;
      } else if (diff < 0 && counts[index] > tier.min) {
        counts[index] -= 1;
        diff += 1;
        progressed = true;
      }
    }
  }

  const byId = new Map<string, number>();
  chattySessions.forEach((session, index) => byId.set(session.id, counts[index]));
  return byId;
}

const ROLE_CYCLE: readonly MessageRole[] = ["user", "assistant"];

function pickStatus(rng: SeededRandom, isTrailingInRunningSession: boolean): MessageStatus {
  if (isTrailingInRunningSession && chance(rng, 0.4)) {
    return pickWeighted(rng, [["streaming", 60], ["pending", 40]] as const);
  }
  return pickWeighted(rng, [["completed", 90], ["failed", 6], ["cancelled", 4]] as const);
}

function buildFeedback(rng: SeededRandom): MessageFeedback {
  return {
    state: pickWeighted(rng, [["helpful", 55], ["unhelpful", 20], ["corrected", 25]] as const),
    revision: nextInt(rng, 1, 4),
  };
}

/**
 * `totalCount` messages distributed unevenly across `sessions` (see `assignSessionMessageCounts`).
 * Every message's `sessionId` references a real entry in `sessions`, timestamps increase
 * monotonically within a session, and a small fraction of assistant replies are long enough to
 * stress a renderer.
 *
 * `executionRunId` is intentionally a self-contained synthetic id here, not a cross-reference into
 * a generated Mission Control run set -- keeping this generator's only input dependency on
 * `sessions` keeps every fixture module independently callable.
 */
export function generateMessages(sessions: readonly Session[], totalCount: number, seed: number = DEFAULT_SEED): ChatMessage[] {
  const rng = createSeededRandom(seed);
  const nextId = createIdFactory("message");
  const nextExecutionRunId = createIdFactory("message-exec-run");
  const countsBySession = assignSessionMessageCounts(rng, sessions, totalCount);
  const messages: ChatMessage[] = [];

  for (const session of sessions) {
    const count = countsBySession.get(session.id) ?? 0;
    if (count === 0) continue;
    let cursorMs = Date.parse(session.createdAt);

    for (let sequence = 1; sequence <= count; sequence += 1) {
      cursorMs += nextInt(rng, 30_000, 40 * 60_000);
      const createdAt = new Date(cursorMs).toISOString();
      const role: MessageRole = sequence % 9 === 0 ? "tool" : sequence % 11 === 0 ? "system" : ROLE_CYCLE[sequence % 2];
      const isTrailing = sequence === count && session.lifecycleState === "running";
      const status = pickStatus(rng, isTrailing);
      const content = role === "assistant"
        ? chance(rng, 0.04) ? words(rng, 300, 600) : words(rng, 8, 90)
        : words(rng, 2, 40);

      messages.push({
        id: nextId(),
        sessionId: session.id,
        content,
        role,
        status,
        thinkingContent: role === "assistant" && chance(rng, 0.1) ? words(rng, 20, 60) : undefined,
        tokenUsage: role === "assistant" && status === "completed"
          ? { input: nextInt(rng, 200, 6000), output: nextInt(rng, 50, 3000) }
          : undefined,
        error: status === "failed" ? "fixture_failure_reason" : undefined,
        createdAt,
        updatedAt: createdAt,
        sessionSequence: sequence,
        executionRunId: role === "assistant" && chance(rng, 0.3) ? nextExecutionRunId() : null,
        feedback: role === "assistant" && status === "completed" && chance(rng, 0.08) ? buildFeedback(rng) : undefined,
      });
    }
  }

  return messages;
}
