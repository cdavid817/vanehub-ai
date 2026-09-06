import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { snapshotSeat } from "../services/seat-presentation";
import { CreateSessionDialogContent } from "./create-session-dialog-content";
import { canCreateSession, defaultSshConnectionDraft, firstMode, submitCreateSession } from "./create-session-dialog-utils";
import { defaultSessionAgent, previousSessionAgentStorageKey, selectSessionAgents } from "./create-session-agents";
import { useCreateSessionOperation } from "./use-create-session-operation";
import { useProjectInspection } from "./use-project-inspection";
import { useStoredSessionAgent } from "./use-stored-session-agent";
import { useCreateSessionReferenceData } from "./use-create-session-reference-data";
import type { WorkspaceMode } from "./create-session-workspace-sections";
import { modeForWorkspace } from "./session-personalization-mode-selector";
import type { SessionPersonalizationMode } from "../types/personalization";
import type { SessionAgentMode } from "./session-agent-mode-selector";
import type { SessionSeat } from "../types/agent";
import type { AgentRegistryEntry, InteractionMode, Session } from "../types/agent";
import type { SaveSshConnectionInput } from "../types/ssh-connection";
import { defaultSessionTitleFromPath } from "../lib/session-path";
export function CreateSessionDialog({
  agents,
  onClose,
  onConfigureOnePiece,
  onCreated,
  open,
}: {
  agents: AgentRegistryEntry[];
  onClose: () => void;
  onConfigureOnePiece: () => void;
  onCreated: (session: Session) => void;
  open: boolean;
}) {
  const { t } = useTranslation();
  const availableAgents = useMemo(
    () => selectSessionAgents(agents),
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
  const [multiSeats, setMultiSeats] = useState<SessionSeat[]>([]);
  const [title, setTitle] = useState("");
  const [titleUserEdited, setTitleUserEdited] = useState(false);
  const [workspaceMode, setWorkspaceMode] = useState<WorkspaceMode>("local");
  const [personalizationMode, setPersonalizationMode] =
    useState<SessionPersonalizationMode>("standard");
  const { expertRoles, knownProjects, knownRemoteWorkspaces, sshConnections } =
    useCreateSessionReferenceData(open);
  const [selectedSshConnectionId, setSelectedSshConnectionId] = useState("");
  const [saveSshConnection, setSaveSshConnection] = useState(false);
  const [sshConnectionDraft, setSshConnectionDraft] = useState<SaveSshConnectionInput>(defaultSshConnectionDraft);
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
  const {
    abandonInFlightInspection,
    browseProject,
    inspectPath,
    inspection,
    projectPath,
    resetInspection,
    setProjectPath,
  } =
    useProjectInspection({
      onError: setError,
      onFolderChanged: () => {
        // The worktree choice was made about the previous folder; a new one has no branch yet.
        setWorktreeEnabled(false);
        setWorktreeName("");
      },
      t,
    });
  // Read inside the effect rather than depended on. `agents` is refetched query data, so a new
  // array arrives on every refetch -- and this effect *resets the form*. Keying it on the agent
  // list means a background refresh wipes a half-filled dialog.
  const latestAgents = useRef(availableAgents);
  latestAgents.current = availableAgents;
  useEffect(() => {
    if (open) return;
    // Closing abandons the watch rather than suspending it. The session may still be created and
    // stays reachable from the session list, but reopening the dialog must not resume a creation
    // the user walked away from and navigate them into it.
    setCreateOperationId(null);
    setLoading(false);
  }, [open]);
  useEffect(() => {
    if (!open) return;
    const agent = defaultSessionAgent(
      latestAgents.current,
      window.localStorage.getItem(previousSessionAgentStorageKey),
    );
    setAgentId(agent?.id ?? "");
    setInteractionMode(firstMode(agent));
    setAgentMode("single");
    setTitle("");
    setTitleUserEdited(false);
    setWorkspaceMode("local");
    // Reset rather than remembered: this is a privacy choice, and a dialog that silently repeated
    // the last one would keep making temporary sessions for a user who chose it once, or -- worse
    // in the other direction -- would not, without either being re-confirmed.
    setPersonalizationMode("standard");
    setRemoteHost("");
    setRemotePort("22");
    setRemoteUser("");
    setRemotePath("");
    setRemoteDisplayName("");
    setSelectedSshConnectionId("");
    setSaveSshConnection(false);
    setSshConnectionDraft(defaultSshConnectionDraft);
    setError(null);
    // Left behind by the previous session otherwise: a cleared path with a stale inspection still
    // reports the old folder's Git capability, and the worktree fields stay filled for it.
    setWorktreeEnabled(false);
    setWorktreeName("");
    setMultiSeats([]);
    resetInspection();
  }, [open, resetInspection]);

  useStoredSessionAgent({
    availableAgents,
    onApply: (agent) => {
      setAgentId(agent.id);
      setInteractionMode(firstMode(agent));
    },
    open,
  });

  useCreateSessionOperation({
    active: open,
    handledOperationId: handledCreateOperationId,
    onCreated,
    operationId: createOperationId,
    setError,
    setHandledOperationId: setHandledCreateOperationId,
    setLoading,
    t,
  });
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

  if (!open) return null;
  const gitCapable = inspection?.isGit ?? false;
  const hasWorkspace =
    workspaceMode === "local" ? projectPath.trim() !== "" : remotePath.trim() !== "";
  // The store refuses a project-only session with no workspace, so the selection is
  // corrected here rather than failing at submit against a control the user cannot see.
  const effectivePersonalizationMode = modeForWorkspace(personalizationMode, hasWorkspace);
  const canSubmit = canCreateSession({
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
  });
  return (
    <CreateSessionDialogContent
      agentMode={agentMode}
      availableAgents={availableAgents}
      expertRoles={expertRoles}
      multiSeats={multiSeats}
      onSeatsChange={setMultiSeats}
      canSubmit={canSubmit}
      error={error}
      gitCapable={gitCapable}
      inspection={inspection}
      knownProjects={knownProjects}
      knownRemoteWorkspaces={knownRemoteWorkspaces}
      loading={loading}
      onAgentModeChange={(mode) => {
        setAgentMode(mode);
        // Seed two seats on first switch so the editor opens in a usable state rather than empty.
        if (mode === "multi" && multiSeats.length === 0) {
          const first = availableAgents[0]?.id ?? "";
          setMultiSeats([
            { agentId: first, roleId: null },
            { agentId: availableAgents[1]?.id ?? first, roleId: null },
          ]);
        }
      }}
      onAgentSelect={(agent) => {
        setAgentId(agent.id);
        setInteractionMode(firstMode(agent));
        window.localStorage.setItem(previousSessionAgentStorageKey, agent.id);
      }}
      onBrowseProject={() => void browseProject()}
      onClose={onClose}
      onConfigureOnePiece={onConfigureOnePiece}
      onInspectPath={(path) => void inspectPath(path)}
      onSubmit={() =>
        void submitCreateSession({
          agentMode,
          multiSeats: multiSeats.map((seat) => snapshotSeat(seat, agents, expertRoles)),
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
          personalizationMode: effectivePersonalizationMode,
          workspaceMode,
          worktreeEnabled,
          worktreeName,
        })
      }
      hasWorkspace={hasWorkspace}
      onPersonalizationModeChange={setPersonalizationMode}
      personalizationMode={effectivePersonalizationMode}
      onTitleChange={(value) => {
        setTitleUserEdited(true);
        setTitle(value);
      }}
      onWorkspaceModeChange={(mode) => {
        setWorkspaceMode(mode);
        setWorktreeEnabled(false);
        setError(null);
        // A local inspection still in flight describes a folder this mode does not ask for. Its
        // late failure would otherwise surface in the shared error line of a remote form that
        // shows no project field at all. The chosen folder itself is kept: switching to Remote and
        // back must not empty a path the user already picked.
        abandonInFlightInspection();
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
