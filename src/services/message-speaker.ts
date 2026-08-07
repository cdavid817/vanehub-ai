import type { AgentRegistryEntry, SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";

export interface MessageSpeaker {
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
 * index that no longer exists, because removing a seat mid-session leaves its messages behind and
 * they must not break the thread.
 */
export function resolveMessageSpeaker({
  agents,
  roles,
  seatIndex,
  seats,
}: {
  agents: AgentRegistryEntry[];
  roles: ExpertRole[];
  seatIndex: number | undefined;
  seats: SessionSeat[];
}): MessageSpeaker | null {
  if (seatIndex === undefined) return null;
  const seat = seats[seatIndex];
  if (!seat) return null;

  const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
  const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
  return {
    avatar: role?.avatar ?? fallbackAvatar,
    color: role?.color ?? fallbackColor,
    roleName: role?.displayName ?? null,
    agentName: agent?.displayName ?? seat.agentId,
    crossFamilyReviewer: role?.reviewPolicy.requireDifferentFamily ?? false,
  };
}
