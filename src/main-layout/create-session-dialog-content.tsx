import { Loader2, X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { CreateSessionAgentSection } from "./create-session-agent-section";
import { RemoteWorkspaceSection } from "./create-session-remote-workspace-section";
import {
  LocalWorkspaceSection,
  WorkspaceModeSelector,
  type WorkspaceMode,
} from "./create-session-workspace-sections";
import {
  SessionAgentModeSelector,
  type SessionAgentMode,
} from "./session-agent-mode-selector";
import type {
  AgentRegistryEntry,
  KnownProject,
  KnownRemoteWorkspace,
  ProjectInspection,
} from "../types/agent";
import type {
  SaveSshConnectionInput,
  SshConnection,
} from "../types/ssh-connection";

export function CreateSessionDialogContent({
  agentMode,
  availableAgents,
  canSubmit,
  error,
  gitCapable,
  inspection,
  knownProjects,
  knownRemoteWorkspaces,
  loading,
  onAgentModeChange,
  onAgentSelect,
  onBrowseProject,
  onClose,
  onInspectPath,
  onSubmit,
  onTitleChange,
  onWorkspaceModeChange,
  projectPath,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
  selectedAgent,
  selectedSshConnectionId,
  setProjectPath,
  setRemoteDisplayName,
  setRemoteHost,
  setRemotePath,
  setRemotePort,
  setRemoteUser,
  setSaveSshConnection,
  setSelectedSshConnectionId,
  setSshConnectionDraft,
  setWorktreeEnabled,
  setWorktreeName,
  sshConnectionDraft,
  sshConnections,
  title,
  workspaceMode,
  worktreeEnabled,
  worktreeName,
}: {
  agentMode: SessionAgentMode;
  availableAgents: AgentRegistryEntry[];
  canSubmit: boolean;
  error: string | null;
  gitCapable: boolean;
  inspection: ProjectInspection | null;
  knownProjects: KnownProject[];
  knownRemoteWorkspaces: KnownRemoteWorkspace[];
  loading: boolean;
  onAgentModeChange: (mode: SessionAgentMode) => void;
  onAgentSelect: (agent: AgentRegistryEntry) => void;
  onBrowseProject: () => void;
  onClose: () => void;
  onInspectPath: (path: string) => void;
  onSubmit: () => void;
  onTitleChange: (value: string) => void;
  onWorkspaceModeChange: (mode: WorkspaceMode) => void;
  projectPath: string;
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
  selectedAgent: AgentRegistryEntry | null;
  selectedSshConnectionId: string;
  setProjectPath: (value: string) => void;
  setRemoteDisplayName: (value: string) => void;
  setRemoteHost: (value: string) => void;
  setRemotePath: (value: string) => void;
  setRemotePort: (value: string) => void;
  setRemoteUser: (value: string) => void;
  setSaveSshConnection: (value: boolean) => void;
  setSelectedSshConnectionId: (value: string) => void;
  setSshConnectionDraft: (value: SaveSshConnectionInput) => void;
  setWorktreeEnabled: (value: boolean) => void;
  setWorktreeName: (value: string) => void;
  sshConnectionDraft: SaveSshConnectionInput;
  sshConnections: SshConnection[];
  title: string;
  workspaceMode: WorkspaceMode;
  worktreeEnabled: boolean;
  worktreeName: string;
}) {
  const { t } = useTranslation();

  return (
    <div className="fixed inset-0 z-50 grid place-items-center bg-background/70 p-4">
      <div className="ucd-panel grid max-h-[88vh] w-full max-w-2xl grid-rows-[auto_minmax(0,1fr)_auto] overflow-hidden rounded-lg shadow-xl">
        <div className="flex items-center justify-between border-b border-border p-4">
          <div>
            <h3 className="text-sm font-semibold">
              {t("createSession.title")}
            </h3>
            <p className="mt-1 text-xs text-muted-foreground">
              {t("createSession.description")}
            </p>
          </div>
          <Button className="h-8 w-8 px-0" onClick={onClose} variant="outline">
            <X className="h-4 w-4" aria-hidden="true" />
          </Button>
        </div>

        <div className="min-h-0 overflow-y-auto p-4">
          <div className="grid gap-4">
            <SessionAgentModeSelector
              mode={agentMode}
              onModeChange={onAgentModeChange}
            />
            <CreateSessionAgentSection
              disabled={agentMode !== "single"}
              agents={availableAgents}
              onAgentSelect={onAgentSelect}
              selectedAgent={selectedAgent}
            />
            <WorkspaceModeSelector
              mode={workspaceMode}
              onModeChange={onWorkspaceModeChange}
            />
            {workspaceMode === "local" ? (
              <LocalWorkspaceSection
                gitCapable={gitCapable}
                inspection={inspection}
                knownProjects={knownProjects}
                onBrowseProject={onBrowseProject}
                onInspectPath={onInspectPath}
                projectPath={projectPath}
                setProjectPath={setProjectPath}
                setWorktreeEnabled={setWorktreeEnabled}
                setWorktreeName={setWorktreeName}
                worktreeEnabled={worktreeEnabled}
                worktreeName={worktreeName}
              />
            ) : (
              <RemoteWorkspaceSection
                knownRemoteWorkspaces={knownRemoteWorkspaces}
                remoteDisplayName={remoteDisplayName}
                remoteHost={remoteHost}
                remotePath={remotePath}
                remotePort={remotePort}
                remoteUser={remoteUser}
                saveSshConnection={saveSshConnection}
                selectedSshConnectionId={selectedSshConnectionId}
                setRemoteDisplayName={setRemoteDisplayName}
                setRemoteHost={setRemoteHost}
                setRemotePath={setRemotePath}
                setRemotePort={setRemotePort}
                setRemoteUser={setRemoteUser}
                setSaveSshConnection={setSaveSshConnection}
                setSelectedSshConnectionId={setSelectedSshConnectionId}
                setSshConnectionDraft={setSshConnectionDraft}
                sshConnectionDraft={sshConnectionDraft}
                sshConnections={sshConnections}
              />
            )}
            <label className="grid gap-1">
              <span className="text-xs font-medium text-muted-foreground">
                {t("createSession.sessionName")}
              </span>
              <input
                className="ucd-input h-9 rounded px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                onChange={(event) => onTitleChange(event.target.value)}
                placeholder={t("createSession.sessionPlaceholder")}
                value={title}
              />
            </label>
          </div>
        </div>

        <div className="flex items-center justify-between gap-3 border-t border-border p-4">
          <span className="min-w-0 truncate text-xs text-destructive">
            {error}
          </span>
          <div className="flex gap-2">
            <Button
              className="h-8 px-3 text-xs"
              onClick={onClose}
              type="button"
              variant="outline"
            >
              {t("createSession.cancel")}
            </Button>
            <Button
              className="h-8 px-3 text-xs"
              disabled={!canSubmit || loading}
              onClick={onSubmit}
              type="button"
            >
              {loading ? (
                <Loader2
                  className="h-3.5 w-3.5 animate-spin"
                  aria-hidden="true"
                />
              ) : null}
              {t("createSession.create")}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
