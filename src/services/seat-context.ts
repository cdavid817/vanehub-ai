export interface SeatTurn {
  speaker: string;
  content: string;
}

export interface SeatContext {
  /** `resume` means the Agent's own session already holds the history and nothing is injected. */
  mode: "resume" | "inject";
  text: string;
}

/**
 * Decides how a seat learns what happened before its turn.
 *
 * Resume first: when the seat's Agent has a provider session, its history is already there and
 * re-injecting it would pay for the same context twice. Otherwise the preceding turns are injected
 * as attributed text — this is also how a seat added mid-session catches up on work it never saw.
 *
 * When the budget is tight the *most recent* turns are kept: the newest exchange is what the seat is
 * being asked to act on, while the oldest is the most likely to be recoverable from the project
 * itself.
 */
export function buildSeatContext({
  maxChars,
  providerSessionId,
  turns,
}: {
  maxChars: number;
  providerSessionId: string | null;
  turns: SeatTurn[];
}): SeatContext {
  if (providerSessionId) return { mode: "resume", text: "" };

  const lines: string[] = [];
  let used = 0;
  for (const turn of [...turns].reverse()) {
    const line = `[${turn.speaker} 说] ${turn.content}`;
    const cost = lines.length === 0 ? line.length : line.length + 1;
    if (used + cost > maxChars) break;
    lines.unshift(line);
    used += cost;
  }
  return { mode: "inject", text: lines.join("\n") };
}
