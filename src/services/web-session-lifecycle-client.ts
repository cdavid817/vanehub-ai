import type { Session } from "../types/agent";
import type { SessionLifecycleService } from "./session-lifecycle-service";
import { i18n } from "../i18n";
import { mockAgents } from "./mock-agent-data";
import { defaultSessionTitleFromPath } from "../lib/session-path";
import { snapshotSeat } from "./seat-presentation";
import { nowIso } from "./web-mock-clock";
import { createWebMockOperation } from "./web-operation-client";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { discoverWebSessionCodeIndex } from "./web-code-index-state";
import { listWebExpertRoles } from "./web-expert-role-client";
import {
  cancelWebActiveStream,
  deleteWebChatSubscribers,
  deleteWebSessionMessages,
} from "./web-chat-state";
import { deleteWebSessionChatConfig } from "./web-chat-config-state";
import { deleteWebRecoveryReports } from "./web-session-recovery-state";
import {
  inspectMockProject,
  joinSiblingPath,
  normalizeRemoteWorkspace,
  resolveProjectPath,
  upsertKnownProject,
  upsertKnownRemoteWorkspace,
  validateWorktreeName,
} from "./web-known-workspace-client";
import {
  createWebSeatId,
  emitWebSessionEvent,
  findWebSession,
  getWebActiveSessionId,
  getWebWorkflowState,
  listWebSessions,
  nextWebSessionSequence,
  prependWebSession,
  replaceWebSessions,
  setWebActiveSessionId,
  setWebWorkflowState,
  updateWebSession,
} from "./web-session-state";

