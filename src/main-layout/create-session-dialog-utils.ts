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
import type { WorkspaceMode } from "./create-session-workspace-sections";
import type { SessionAgentMode } from "./session-agent-mode-selector";

const defaultCliOrder = ["codex-cli", "claude-code", "gemini-cli", "opencode"];
export const previousSessionAgentStorageKey = "vanehub.create-session.agent.v1";

export function isSessionAgentSelectable(agent: AgentRegistryEntry): boolean {
  if (agent.availabilityState === "available" || agent.availabilityState === "unknown") {
    return true;
  }
  const sdkId = agent.managedSdkDependencyId;
  return agent.availabilityState === "unavailable"
    && agent.supportedInteractionModes.includes("cli")
    && Boolean(sdkId)
    && agent.unavailableReason === `Managed SDK dependency '${sdkId}' is not installed.`;
}

export function selectSessionAgents(agents: AgentRegistryEntry[]): AgentRegistryEntry[] {
  return agents
    .filter((agent) =>
      agent.supportedInteractionModes.some((mode) => mode === "cli" || mode === "api"),
    )
    .sort((left, right) => {
      const leftCli = defaultCliOrder.indexOf(left.id);
      const rightCli = defaultCliOrder.indexOf(right.id);
      if (leftCli !== -1 || rightCli !== -1) {
        return (leftCli === -1 ? Number.MAX_SAFE_INTEGER : leftCli)
          - (rightCli === -1 ? Number.MAX_SAFE_INTEGER : rightCli);
      }
      if (left.id === "onepiece") return -1;
      if (right.id === "onepiece") return 1;
      return left.displayName.localeCompare(right.displayName);
    });
}

export function defaultSessionAgent(
  agents: AgentRegistryEntry[],
  previousAgentId: string | null,
): AgentRegistryEntry | null {
  if (previousAgentId === "onepiece") {
    const onepiece = agents.find((agent) =>
      agent.id === "onepiece"
      && isSessionAgentSelectable(agent),
    );
    if (onepiece) return onepiece;
  }
  return agents.find((agent) =>
    defaultCliOrder.includes(agent.id) && isSessionAgentSelectable(agent),
  ) ?? agents.find(isSessionAgentSelectable) ?? agents[0] ?? null;
}

export function groupSessionAgents(agents: AgentRegistryEntry[]) {
  return [
    {
      id: "builtin-cli" as const,
      labelKey: "onepiece.group.builtinCli",
      agents: agents.filter((agent) =>
        agent.id !== "onepiece"
        && agent.agentOrigin === "builtin"
        && agent.supportedInteractionModes.includes("cli"),
      ),
    },
    {
      id: "native" as const,
      labelKey: "onepiece.group.native",
      agents: agents.filter((agent) => agent.id === "onepiece"),
    },
    {
      id: "custom-api" as const,
      labelKey: "onepiece.group.customApi",
      agents: agents.filter((agent) =>
        agent.agentOrigin === "user" && agent.supportedInteractionModes.includes("api"),
      ),
    },
  ].filter((group) => group.agents.length > 0);
}

export const defaultSshConnectionDraft: SaveSshConnectionInput = {
  name: "",
  host: "",
  port: 22,
  user: "",
  defaultPath: "",
  authMode: "key",
  keyPath: "",
};

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

export function canCreateSession({
  agentMode,
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
  const port = Number(remotePort.trim() || "22");
  const remotePortValid = Number.isInteger(port) && port >= 1 && port <= 65535;
  const savedConnectionValid =
    !saveSshConnection ||
    sshConnectionSaveErrorKey(remoteUser, sshConnectionDraft) === null;
  // A multi-Agent session needs at least two seats, each bound to an Agent; otherwise it is just
  // a single-Agent session wearing the wrong mode.
  const seatsReady =
    agentMode === "single" ||
    (multiSeats.length >= 2 && multiSeats.every((seat) => seat.agentId.trim().length > 0));
  return Boolean(
    selectedAgent &&
    isSessionAgentSelectable(selectedAgent) &&
    !(selectedAgent.id === "onepiece" && workspaceMode === "remote") &&
    seatsReady &&
    (workspaceMode === "remote"
      ? remoteHost.trim() &&
        remotePath.trim() &&
        remotePortValid &&
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
