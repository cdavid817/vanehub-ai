import { parseHandoffMentions, type HandoffTruncationReason } from "./mention-routing";

export type ChainEndReason = HandoffTruncationReason | "max-depth";

export interface NextTurn {
  targets: string[];
  /**
   * Why the chain stopped short, or null when it simply ran out of mentions. Null is the normal
   * ending: a reply that names nobody has finished the round, not failed.
   */
  endedReason: ChainEndReason | null;
}

/**
 * Where a user message goes. An unaddressed message continues with whoever last held the turn,
 * which matches how a person replies in a group without naming anyone.
 */
export function routeUserMessage({
  firstSeat,
  lastHolder,
  mentions,
  text,
}: {
  firstSeat: string;
  lastHolder: string | null;
  mentions: string[];
  text: string;
}): string {
  const { targets } = parseHandoffMentions({ text, mentions, selfMention: null, maxMentions: 1 });
  return targets[0] ?? lastHolder ?? firstSeat;
}

/**
 * Which seats a completed reply hands off to.
 *
 * The depth limit exists because agents mention each other autonomously; without it a pair can
 * ping-pong indefinitely. When it fires the reason is surfaced rather than the chain just stopping,
 * so a user is not left wondering why nobody answered.
 */
export function nextTurnTargets({
  depth,
  maxDepth,
  maxMentions,
  mentions,
  reply,
  speaker,
}: {
  depth: number;
  maxDepth: number;
  maxMentions: number;
  mentions: string[];
  reply: string;
  speaker: string;
}): NextTurn {
  if (depth >= maxDepth) return { targets: [], endedReason: "max-depth" };
  const { targets, truncatedReason } = parseHandoffMentions({
    text: reply,
    mentions,
    selfMention: speaker,
    maxMentions,
  });
  return { targets, endedReason: truncatedReason };
}
