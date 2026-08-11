import type { AgentRegistryEntry, SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";

export interface MessageSpeaker {
  agentId: string;
  avatar: string;
  color: string;
  /** Null when the seat carries no role: the Agent alone identifies the speaker. */
  roleName: string | null;
  agentName: string;
  crossFamilyReviewer: boolean;
}

const fallbackAvatar = "🤖";
const fallbackColor = "#7A8899";

/**
 * Resolves the seat that spoke a message into something renderable.
 *
 * Returns null rather than a placeholder when there is no seat — messages predate seats, and a
 * single-seat session must keep rendering exactly as it did before. It also returns null for a seat
 * identity that no longer exists, because malformed legacy attribution must not break the thread.
 */
export function resolveMessageSpeaker({
  agents,
  roles,
  speakerSeatId,
  seatIndex,
  seats,
}: {
  agents: AgentRegistryEntry[];
  roles: ExpertRole[];
  speakerSeatId?: string;
  seatIndex: number | undefined;
  seats: SessionSeat[];
}): MessageSpeaker | null {
  const seat = speakerSeatId
    ? seats.find((candidate) => candidate.seatId === speakerSeatId)
    : seatIndex === undefined
      ? undefined
      : seats[seatIndex];
  if (!seat) return null;

  const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
  const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
  const snapshot = seat.roleSnapshot;
  return {
    agentId: seat.agentId,
    avatar: snapshot?.avatar ?? role?.avatar ?? fallbackAvatar,
    color: snapshot?.color ?? role?.color ?? fallbackColor,
    roleName: snapshot?.roleName ?? role?.displayName ?? null,
    agentName: snapshot?.agentName ?? agent?.displayName ?? seat.agentId,
    crossFamilyReviewer:
      snapshot?.crossFamilyReviewer ?? role?.reviewPolicy.requireDifferentFamily ?? false,
  };
}
