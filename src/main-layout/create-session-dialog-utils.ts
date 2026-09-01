import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type {
  SessionSeat,
  AgentRegistryEntry,
  CreateSessionInput,
  InteractionMode,
  Session,
} from "../types/agent";
import type { OperationTask } from "../types/operation";
import type { SaveSshConnectionInput } from "../types/ssh-connection";
import { isSessionAgentSelectable } from "./create-session-agents";
import { agentSupportsRemoteWorkspace } from "./create-session-draft-model";
import type { WorkspaceMode } from "./create-session-workspace-sections";
import type { SessionPersonalizationMode } from "../types/personalization";
import type { SessionAgentMode } from "./session-agent-mode-selector";

// Moved to create-session-draft-model.ts (the field's natural home, and the only direction that
// keeps this file and that one from importing each other); re-exported so existing importers of
// this module keep working unchanged.
export { defaultSshConnectionDraft } from "./create-session-draft-model";

export function firstMode(agent: AgentRegistryEntry | null): InteractionMode {
  return agent?.supportedInteractionModes[0] ?? "cli";
}

export function conciseError(error: unknown, t: (key: string) => string) {
  const message = error instanceof Error ? error.message : String(error);
  if (message.includes("Git")) return t("createSession.error.git");
  if (message.includes("Project")) return t("createSession.error.project");
  if (message.includes("Agent")) return t("createSession.error.agent");
  return t("createSession.error.command");
}

export function sessionResult(result: OperationTask["result"]): Session | null {
  if (!result || typeof result !== "object") return null;
  if (typeof result.id !== "string") return null;
  if (typeof result.agentId !== "string") return null;
  if (typeof result.interactionMode !== "string") return null;
  return result as unknown as Session;
}

export async function resolveCreatedSession(
  result: OperationTask["result"],
  loadSession: (sessionId: string) => Promise<Session> = (sessionId) =>
    agentService.getSession(sessionId),
): Promise<Session | null> {
  const operationSession = sessionResult(result);
  if (!operationSession) return null;
  // Workspace operations intentionally expose only a bounded result. Reload through the session
  // service so stable seat ids and captured role presentation reach React as one canonical shape.
  return loadSession(operationSession.id);
}

export function remotePortIsValid(remotePort: string): boolean {
  const port = Number(remotePort.trim() || "22");
  return Number.isInteger(port) && port >= 1 && port <= 65535;
}

export function canCreateSession({
  agentMode,
  availableAgents,
  multiSeats,
  projectPath,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
  selectedAgent,
  sshConnectionDraft,
  workspaceMode,
  worktreeEnabled,
  worktreeName,
}: {
  agentMode: SessionAgentMode;
  /**
   * Optional so this function keeps working for callers -- and its own pre-existing unit tests --
   * that only have seat id strings, not the full registry. Omitting it skips only the staleness
   * re-check below; every other rule still applies.
   */
  availableAgents?: AgentRegistryEntry[];
  multiSeats: SessionSeat[];
  projectPath: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
  selectedAgent: AgentRegistryEntry | null;
  sshConnectionDraft: SaveSshConnectionInput;
  workspaceMode: WorkspaceMode;
  worktreeEnabled: boolean;
  worktreeName: string;
}) {
  const savedConnectionValid =
    !saveSshConnection ||
    sshConnectionSaveErrorKey(remoteUser, sshConnectionDraft) === null;
  // A multi-Agent session needs at least two seats, each bound to an Agent that is still
  // selectable today -- a seat can go stale if its Agent's availability changes after it was
  // added -- otherwise it is just a single-Agent session wearing the wrong mode.
  const seatsReady =
    agentMode === "single" ||
    (multiSeats.length >= 2 &&
      multiSeats.every((seat) => seat.agentId.trim().length > 0) &&
      (!availableAgents ||
        multiSeats.every((seat) => {
          const seatAgent = availableAgents.find((candidate) => candidate.id === seat.agentId);
          return seatAgent != null && isSessionAgentSelectable(seatAgent);
        })));
  return Boolean(
    selectedAgent &&
    isSessionAgentSelectable(selectedAgent) &&
    (workspaceMode !== "remote" || agentSupportsRemoteWorkspace(selectedAgent)) &&
    seatsReady &&
    (workspaceMode === "remote"
      ? remoteHost.trim() &&
        remotePath.trim() &&
        remotePortIsValid(remotePort) &&
        savedConnectionValid
      : projectPath.trim() && (!worktreeEnabled || worktreeName.trim())),
  );
}

