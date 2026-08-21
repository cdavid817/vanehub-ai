import type { SessionSeat, UpdateSessionSeatsInput } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import type { SessionSeatService } from "./session-lifecycle-service";
import { seatHandlesFromNames } from "./session-seats";
import { routeUserMessage } from "./turn-routing";
import { nowIso } from "./web-mock-clock";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { createWebSeatId, findWebSession, updateWebSession } from "./web-session-state";

export const webSessionSeatClient: SessionSeatService = {
  async updateSessionSeats(input: UpdateSessionSeatsInput) {
    const session = findWebSession(input.sessionId);
    if (session.updatedAt !== input.expectedUpdatedAt) {
      throw new Error("validation error: Session participants changed since they were loaded.");
    }
    if (input.seats.length === 0) {
      throw new Error("validation error: A session must keep at least one active participant.");
    }
    const changedAt = nowIso();
    const historical = session.seats ?? [{
      seatId: `${session.id}:seat:0`,
      agentId: session.agentId,
      roleId: null,
      joinedAt: session.createdAt,
      leftAt: null,
    }];
    const retained = new Set<string>();
    const additions: SessionSeat[] = [];
    for (const requested of input.seats) {
      const existing = historical.find((seat) =>
        seat.leftAt == null && !retained.has(seat.seatId ?? "") &&
        ((Boolean(requested.seatId) && seat.seatId === requested.seatId &&
          seat.agentId === requested.agentId && seat.roleId === requested.roleId) ||
          (!requested.seatId && seat.agentId === requested.agentId && seat.roleId === requested.roleId)),
      );
      if (existing?.seatId) {
        retained.add(existing.seatId);
      } else {
        additions.push({
          ...requested,
          seatId: createWebSeatId(),
          joinedAt: changedAt,
          leftAt: null,
        });
      }
    }
    const seats = [
      ...historical.map((seat) =>
        seat.leftAt == null && !retained.has(seat.seatId ?? "")
          ? { ...seat, leftAt: changedAt }
          : seat,
      ),
      ...additions,
    ];
    const firstActive = seats.find((seat) => seat.leftAt == null);
    if (!firstActive) {
      throw new Error("validation error: A session must keep at least one active participant.");
    }
    return updateWebSession(input.sessionId, { seats, agentId: firstActive.agentId });
  },

  async rebindRemoteSessionSshConnection(
    sessionId: string,
    connectionId: string,
  ) {
    const session = findWebSession(sessionId);
    if (!session.remoteWorkspace) {
      throw new Error(
        "Only remote workspace sessions can bind an SSH connection.",
      );
    }
    const connection = findWebSshConnection(connectionId);
    if (!connection) {
      throw new Error(`SSH connection not found: ${connectionId}`);
    }
    if (
      connection.host !== session.remoteWorkspace.host ||
      connection.port !== (session.remoteWorkspace.port ?? 22) ||
      connection.user !== (session.remoteWorkspace.user ?? "")
    ) {
      throw new Error(
        "SSH connection endpoint does not match the remote workspace snapshot.",
      );
    }
    return updateWebSession(sessionId, {
      remoteSshConnectionId: connection.id,
      remoteSshConnectionRevision: connection.revision,
    });
  },
};

/**
 * Which seat answers a user message in mock mode.
 *
 * The same rule the native runtime applies (`route_user_message` in seat_turn.rs): a line-leading
 * `@handle` addresses that seat, an unaddressed message continues with whoever last spoke, and a
 * thread nobody has spoken in goes to the first seat. Mock mode answered as the first seat every
 * time, which made the browser build disagree with the desktop one about who was even talking.
 *
 * `undefined` for a single-seat session: there is no group to attribute within, which is what
 * every session that never seated a second Agent looks like.
 */
export function webRoutedSeatId(
  activeSeats: SessionSeat[],
  messages: ChatMessage[],
  text: string,
): string | undefined {
  if (activeSeats.length < 2) return undefined;
  const handles = seatHandlesFromNames(activeSeats.map((seat, index) =>
    seat.roleSnapshot?.roleName ?? seat.roleSnapshot?.agentName ?? seat.agentId ?? `席位${index + 1}`));
  const lastSpoken = [...messages].reverse().find((message) => message.speakerSeatId)?.speakerSeatId;
  const lastHolderIndex = activeSeats.findIndex((seat) => seat.seatId === lastSpoken);
  const routed = routeUserMessage({
    text,
    mentions: handles,
    lastHolder: lastHolderIndex >= 0 ? handles[lastHolderIndex] : null,
    firstSeat: handles[0],
  });
  return activeSeats[handles.indexOf(routed)]?.seatId ?? activeSeats[0]?.seatId;
}
