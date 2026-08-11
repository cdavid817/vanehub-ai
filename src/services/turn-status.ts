import type { TurnStatus } from "../components/chat/TurnStatusBar";

/** The turn status as the native layer publishes it. */
export type TurnStatusEvent =
  | { kind: "agent"; seatId?: string; seatIndex: number; mention: string; depth: number; maxDepth: number }
  | { kind: "waiting_human"; seatId?: string; seatIndex: number; mention: string; since: string }
  | { kind: "round_complete"; seatId?: string; seatIndex: number; mention: string };

/**
 * Counts how long a paused round has been waiting.
 *
 * Counted from the moment the pause began rather than accumulated natively, so the bar ticks
 * without the backend having to publish once a minute. Two clocks are involved, so a wait can
 * appear to start in the future; that reads as zero rather than as a negative number.
 */
export function waitedMinutes(since: string, now: Date): number {
  const started = new Date(since).getTime();
  if (Number.isNaN(started)) return 0;
  return Math.max(0, Math.floor((now.getTime() - started) / 60_000));
}

export function turnStatusFromEvent(event: TurnStatusEvent, now = new Date()): TurnStatus {
  if (event.kind === "waiting_human") {
    return {
      kind: "waiting-human",
      seatId: event.seatId,
      requesterName: event.mention,
      waitedMinutes: waitedMinutes(event.since, now),
    };
  }
  if (event.kind === "round_complete") {
    return { kind: "round-complete", seatId: event.seatId, finisherName: event.mention };
  }
  return {
    kind: "agent",
    seatId: event.seatId,
    holderName: event.mention,
    depth: event.depth,
    maxDepth: event.maxDepth,
  };
}