export const webSessionLifecycleClient: SessionLifecycleService = {
  async createSession(input) {
    const agent = mockAgents.find((candidate) => candidate.id === input.agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${input.agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(input.interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${input.interactionMode}.`);
    }
    if (agent.availabilityState !== "available" && agent.availabilityState !== "unknown") {
      throw new Error(agent.unavailableReason ?? `${agent.displayName} is not available.`);
    }
    const remoteWorkspace = input.remoteWorkspace ? normalizeRemoteWorkspace(input.remoteWorkspace) : null;
    if (agent.id === "onepiece" && remoteWorkspace) {
      throw new Error("OnePiece supports local projects and local Git worktrees only.");
    }
    const sshConnection = input.remoteWorkspace?.sshConnectionId
      ? findWebSshConnection(input.remoteWorkspace.sshConnectionId)
      : null;
    if (input.remoteWorkspace?.sshConnectionId && !sshConnection) {
      throw new Error(`SSH connection not found: ${input.remoteWorkspace.sshConnectionId}`);
    }
    if (
      sshConnection &&
      remoteWorkspace &&
      (sshConnection.host !== remoteWorkspace.host ||
        sshConnection.port !== (remoteWorkspace.port ?? 22) ||
        sshConnection.user !== (remoteWorkspace.user ?? ""))
    ) {
      throw new Error(
        "SSH connection endpoint does not match the remote workspace snapshot.",
      );
    }
    if (remoteWorkspace && input.worktree?.enabled) {
      throw new Error("Remote workspace cannot use Git worktree");
    }
    const projectPath = remoteWorkspace ? null : resolveProjectPath(input);
    const inspection = projectPath ? inspectMockProject(projectPath) : null;
    if (inspection) {
      upsertKnownProject(inspection);
    }
    if (remoteWorkspace) {
      upsertKnownRemoteWorkspace(remoteWorkspace);
    }
    let effectiveFolder = remoteWorkspace?.uri ?? projectPath;
    let worktreePath: string | null = null;
    let worktreeName: string | null = null;
    let worktreeBranch: string | null = null;
    if (input.worktree?.enabled) {
      if (!inspection?.isGit) {
        throw new Error("Git worktree unavailable");
      }
      worktreeName = validateWorktreeName(input.worktree.name ?? "");
      worktreePath = joinSiblingPath(inspection.path, worktreeName);
      worktreeBranch = `vanehub/${worktreeName}`;
      effectiveFolder = worktreePath;
    }
    const timestamp = nowIso();
    const titleSource = remoteWorkspace?.displayName || effectiveFolder || "";
    const normalizedSeats = (input.seats?.length ? input.seats : [{ agentId: input.agentId, roleId: null }]).map(
      (seat) => ({
        ...snapshotSeat(seat, mockAgents, listWebExpertRoles()),
        seatId: createWebSeatId(),
        joinedAt: timestamp,
        leftAt: null,
      }),
    );
    const session: Session = {
      id: `web-session-${nextWebSessionSequence()}`,
      title: input.title?.trim() || defaultSessionTitleFromPath(titleSource) || i18n.t("createSession.sessionPlaceholder"),
      agentId: normalizedSeats[0]?.agentId ?? input.agentId,
      // Mirrors the native normalization: no seats means one seat built from the Agent.
      seats: normalizedSeats,
      interactionMode: input.interactionMode,
      // Whatever the caller asked for. Forcing `standard` here would let a page ship whose
      // temporary session quietly retains everything, and the mock is where that has to be
      // reachable in a test.
      personalizationMode: input.personalizationMode ?? "standard",
      lifecycleState: "idle",
      recoveryStatus: "clean",
      recoveryRevision: 0,
      stateRevision: 0,
      historyRevision: 0,
      activeExecutionRunId: null,
      folder: effectiveFolder,
      projectPath,
      worktreePath,
      worktreeName,
      worktreeBranch,
      remoteWorkspace,
      remoteSshConnectionId: sshConnection?.id ?? null,
      remoteSshConnectionRevision: sshConnection?.revision ?? null,
      runtimeSessionId: null,
      categoryId: null,
      pinned: false,
      archived: false,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    prependWebSession(session);
    setWebActiveSessionId(session.id);
    discoverWebSessionCodeIndex(session);
    emitWebSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    });
    return createWebMockOperation({
      id: `web-session-create-${session.id}-${Date.now()}`,
      kind: "workspace",
      relatedEntityId: remoteWorkspace?.uri ?? projectPath,
      message: `Created mock session ${session.id}`,
      terminalStatus: "succeeded",
      error: null,
      result: session as unknown as Record<string, unknown>,
    });
  },

  async deleteSession(sessionId: string) {
    findWebSession(sessionId);
    cancelWebActiveStream(sessionId);
    deleteWebSessionMessages(sessionId);
    deleteWebRecoveryReports(sessionId);
    deleteWebChatSubscribers(sessionId);
    deleteWebSessionChatConfig(sessionId);
    replaceWebSessions(listWebSessions().filter((session) => session.id !== sessionId));
    if (getWebActiveSessionId() === sessionId) {
      setWebActiveSessionId(null);
      emitWebSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
  },

  async switchSession(sessionId: string) {
    const session = findWebSession(sessionId);
    if (session.archived) {
      throw new Error(`Cannot switch to archived session: ${sessionId}`);
    }
    setWebActiveSessionId(session.id);
    emitWebSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    });
    return session;
  },

  async renameSession(sessionId: string, title: string) {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      throw new Error(i18n.t("web.error.sessionTitleRequired"));
    }
    return updateWebSession(sessionId, { title: trimmedTitle });
  },

  async pinSession(sessionId: string) {
    return updateWebSession(sessionId, { pinned: true });
  },

  async unpinSession(sessionId: string) {
    return updateWebSession(sessionId, { pinned: false });
  },

  async archiveSession(sessionId: string) {
    const cancelled = cancelWebActiveStream(sessionId);
    const session = updateWebSession(sessionId, { archived: true, ...(cancelled ? { lifecycleState: "stopped" } : {}) });
    if (getWebActiveSessionId() === sessionId) {
      setWebActiveSessionId(null);
      emitWebSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
    return session;
  },

  async unarchiveSession(sessionId: string) {
    return updateWebSession(sessionId, { archived: false });
  },
};
