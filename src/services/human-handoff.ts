export type HumanHandoffIntent = "handoff" | "fyi" | "done";

export interface HumanHandoffEffect {
  turnHolder: "agents" | "human";
  roundComplete: boolean;
  /** Whether a waiting duration starts accumulating, so a stalled round is visible. */
  startsWaiting: boolean;
}

const userMention = "@用户";
const linePrefix = /^\s*(?:(?:>\s*)|(?:[-*+]\s+)|(?:\d+[.)]\s+))*/;

/**
 * Reads how an Agent handed back to the human.
 *
 * A bare `@用户` with no intent is treated as informational rather than blocking. Defaulting to
 * blocking would punish an Agent for mentioning the human at all, and it would learn to stop —
 * which is exactly the visibility loss the three intents exist to prevent.
 */
export function parseHumanHandoff(reply: string): HumanHandoffIntent | null {
  for (const line of reply.split("\n")) {
    const rest = line.replace(linePrefix, "");
    if (!rest.startsWith(userMention)) continue;
    const remainder = rest.slice(userMention.length).trim().toLowerCase();
    if (remainder.startsWith("handoff")) return "handoff";
    if (remainder.startsWith("done")) return "done";
    return "fyi";
  }
  return null;
}

/**
 * What each intent does to the round. Only `handoff` interrupts: that separation is the point, since
 * a single blocking "notify the human" action teaches Agents to avoid notifying.
 */
export function applyHumanHandoff(intent: HumanHandoffIntent): HumanHandoffEffect {
  switch (intent) {
    case "fyi":
      return { turnHolder: "agents", roundComplete: false, startsWaiting: false };
    case "handoff":
      return { turnHolder: "human", roundComplete: false, startsWaiting: true };
    case "done":
      return { turnHolder: "human", roundComplete: true, startsWaiting: false };
  }
}
