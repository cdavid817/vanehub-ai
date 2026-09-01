import { FolderTree } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CreateSessionSection } from "./create-session-section";
import { LocalWorkspaceSection } from "./create-session-workspace-sections";
import { RemoteWorkspaceSection } from "./create-session-remote-workspace-section";
import type { CreateSessionValidation } from "./create-session-validation";
import type {
  KnownProject,
  KnownRemoteWorkspace,
  ProjectInspection,
} from "../types/agent";
import type { SaveSshConnectionInput, SshConnection } from "../types/ssh-connection";
import type { WorkspaceMode } from "./create-session-workspace-sections";

/**
 * Step 3 (task 11.6): "recent/discovered project, remote workspace, branch, worktree,
 * availability, and trust." `LocalWorkspaceSection`/`RemoteWorkspaceSection` (reused verbatim,
 * only re-homed here) already cover recent/discovered project (`knownProjects`), remote
 * workspace, branch/worktree (`worktreeEnabled`/`worktreeName`), and availability
 * (`ProjectInspection.isGit`/`gitRoot` drive the git-vs-folder distinction shown today). "Trust"
 * has no backing concept anywhere in this app's data model — grepped `ProjectInspection`,
 * `RemoteWorkspace`, and the SSH connection types for a trust/verification field and found none —
 * so nothing was invented for it here; a real trust signal (e.g. an unrecognized-host warning for
 * a remote connection) would need its own design and backend support, not a guessed indicator.
 */
export function CreateSessionStep3({
  gitCapable,
  inspection,
  knownProjects,
  knownRemoteWorkspaces,
  onBrowseProject,
  onInspectPath,
  projectPath,
  remoteDisplayName,
  remoteHost,
  remotePath,
  remotePort,
  remoteUser,
  saveSshConnection,
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
  validation,
  workspaceMode,
  worktreeEnabled,
  worktreeName,
}: {
  gitCapable: boolean;
  inspection: ProjectInspection | null;
  knownProjects: KnownProject[];
  knownRemoteWorkspaces: KnownRemoteWorkspace[];
  onBrowseProject: () => void;
  onInspectPath: (path: string) => void;
  projectPath: string;
  remoteDisplayName: string;
  remoteHost: string;
  remotePath: string;
  remotePort: string;
  remoteUser: string;
  saveSshConnection: boolean;
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
  /** Task 11.10: the field this step owns is `workspace`; `sshConnection` is shown inline by
   *  `RemoteWorkspaceSection` itself already, at the field it actually describes. */
  validation: CreateSessionValidation;
  workspaceMode: WorkspaceMode;
  worktreeEnabled: boolean;
  worktreeName: string;
}) {
  const { t } = useTranslation();
  return (
    <CreateSessionSection hint={t("createSession.section.workspaceHint")} icon={FolderTree} title={t("createSession.section.workspace")}>
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
      {validation.workspace ? <p className="text-xs text-destructive" role="alert">{t(`createSession.validation.${validation.workspace}`)}</p> : null}
    </CreateSessionSection>
  );
}
