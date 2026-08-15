import { activeSeatsFromSession } from "../session-seats";
import type { Session } from "../../types/agent";

/** `agentId === "onepiece"` is the codebase's established test for the built-in native agent. */
export function isOnePieceSession(session: Session): boolean {
  return session.agentId === "onepiece";
}

export function isMultiSeatCliSession(session: Session): boolean {
  return session.interactionMode === "cli" && activeSeatsFromSession(session).length > 1;
}

/**
 * The single gate. Phase 1 ships OnePiece only; widening to multi-seat CLI sessions later is a
 * change to this one expression plus each command's own `appliesTo`, and touches nothing else.
 */
export function slashCommandsEnabled(session: Session | null): boolean {
  if (!session) return false;
  return isOnePieceSession(session);
}
