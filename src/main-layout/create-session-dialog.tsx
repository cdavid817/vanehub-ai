import { CreateSessionDialogContent } from "./create-session-dialog-content";
import { useCreateSessionDraft } from "./use-create-session-draft";
import type { AgentRegistryEntry, Session } from "../types/agent";

/**
 * Wires `useCreateSessionDraft` (the draft/validation model, task 11.1) onto
 * `CreateSessionDialogContent`'s existing props. Everything this component used to own directly
 * -- ~26 pieces of `useState`, four effects, the inspect/browse handlers, and submission -- now
 * lives in the hook; this file is left with only the mapping.
 */
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
  const model = useCreateSessionDraft({ agents, onCreated, open });

  if (!open) return null;
  const { actions, draft, lifecycle, referenceData, validation } = model;

  return (
    <CreateSessionDialogContent
      agentMode={draft.agentMode}
      availableAgents={model.availableAgents}
      expertRoles={referenceData.expertRoles}
      multiSeats={draft.multiSeats}
      onSeatsChange={actions.setSeats}
      canSubmit={validation.canSubmit}
      error={lifecycle.error}
      gitCapable={model.gitCapable}
      inspection={referenceData.inspection}
      knownProjects={referenceData.knownProjects}
      knownRemoteWorkspaces={referenceData.knownRemoteWorkspaces}
      loading={lifecycle.loading}
      onAgentModeChange={actions.setAgentMode}
      onAgentSelect={actions.selectAgent}
      onBrowseProject={actions.browseProject}
      onClose={onClose}
      onConfigureOnePiece={onConfigureOnePiece}
      onInspectPath={actions.inspectPath}
      onSubmit={actions.submit}
      hasWorkspace={model.hasWorkspace}
      onPersonalizationModeChange={actions.setPersonalizationMode}
      personalizationMode={model.effectivePersonalizationMode}
      onTitleChange={actions.setTitle}
      onWorkspaceModeChange={actions.setWorkspaceMode}
      projectPath={draft.projectPath}
      remoteDisplayName={draft.remoteDisplayName}
      remoteHost={draft.remoteHost}
      remotePath={draft.remotePath}
      remotePort={draft.remotePort}
      remoteUser={draft.remoteUser}
      saveSshConnection={draft.saveSshConnection}
      selectedAgent={model.selectedAgent}
      selectedSshConnectionId={draft.selectedSshConnectionId}
      setProjectPath={actions.setProjectPath}
      setRemoteDisplayName={actions.setRemoteDisplayName}
      setRemoteHost={actions.setRemoteHost}
      setRemotePath={actions.setRemotePath}
      setRemotePort={actions.setRemotePort}
      setRemoteUser={actions.setRemoteUser}
      setSaveSshConnection={actions.setSaveSshConnection}
      setSelectedSshConnectionId={actions.setSelectedSshConnectionId}
      setSshConnectionDraft={actions.setSshConnectionDraft}
      setWorktreeEnabled={actions.setWorktreeEnabled}
      setWorktreeName={actions.setWorktreeName}
      sshConnectionDraft={draft.sshConnectionDraft}
      sshConnections={referenceData.sshConnections}
      title={draft.title}
      workspaceMode={draft.workspaceMode}
      worktreeEnabled={draft.worktreeEnabled}
      worktreeName={draft.worktreeName}
    />
  );
}
