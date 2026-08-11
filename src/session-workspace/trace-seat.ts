import type { SafeAttribute } from "../types/execution-observability";

export interface TraceSeat {
  seatId: string | null;
  seatIndex: number;
  /** Null when the span predates mention handles or the seat carried none. */
  mention: string | null;
}

/**
 * Reads which seat produced a trace span.
 *
 * The execution trace stays session-scoped because it shows a whole round including the handoffs
 * between seats; splitting it per seat would destroy the thing it exists to show. Seats are told
 * apart by colour instead, which needs this marker.
 */
export function traceSeat(attributes: Record<string, SafeAttribute>): TraceSeat | null {
  const raw = attributes["vanehub.seat.index"];
  if (raw === undefined) return null;
  const seatIndex = Number(raw);
  if (!Number.isInteger(seatIndex) || seatIndex < 0) return null;
  const mention = attributes["vanehub.seat.mention"];
  const seatId = attributes["vanehub.seat.id"];
  return {
    seatId: typeof seatId === "string" ? seatId : null,
    seatIndex,
    mention: typeof mention === "string" ? mention : null,
  };
}
