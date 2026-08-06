import type { Session, SessionSeat } from "../types/agent";

/**
 * Seats were added after sessions shipped, so `Session.seats` is optional and `Session.agentId`
 * mirrors the first seat. That mirroring is what lets roughly 148 existing frontend readers of
 * `agentId` keep working without being rewritten.
 */
export function seatsFromSession(session: Session): SessionSeat[] {
  const seats = session.seats ?? [];
  if (seats.length > 0) return seats;
  return [{ agentId: session.agentId, roleId: null }];
}

export function sessionAgentIdFromSeats(seats: SessionSeat[], fallback = ""): string {
  return seats[0]?.agentId ?? fallback;
}
