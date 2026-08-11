import type { AgentRegistryEntry, SessionSeat, SessionSeatRoleSnapshot } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { builtinExpertRoles } from "../config/builtin-expert-roles";
import { normalizeModelFamily } from "./model-family";

export function snapshotSeat(
  seat: SessionSeat,
  agents: AgentRegistryEntry[],
  roles: ExpertRole[],
): SessionSeat {
  if (seat.roleSnapshot) return seat;
  const agent = agents.find((candidate) => candidate.id === seat.agentId) ?? null;
  const role = roles.find((candidate) => candidate.id === seat.roleId) ?? null;
  const roleSnapshot: SessionSeatRoleSnapshot = {
    roleName: role?.displayName ?? null,
    avatar: role?.avatar ?? "🤖",
    color: role?.color ?? "#7A8899",
    responsibility: role?.responsibility ?? null,
    agentName: agent?.displayName ?? seat.agentId,
    modelFamily: normalizeModelFamily({ id: seat.agentId, provider: agent?.provider ?? "" }),
    crossFamilyReviewer: role?.reviewPolicy.requireDifferentFamily ?? false,
  };
  return { ...seat, roleSnapshot };
}

export function seatDisplayName(seat: SessionSeat): string {
  const builtInRole = builtinExpertRoles.find((role) => role.id === seat.roleId);
  return seat.roleSnapshot?.roleName ?? builtInRole?.displayName ?? seat.roleSnapshot?.agentName ?? seat.agentId;
}
