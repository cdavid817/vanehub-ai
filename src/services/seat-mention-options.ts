import type { SeatMentionOption } from "../components/chat/SeatMentionCompletion";
import type { AgentRegistryEntry, Session } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { normalizeModelFamily } from "./model-family";
import { activeSeatsFromSession } from "./session-seats";

export function seatMentionOptions(
  session: Session | null,
  agents: AgentRegistryEntry[],
  roles: ExpertRole[],
): SeatMentionOption[] {
  if (!session) return [];
  const seats = activeSeatsFromSession(session);
  if (seats.length < 2) return [];
  const used = new Map<string, number>();
  return seats.map((seat, index) => {
    const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
    const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
    const roleName = seat.roleSnapshot?.roleName ?? role?.displayName ?? null;
    const raw = roleName ?? seat.roleSnapshot?.agentName ?? agent?.displayName ?? `席位${index + 1}`;
    const base = raw.split(/\s+/).filter(Boolean).join("-") || `席位${index + 1}`;
    const count = (used.get(base) ?? 0) + 1;
    used.set(base, count);
    return {
      mention: count === 1 ? base : `${base}-${count}`,
      roleName,
      agentName: seat.roleSnapshot?.agentName ?? agent?.displayName ?? seat.agentId,
      modelFamily: seat.roleSnapshot?.modelFamily ?? normalizeModelFamily({
        id: seat.agentId,
        provider: agent?.provider ?? "",
      }),
      avatar: seat.roleSnapshot?.avatar ?? role?.avatar ?? "🤖",
    };
  });
}
