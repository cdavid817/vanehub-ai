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

export function activeSeatsFromSession(session: Session): SessionSeat[] {
  return seatsFromSession(session).filter((seat) => seat.leftAt == null);
}

export function sessionAgentIdFromSeats(seats: SessionSeat[], fallback = ""): string {
  return seats.find((seat) => seat.leftAt == null)?.agentId ?? fallback;
}

/**
 * The handle each seat answers to, from the names its callers resolved.
 *
 * Mirrors `derive_mentions` (seat_roster.rs): whitespace becomes a hyphen, and a name already
 * taken gets a numeric suffix so an `@` addresses exactly one seat. The names are passed in rather
 * than read here because each caller resolves them from what it has -- a role snapshot, a loaded
 * role, an Agent -- while the disambiguation has to stay one implementation, or two seats can end
 * up sharing a handle in one surface and not the other.
 */
export function seatHandlesFromNames(names: string[]): string[] {
  const used = new Map<string, number>();
  return names.map((name, index) => {
    const base = name.split(/\s+/).filter(Boolean).join("-") || `席位${index + 1}`;
    const count = (used.get(base) ?? 0) + 1;
    used.set(base, count);
    return count === 1 ? base : `${base}-${count}`;
  });
}
