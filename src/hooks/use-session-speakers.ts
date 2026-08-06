import { useQuery } from "@tanstack/react-query";
import { useMemo } from "react";
import { agentService } from "../services/runtime-agent-client";
import { resolveMessageSpeaker, type MessageSpeaker } from "../services/message-speaker";
import { seatsFromSession } from "../services/session-seats";
import type { AgentRegistryEntry, Session, SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";

/**
 * Maps each seat index to how that seat is rendered in the thread.
 *
 * A one-seat session resolves to nothing on purpose: it has to render exactly as it did before
 * seats existed, and an avatar and role label would be a visible change for a user who never
 * enabled the feature.
 */
export function sessionSpeakers({
  agents,
  roles,
  seats,
}: {
  agents: AgentRegistryEntry[];
  roles: ExpertRole[];
  seats: SessionSeat[];
}): Map<number, MessageSpeaker> {
  const speakers = new Map<number, MessageSpeaker>();
  if (seats.length < 2) return speakers;
  seats.forEach((_, seatIndex) => {
    const speaker = resolveMessageSpeaker({ agents, roles, seatIndex, seats });
    if (speaker) speakers.set(seatIndex, speaker);
  });
  return speakers;
}

export function useSessionSpeakers(session: Session | null): Map<number, MessageSpeaker> {
  const seats = useMemo(() => (session ? seatsFromSession(session) : []), [session]);
  const isMultiSeat = seats.length > 1;
  // Neither query runs for a single-Agent session, which must not pay for a feature it never uses.
  const rolesQuery = useQuery({
    queryKey: ["expert-roles"],
    queryFn: () => agentService.listExpertRoles(),
    enabled: isMultiSeat,
  });
  const agentsQuery = useQuery({
    queryKey: ["agents"],
    queryFn: () => agentService.listAgents(),
    enabled: isMultiSeat,
  });

  return useMemo(
    () =>
      sessionSpeakers({
        agents: agentsQuery.data ?? [],
        roles: rolesQuery.data ?? [],
        seats,
      }),
    [agentsQuery.data, rolesQuery.data, seats],
  );
}
