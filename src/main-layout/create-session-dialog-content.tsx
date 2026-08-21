import { SessionSeatAssignment } from "./session-seat-assignment";
import { withModelFamily } from "../services/agent-model-family";
import type { SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";
import { FolderTree, Loader2, Tag, UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../components/ui/application-dialog";
import { Button } from "../components/ui/button";
import { CreateSessionAgentSection } from "./create-session-agent-section";
import { CreateSessionSection } from "./create-session-section";
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
  expertRoles,
  multiSeats,
  onSeatsChange,
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
  onConfigureOnePiece,
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
  expertRoles: ExpertRole[];
  multiSeats: SessionSeat[];
  onSeatsChange: (seats: SessionSeat[]) => void;
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
  onConfigureOnePiece: () => void;
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
    <ApplicationDialog
      closeDisabled={loading}
      description={t("createSession.description")}
      footer={(
        <div className="flex items-start justify-between gap-3">
          {/* Truncating this hid the reason a path or SSH target was rejected, which is the
              only actionable part of the failure. */}
          <p className="min-w-0 flex-1 wrap-break-word text-xs leading-5 text-destructive" role="alert">
            {error}
          </p>
          <div className="flex shrink-0 gap-2">
            <Button
              className="h-8 px-3 text-xs"
              disabled={loading}
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
      )}
      onClose={onClose}
      title={t("createSession.title")}
    >
      <div className="grid gap-4">
        <CreateSessionSection
          hint={t("createSession.section.participantsHint")}
          icon={UsersRound}
          title={t("createSession.section.participants")}
        >
          <SessionAgentModeSelector
            mode={agentMode}
            onModeChange={onAgentModeChange}
          />
          {agentMode === "multi" ? (
            <SessionSeatAssignment
              agents={withModelFamily(availableAgents)}
              onSeatsChange={onSeatsChange}
              roles={expertRoles}
              seats={multiSeats}
            />
          ) : (
            <CreateSessionAgentSection
              agents={availableAgents}
              onAgentSelect={onAgentSelect}
              onConfigureOnePiece={onConfigureOnePiece}
              selectedAgent={selectedAgent}
            />
          )}
        </CreateSessionSection>
        <CreateSessionSection
          hint={t("createSession.section.workspaceHint")}
          icon={FolderTree}
          title={t("createSession.section.workspace")}
        >
          <WorkspaceModeSelector
            mode={workspaceMode}
            onModeChange={onWorkspaceModeChange}
            remoteDisabled={selectedAgent?.id === "onepiece"}
          />
          {selectedAgent?.id === "onepiece" ? <p className="text-xs text-muted-foreground">{t("onepiece.localOnly")}</p> : null}
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
        </CreateSessionSection>
        <CreateSessionSection
          hint={t("createSession.section.identityHint")}
          icon={Tag}
          title={t("createSession.section.identity")}
        >
          <label className="grid gap-1">
            <span className="text-xs font-medium text-muted-foreground">
              {t("createSession.sessionName")}
            </span>
            <input
              className="ucd-input h-9 rounded px-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring"
              onChange={(event) => onTitleChange(event.target.value)}
              placeholder={t("createSession.sessionPlaceholder")}
              value={title}
            />
          </label>
        </CreateSessionSection>
      </div>
    </ApplicationDialog>
  );
}
