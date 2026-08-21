import type { SeatMentionOption } from "../components/chat/SeatMentionCompletion";
import type { AgentRegistryEntry, Session } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { normalizeModelFamily } from "./model-family";
import { activeSeatsFromSession, seatHandlesFromNames } from "./session-seats";

export function seatMentionOptions(
  session: Session | null,
  agents: AgentRegistryEntry[],
  roles: ExpertRole[],
): SeatMentionOption[] {
  if (!session) return [];
  const seats = activeSeatsFromSession(session);
  if (seats.length < 2) return [];
  const resolved = seats.map((seat, index) => {
    const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
    const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
    const roleName = seat.roleSnapshot?.roleName ?? role?.displayName ?? null;
    const raw = roleName ?? seat.roleSnapshot?.agentName ?? agent?.displayName ?? `席位${index + 1}`;
    return { agent, role, roleName, raw };
  });
  const handles = seatHandlesFromNames(resolved.map((entry) => entry.raw));
  return seats.map((seat, index) => {
    const { agent, role, roleName } = resolved[index];
    return {
      mention: handles[index],
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
