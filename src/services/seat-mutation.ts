import type { SessionSeat } from "../types/agent";

export interface SeatMutation {
  seats: SessionSeat[];
  /** Kept in step with seat 0, because existing readers of the session still use it. */
  agentId: string;
}

/**
 * Seats change while a session runs, because a collaboration path emerges during the work. Adding
 * one is how a session that started with an architect and an implementer gains a reviewer, and a
 * one-seat session grows into a multi-seat one this way rather than being recreated.
 */
export function addSeat(seats: SessionSeat[], seat: SessionSeat): SeatMutation {
  const next = [...seats, seat];
  return { seats: next, agentId: next[0].agentId };
}

/**
 * Returns null rather than an empty session: a session always has someone in it, so removing the
 * last seat is refused rather than silently emptying it.
 *
 * Messages the removed seat already spoke stay in the thread — removing a participant is not a
 * reason to rewrite history — which is why the renderer tolerates a seat index that no longer
 * resolves.
 */
export function removeSeat(seats: SessionSeat[], index: number): SeatMutation | null {
  if (index < 0 || index >= seats.length) return null;
  if (seats.length <= 1) return null;
  const next = seats.filter((_, position) => position !== index);
  return { seats: next, agentId: next[0].agentId };
}
