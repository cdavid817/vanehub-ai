import { agentService } from "../services/runtime-agent-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import type {
  AgentRegistryEntry,
  CreateSessionInput,
  InteractionMode,
  Session,
} from "../types/agent";
import type { OperationTask } from "../types/operation";
import type { SaveSshConnectionInput } from "../types/ssh-connection";
import type { WorkspaceMode } from "./create-session-workspace-sections";
import type { SessionAgentMode } from "./session-agent-mode-selector";

export const preferredAgentIds = [
  "claude-code",
  "gemini-cli",
  "codex-cli",
  "opencode",
];

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
  return Boolean(
    selectedAgent &&
    agentMode === "single" &&
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
  interactionMode,
  projectPath,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
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
  interactionMode: InteractionMode;
  projectPath: string;
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
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
  if (agentMode !== "single") return;
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
  const input: CreateSessionInput = {
    agentId: selectedAgent.id,
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
          }
        : null,
    worktree:
      workspaceMode === "local" && worktreeEnabled
        ? { enabled: true, name: worktreeName }
        : null,
  };

  try {
    if (workspaceMode === "remote" && saveSshConnection) {
      await sshConnectionService.createConnection({
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