export async function submitCreateSession({
  agentMode,
  multiSeats,
  interactionMode,
  projectPath,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
  selectedSshConnectionId,
  selectedAgent,
  setCreateOperationId,
  setError,
  setHandledCreateOperationId,
  setLoading,
  sshConnectionDraft,
  title,
  t,
  personalizationMode,
  workspaceMode,
  worktreeEnabled,
  worktreeName,
}: {
  agentMode: SessionAgentMode;
  multiSeats: SessionSeat[];
  interactionMode: InteractionMode;
  projectPath: string;
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
  selectedSshConnectionId: string;
  selectedAgent: AgentRegistryEntry | null;
  setCreateOperationId: (value: string | null) => void;
  setError: (value: string | null) => void;
  setHandledCreateOperationId: (value: string | null) => void;
  setLoading: (value: boolean) => void;
  sshConnectionDraft: SaveSshConnectionInput;
  title: string;
  t: (key: string) => string;
  personalizationMode: SessionPersonalizationMode;
  workspaceMode: WorkspaceMode;
  worktreeEnabled: boolean;
  worktreeName: string;
}) {
  if (!selectedAgent) return;
  if (workspaceMode === "local" && !projectPath.trim()) return;
  if (workspaceMode === "remote" && (!remoteHost.trim() || !remotePath.trim()))
    return;
  const parsedRemotePort = Number(remotePort.trim() || "22");
  if (
    workspaceMode === "remote" &&
    (!Number.isInteger(parsedRemotePort) ||
      parsedRemotePort < 1 ||
      parsedRemotePort > 65535)
  )
    return;
  const saveErrorKey = saveSshConnection
    ? sshConnectionSaveErrorKey(remoteUser, sshConnectionDraft)
    : null;
  if (saveErrorKey) {
    setError(t(saveErrorKey));
    return;
  }

  setLoading(true);
  setError(null);
  // agentId mirrors seat 0 so every existing reader of the session's agent keeps working.
  const seats = agentMode === "multi" && multiSeats.length > 0 ? multiSeats : null;
  const input: CreateSessionInput = {
    agentId: seats ? seats[0].agentId : selectedAgent.id,
    ...(seats ? { seats } : {}),
    interactionMode,
    personalizationMode,
    title,
    projectPath: workspaceMode === "local" ? projectPath : null,
    folder: workspaceMode === "local" ? projectPath : null,
    remoteWorkspace:
      workspaceMode === "remote"
        ? {
            host: remoteHost,
            port: parsedRemotePort,
            user: remoteUser || null,
            path: remotePath,
            displayName: remoteDisplayName || null,
            sshConnectionId: selectedSshConnectionId || null,
          }
        : null,
    worktree:
      workspaceMode === "local" && worktreeEnabled
        ? { enabled: true, name: worktreeName }
        : null,
  };

  try {
    if (workspaceMode === "remote" && saveSshConnection) {
      const connection = await sshConnectionService.createConnection({
        ...sshConnectionDraft,
        name:
          sshConnectionDraft.name.trim() ||
          remoteDisplayName.trim() ||
          remoteHost.trim(),
        host: remoteHost,
        port: parsedRemotePort,
        user: remoteUser,
        defaultPath: remotePath,
      });
      if (input.remoteWorkspace) {
        input.remoteWorkspace.sshConnectionId = connection.id;
      }
    }
    const operation = await agentService.createSession(input);
    setCreateOperationId(operation.id);
    setHandledCreateOperationId(null);
  } catch (createError) {
    setError(conciseError(createError, t));
    setLoading(false);
  }
}

export function sshConnectionSaveErrorKey(
  remoteUser: string,
  draft: SaveSshConnectionInput,
): string | null {
  if (!remoteUser.trim()) return "sshConnections.validation.user";
  if (draft.authMode === "key" && !draft.keyPath?.trim()) {
    return "sshConnections.validation.keyPath";
  }
  if (draft.authMode === "password" && !draft.password?.trim()) {
    return "sshConnections.validation.password";
  }
  return null;
}
