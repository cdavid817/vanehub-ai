import type {
  ExportSessionInput,
  InteractionMode,
  Session,
  SessionDetails,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
} from "../types/agent";
import type { SessionQueryService } from "./session-query-service";
import { mockAgents } from "./mock-agent-data";
import { nowIso } from "./web-mock-clock";
import { getWebSessionMessages } from "./web-chat-state";
import { isWebLoopRoleSession } from "./web-loop-state";
import { webRunnerDescriptors } from "./web-agent-runner";
import {
  findWebSession,
  getWebActiveSessionId,
  getWebWorkflowState,
  listWebSessions,
  setWebWorkflowState,
  sortWebSessions,
} from "./web-session-state";

function searchText(value: string | null | undefined, query: string) {
  return (value ?? "").toLocaleLowerCase().includes(query.toLocaleLowerCase());
}

function sessionSearchMatches(session: Session, query: string): SessionSearchResult | null {
  const matches: SessionSearchResult["matches"] = [];
  if (searchText(session.title, query)) {
    matches.push({ kind: "title", excerpt: session.title });
  }
  const remoteWorkspace = session.remoteWorkspace;
  const projectMatch = [
    session.folder,
    session.projectPath,
    session.worktreePath,
    session.worktreeName,
    session.worktreeBranch,
    remoteWorkspace?.host,
    remoteWorkspace?.user,
    remoteWorkspace?.path,
    remoteWorkspace?.displayName,
    remoteWorkspace?.uri,
  ].find((value) => searchText(value, query));
  if (projectMatch) {
    matches.push({ kind: "project", excerpt: projectMatch });
  }
  const messageMatch = getWebSessionMessages(session.id).find((message) => searchText(message.content, query));
  if (messageMatch) {
    matches.push({
      kind: "message",
      excerpt: messageMatch.content.slice(0, 160),
      messageId: messageMatch.id,
    });
  }
  return matches.length > 0 ? { session: { ...session }, matches } : null;
}

function serializeWebSessionExport(input: ExportSessionInput): SessionExportResult {
  const session = findWebSession(input.sessionId);
  const payload = {
    version: 1,
    exportedAt: nowIso(),
    session,
    messages: getWebSessionMessages(session.id),
  };
  const content =
    input.format === "json"
      ? JSON.stringify(payload, null, 2)
      : [`# ${session.title}`, "", `- ID: \`${session.id}\``, `- Agent: \`${session.agentId}\``, "", "## Messages", ""]
          .concat(
            payload.messages.flatMap((message) => [
              `### ${message.role.toUpperCase()} - \`${message.status}\``,
              "",
              message.content,
              "",
            ]),
          )
          .join("\n");
  return {
    status: input.destinationDirectory === null ? "cancelled" : "exported",
    path: input.destinationDirectory ? `${input.destinationDirectory}\\${session.id}.${input.format === "json" ? "json" : "md"}` : null,
    content,
  };
}

export const webSessionQueryClient: SessionQueryService = {
  async getWorkflowState() {
    return getWebWorkflowState();
  },

  async selectAgent(agentId: string, interactionMode: InteractionMode) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${interactionMode}.`);
    }
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: agentId,
      activeInteractionMode: interactionMode,
      lifecycleState: "idle",
    });
    return getWebWorkflowState();
  },

  async launchActiveWorkflow() {
    setWebWorkflowState({
      ...getWebWorkflowState(),
      lifecycleState: getWebWorkflowState().activeAgentId ? "running" : "failed",
    });
    return {
      workflow: getWebWorkflowState(),
      message: getWebWorkflowState().activeAgentId
        ? "Web preview session marked as running."
        : "Select an agent before launching.",
    };
  },

  async getSessionDetails(): Promise<SessionDetails> {
    const adapter = getWebWorkflowState().activeInteractionMode ?? "none";
    return {
      agentId: getWebWorkflowState().activeAgentId,
      interactionMode: getWebWorkflowState().activeInteractionMode,
      lifecycleState: getWebWorkflowState().lifecycleState,
      adapter,
      details: {
        runtime: "web",
        storage: "in-memory",
      },
    };
  },

  async listSessions() {
    return sortWebSessions(listWebSessions().filter((session) => !session.archived && !isWebLoopRoleSession(session.id)));
  },

  async listArchivedSessions() {
    return sortWebSessions(listWebSessions().filter((session) => session.archived && !isWebLoopRoleSession(session.id)));
  },

  async searchSessions(input: SessionSearchInput) {
    const query = input.query.trim();
    if (!query) return [];
    return sortWebSessions(listWebSessions().filter((session) => !isWebLoopRoleSession(session.id)))
      .map((session) => sessionSearchMatches(session, query))
      .filter((result): result is SessionSearchResult => result !== null)
      .slice(0, input.limit ?? 50);
  },

  async getSession(sessionId: string) {
    return findWebSession(sessionId);
  },

  async getActiveSession() {
    const sessionId = getWebActiveSessionId();
    if (!sessionId) return null;
    return listWebSessions().find((session) => session.id === sessionId) ?? null;
  },

  async exportSession(input: ExportSessionInput) {
    return serializeWebSessionExport(input);
  },

  async listAgentRunners(sessionId, agentId) {
    return structuredClone(webRunnerDescriptors(sessionId, agentId));
  },
};
