import type { AgentRegistryEntry } from "../types/agent";
import { isSessionAgentSelectable } from "./create-session-agents";
import {
  canCreateSession,
  remotePortIsValid,
  sshConnectionSaveErrorKey,
} from "./create-session-dialog-utils";
import { agentSupportsRemoteWorkspace } from "./create-session-draft-model";
import type { CreateSessionDraft } from "./create-session-draft-model";

/**
 * Per-field validation reasons for the create-session draft (task 11.1's granularity
 * requirement). These are presentation-independent codes, not i18n keys or display text -- a
 * future task (11.10, not this one) maps a code to where and how it is shown. `sshConnection`
 * stays an exception: it reuses `sshConnectionSaveErrorKey`'s pre-existing i18n-key convention,
 * already relied on directly by `create-session-remote-workspace-section.tsx`, rather than
 * inventing a second parallel code for the same check.
 */
export type CreateSessionValidationReason =
  | "agent-unselectable"
  | "agent-remote-unsupported"
  | "workspace-path-missing"
  | "workspace-worktree-name-missing"
  | "workspace-remote-host-missing"
  | "workspace-remote-path-missing"
  | "workspace-remote-port-invalid"
  | "seats-too-few"
  | "seats-missing-agent"
  | "seats-agent-unselectable";

export interface CreateSessionValidation {
  /** Single source of truth for whether Create may be pressed; equals `canCreateSession(...)`. */
  canSubmit: boolean;
  agent: CreateSessionValidationReason | null;
  workspace: CreateSessionValidationReason | null;
  seats: CreateSessionValidationReason | null;
  sshConnection: string | null;
}

export function validateCreateSessionDraft(
  draft: CreateSessionDraft,
  selectedAgent: AgentRegistryEntry | null,
  availableAgents: AgentRegistryEntry[],
): CreateSessionValidation {
  return {
    canSubmit: canCreateSession({
      agentMode: draft.agentMode,
      availableAgents,
      multiSeats: draft.multiSeats,
      projectPath: draft.projectPath,
      remoteHost: draft.remoteHost,
      remotePath: draft.remotePath,
      remotePort: draft.remotePort,
      remoteUser: draft.remoteUser,
      saveSshConnection: draft.saveSshConnection,
      selectedAgent,
      sshConnectionDraft: draft.sshConnectionDraft,
      workspaceMode: draft.workspaceMode,
      worktreeEnabled: draft.worktreeEnabled,
      worktreeName: draft.worktreeName,
    }),
    agent: validateAgentField(draft, selectedAgent),
    workspace: validateWorkspaceField(draft),
    seats: validateSeatsField(draft, availableAgents),
    sshConnection: draft.saveSshConnection
      ? sshConnectionSaveErrorKey(draft.remoteUser, draft.sshConnectionDraft)
      : null,
  };
}

function validateAgentField(
  draft: CreateSessionDraft,
  selectedAgent: AgentRegistryEntry | null,
): CreateSessionValidationReason | null {
  if (!selectedAgent || !isSessionAgentSelectable(selectedAgent)) return "agent-unselectable";
  if (draft.workspaceMode === "remote" && !agentSupportsRemoteWorkspace(selectedAgent)) {
    return "agent-remote-unsupported";
  }
  return null;
}

function validateWorkspaceField(draft: CreateSessionDraft): CreateSessionValidationReason | null {
  if (draft.workspaceMode === "remote") {
    if (!draft.remoteHost.trim()) return "workspace-remote-host-missing";
    if (!draft.remotePath.trim()) return "workspace-remote-path-missing";
    if (!remotePortIsValid(draft.remotePort)) return "workspace-remote-port-invalid";
    return null;
  }
  if (!draft.projectPath.trim()) return "workspace-path-missing";
  if (draft.worktreeEnabled && !draft.worktreeName.trim()) return "workspace-worktree-name-missing";
  return null;
}

/**
 * Closes task 11.2's other named gap: `agentMode` (single vs. multi seat) had no compatibility
 * gate at all -- any Agent could be added to a seat regardless of whether it was still available.
 * No backend rule restricts *which* Agents may hold a seat (grepped
 * `src-tauri/src/contexts/sessions` for a seat-agent restriction and found none: seats are
 * accepted as given), so there is nothing to encode there. What genuinely was missing is
 * re-checked here: `canCreateSession`'s multi-seat readiness only checked for a non-empty agent
 * id string, never that the id still resolved to a selectable Agent -- unlike the single-Agent
 * path, which already called `isSessionAgentSelectable`. A seat added while its Agent was
 * available, left stale after that Agent's availability changed, now blocks submission the same
 * way the single-Agent path already did.
 */
function validateSeatsField(
  draft: CreateSessionDraft,
  availableAgents: AgentRegistryEntry[],
): CreateSessionValidationReason | null {
  if (draft.agentMode === "single") return null;
  if (draft.multiSeats.length < 2) return "seats-too-few";
  if (draft.multiSeats.some((seat) => seat.agentId.trim().length === 0)) {
    return "seats-missing-agent";
  }
  const allSelectable = draft.multiSeats.every((seat) => {
    const agent = availableAgents.find((candidate) => candidate.id === seat.agentId);
    return agent != null && isSessionAgentSelectable(agent);
  });
  return allSelectable ? null : "seats-agent-unselectable";
}
