import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import { operationService } from "../services/runtime-operation-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import { CreateSessionDialogContent } from "./create-session-dialog-content";
import {
  canCreateSession,
  conciseError,
  defaultSshConnectionDraft,
  firstMode,
  preferredAgentIds,
  sessionResult,
  submitCreateSession,
} from "./create-session-dialog-utils";
import type { WorkspaceMode } from "./create-session-workspace-sections";
import type { SessionAgentMode } from "./session-agent-mode-selector";
import type {
  AgentRegistryEntry,
  InteractionMode,
  KnownRemoteWorkspace,
  KnownProject,
  ProjectInspection,
  Session,
} from "../types/agent";
import type {
  SaveSshConnectionInput,
  SshConnection,
} from "../types/ssh-connection";
import {
  defaultSessionTitleFromPath,
  normalizeDisplayPath,
} from "../lib/session-path";
export function CreateSessionDialog({
  agents,
  onClose,
  onCreated,
  open,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  onCreated: (session: Session) => void;
  open: boolean;
}) {
  const { t } = useTranslation();
  const availableAgents = useMemo(
    () =>
      preferredAgentIds
        .map((agentId) => agents.find((agent) => agent.id === agentId))
        .filter((agent): agent is AgentRegistryEntry => Boolean(agent)),
    [agents],
  );
  const [agentId, setAgentId] = useState("");
  const selectedAgent =
    availableAgents.find((agent) => agent.id === agentId) ??
    availableAgents[0] ??
    null;
  const [interactionMode, setInteractionMode] =
    useState<InteractionMode>("cli");
  const [agentMode, setAgentMode] = useState<SessionAgentMode>("single");
  const [title, setTitle] = useState("");
  const [titleUserEdited, setTitleUserEdited] = useState(false);
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("local");
  const [projectPath, setProjectPath] = useState("");
  const [knownProjects, setKnownProjects] = useState<KnownProject[]>([]);
  const [knownRemoteWorkspaces, setKnownRemoteWorkspaces] = useState<KnownRemoteWorkspace[]>([]);
  const [sshConnections, setSshConnections] = useState<SshConnection[]>([]);
  const [selectedSshConnectionId, setSelectedSshConnectionId] = useState("");
  const [saveSshConnection, setSaveSshConnection] = useState(false);
  const [sshConnectionDraft, setSshConnectionDraft] = useState<SaveSshConnectionInput>(defaultSshConnectionDraft);
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);
  const [worktreeEnabled, setWorktreeEnabled] = useState(false);
  const [worktreeName, setWorktreeName] = useState("");
  const [remoteHost, setRemoteHost] = useState("");
  const [remotePort, setRemotePort] = useState("22");
  const [remoteUser, setRemoteUser] = useState("");
  const [remotePath, setRemotePath] = useState("");
  const [remoteDisplayName, setRemoteDisplayName] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [createOperationId, setCreateOperationId] = useState<string | null>(null);
  const [handledCreateOperationId, setHandledCreateOperationId] = useState<string | null>(null);
  useEffect(() => {
    if (!open) return;
    const agent = availableAgents[0] ?? null;
    setAgentId(agent?.id ?? "");
    setInteractionMode(firstMode(agent));
    setAgentMode("single");
    setTitle("");
    setTitleUserEdited(false);
    setWorkspaceMode("local");
    setProjectPath("");
    setRemoteHost("");
    setRemotePort("22");
    setRemoteUser("");
    setRemotePath("");
    setRemoteDisplayName("");
    setSelectedSshConnectionId("");
    setSaveSshConnection(false);
    setSshConnectionDraft(defaultSshConnectionDraft);
    setError(null);
    void agentService
      .listKnownProjects()
      .then(setKnownProjects)
      .catch(() => setKnownProjects([]));
    void agentService
      .listKnownRemoteWorkspaces()
      .then(setKnownRemoteWorkspaces)
      .catch(() => setKnownRemoteWorkspaces([]));
    void sshConnectionService
      .listConnections()
      .then(setSshConnections)
      .catch(() => setSshConnections([]));
  }, [availableAgents, open]);

  useEffect(() => {
    if (!createOperationId || handledCreateOperationId === createOperationId)
      return;
    const operationId = createOperationId;
    let cancelled = false;
    let timer: number | undefined;

    async function pollOperation() {
      try {
        const operation =
          await operationService.getOperationStatus(operationId);
        if (cancelled) return;
        if (operation.status === "queued" || operation.status === "running") {
          timer = window.setTimeout(() => void pollOperation(), 600);
          return;
        }
        setHandledCreateOperationId(operation.id);
        setLoading(false);
        if (operation.status === "failed") {
          setError(operation.error ?? t("createSession.error.command"));
          return;
        }
        const session = sessionResult(operation.result);
        if (!session) {
          setError(t("createSession.error.command"));
          return;
        }
        onCreated(session);
      } catch (operationError) {
        if (!cancelled) {
          setLoading(false);
          setError(conciseError(operationError, t));
        }
      }
    }

    void pollOperation();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [createOperationId, handledCreateOperationId, onCreated, t]);
  useEffect(() => {
    if (!selectedAgent) return;
    if (!selectedAgent.supportedInteractionModes.includes(interactionMode)) {
      setInteractionMode(firstMode(selectedAgent));
    }
  }, [interactionMode, selectedAgent]);
  useEffect(() => {
    if (titleUserEdited) return;
    const source =
      workspaceMode === "local" ? projectPath : remoteDisplayName || remotePath;
    const nextTitle = defaultSessionTitleFromPath(source);
    setTitle(nextTitle);
  }, [
    projectPath,
    remoteDisplayName,
    remotePath,
    titleUserEdited,
    workspaceMode,
  ]);
  async function inspectPath(path: string) {
    const trimmed = normalizeDisplayPath(path.trim());
    setProjectPath(trimmed);
    setWorktreeEnabled(false);
    setWorktreeName("");
    setInspection(null);
    setError(null);
    if (!trimmed) return;
    try {
      setInspection(await agentService.inspectProject(trimmed));
    } catch (inspectionError) {
      setError(conciseError(inspectionError, t));
    }
  }
  async function browseProject() {
    setError(null);
    try {
      const selectedPath = await agentService.selectProjectDirectory();
      if (selectedPath) {
        await inspectPath(selectedPath);
      }
    } catch (browseError) {
      setError(conciseError(browseError, t));
    }
  }

  if (!open) return null;
  const gitCapable = inspection?.isGit ?? false;
  const canSubmit = canCreateSession({
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
  });
  return (
    <CreateSessionDialogContent
      agentMode={agentMode}
      availableAgents={availableAgents}
      canSubmit={canSubmit}
      error={error}
      gitCapable={gitCapable}
      inspection={inspection}
      knownProjects={knownProjects}
      knownRemoteWorkspaces={knownRemoteWorkspaces}
      loading={loading}
      onAgentModeChange={setAgentMode}
      onAgentSelect={(agent) => {
        setAgentId(agent.id);
        setInteractionMode(firstMode(agent));
      }}
      onBrowseProject={() => void browseProject()}
      onClose={onClose}
      onInspectPath={(path) => void inspectPath(path)}
      onSubmit={() =>
        void submitCreateSession({
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
        })
      }
      onTitleChange={(value) => {
        setTitleUserEdited(true);
        setTitle(value);
      }}
      onWorkspaceModeChange={(mode) => {
        setWorkspaceMode(mode);
        setWorktreeEnabled(false);
        setError(null);
      }}
      projectPath={projectPath}
      remoteDisplayName={remoteDisplayName}
      remoteHost={remoteHost}
      remotePath={remotePath}
      remotePort={remotePort}
      remoteUser={remoteUser}
      saveSshConnection={saveSshConnection}
      selectedAgent={selectedAgent}
      selectedSshConnectionId={selectedSshConnectionId}
      setProjectPath={setProjectPath}
      setRemoteDisplayName={setRemoteDisplayName}
      setRemoteHost={setRemoteHost}
      setRemotePath={setRemotePath}
      setRemotePort={setRemotePort}
      setRemoteUser={setRemoteUser}
      setSaveSshConnection={setSaveSshConnection}
      setSelectedSshConnectionId={setSelectedSshConnectionId}
      setSshConnectionDraft={setSshConnectionDraft}
      setWorktreeEnabled={setWorktreeEnabled}
      setWorktreeName={setWorktreeName}
      sshConnectionDraft={sshConnectionDraft}
      sshConnections={sshConnections}
      title={title}
      workspaceMode={workspaceMode}
      worktreeEnabled={worktreeEnabled}
      worktreeName={worktreeName}
    />
  );
}
