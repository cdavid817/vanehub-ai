import type { AgentService } from "./agent-service";
import { mockAgents, mockWorkflowState } from "./mock-agent-data";
import type { InteractionMode, Session, SessionDetails, WorkflowState } from "../types/agent";

let workflowState: WorkflowState = { ...mockWorkflowState };
let nextSessionId = 1;
let activeSessionId: string | null = null;
let sessions: Session[] = [];

function nowIso() {
  return new Date().toISOString();
}

function sortSessions(items: Session[]) {
  return [...items].sort((left, right) => {
    if (left.pinned !== right.pinned) return left.pinned ? -1 : 1;
    if (left.archived !== right.archived) return left.archived ? 1 : -1;
    return right.updatedAt.localeCompare(left.updatedAt);
  });
}

function findSession(sessionId: string) {
  const session = sessions.find((candidate) => candidate.id === sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  return session;
}

function updateSession(sessionId: string, updates: Partial<Session>) {
  const timestamp = nowIso();
  let updated: Session | null = null;
  sessions = sessions.map((session) => {
    if (session.id !== sessionId) return session;
    updated = { ...session, ...updates, updatedAt: timestamp };
    return updated;
  });
  if (!updated) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  return updated;
}

export const webAgentClient: AgentService = {
  async listAgents(capabilityTag) {
    return capabilityTag
      ? mockAgents.filter((agent) => agent.capabilityTags.includes(capabilityTag))
      : mockAgents;
  },

  async getAgentById(agentId) {
    return mockAgents.find((agent) => agent.id === agentId) ?? null;
  },

  async getWorkflowState() {
    return workflowState;
  },

  async selectAgent(agentId: string, interactionMode: InteractionMode) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${interactionMode}.`);
    }
    workflowState = {
      ...workflowState,
      activeAgentId: agentId,
      activeInteractionMode: interactionMode,
      lifecycleState: "idle",
    };
    return workflowState;
  },

  async checkBrowserReadiness(agentId: string) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    const supportsBrowser = agent?.supportedInteractionModes.includes("browser") ?? false;
    return {
      ready: supportsBrowser,
      reason: supportsBrowser ? undefined : "This agent does not support browser interaction mode.",
      requiresAuthentication: supportsBrowser,
    };
  },

  async launchActiveWorkflow() {
    workflowState = {
      ...workflowState,
      lifecycleState: workflowState.activeAgentId ? "running" : "failed",
    };
    return {
      workflow: workflowState,
      message: workflowState.activeAgentId
        ? "Web preview session marked as running."
        : "Select an agent before launching.",
    };
  },

  async getSessionDetails(): Promise<SessionDetails> {
    const adapter = workflowState.activeInteractionMode ?? "none";
    return {
      agentId: workflowState.activeAgentId,
      interactionMode: workflowState.activeInteractionMode,
      lifecycleState: workflowState.lifecycleState,
      adapter,
      details: {
        runtime: "web",
        storage: "in-memory",
      },
    };
  },

  async listSessions() {
    return sortSessions(sessions.filter((session) => !session.archived));
  },

  async listArchivedSessions() {
    return sortSessions(sessions.filter((session) => session.archived));
  },

  async getActiveSession() {
    if (!activeSessionId) return null;
    return sessions.find((session) => session.id === activeSessionId) ?? null;
  },

  async createSession(input) {
    const agent = mockAgents.find((candidate) => candidate.id === input.agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${input.agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(input.interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${input.interactionMode}.`);
    }
    const timestamp = nowIso();
    const session: Session = {
      id: `web-session-${nextSessionId}`,
      title: input.title?.trim() || "新会话",
      agentId: input.agentId,
      interactionMode: input.interactionMode,
      lifecycleState: "idle",
      folder: input.folder ?? null,
      pinned: false,
      archived: false,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    nextSessionId += 1;
    sessions = [session, ...sessions];
    activeSessionId = session.id;
    workflowState = {
      ...workflowState,
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    };
    return session;
  },

  async deleteSession(sessionId: string) {
    findSession(sessionId);
    sessions = sessions.filter((session) => session.id !== sessionId);
    if (activeSessionId === sessionId) {
      activeSessionId = null;
    }
  },

  async switchSession(sessionId: string) {
    const session = findSession(sessionId);
    if (session.archived) {
      throw new Error(`Cannot switch to archived session: ${sessionId}`);
    }
    activeSessionId = session.id;
    workflowState = {
      ...workflowState,
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    };
    return session;
  },

  async renameSession(sessionId: string, title: string) {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      throw new Error("Session title cannot be empty.");
    }
    return updateSession(sessionId, { title: trimmedTitle });
  },

  async pinSession(sessionId: string) {
    return updateSession(sessionId, { pinned: true });
  },

  async unpinSession(sessionId: string) {
    return updateSession(sessionId, { pinned: false });
  },

  async archiveSession(sessionId: string) {
    const session = updateSession(sessionId, { archived: true });
    if (activeSessionId === sessionId) {
      activeSessionId = null;
    }
    return session;
  },

  async unarchiveSession(sessionId: string) {
    return updateSession(sessionId, { archived: false });
  },
};
