import { useEffect, useMemo, useReducer, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import { operationService } from "../services/runtime-operation-client";
import { sshConnectionService } from "../services/runtime-ssh-connection-client";
import { snapshotSeat } from "../services/seat-presentation";
import { normalizeDisplayPath } from "../lib/session-path";
import { modeForWorkspace } from "./session-personalization-mode-selector";
import {
  defaultSessionAgent,
  previousSessionAgentStorageKey,
  selectSessionAgents,
} from "./create-session-agents";
import {
  createInitialCreateSessionDraft,
  createSessionDraftReducer,
} from "./create-session-draft-model";
import {
  conciseError,
  firstMode,
  resolveCreatedSession,
  submitCreateSession,
} from "./create-session-dialog-utils";
import { validateCreateSessionDraft } from "./create-session-validation";
import type { ExpertRole } from "../types/expert-role";
import type {
  AgentRegistryEntry,
  KnownProject,
  KnownRemoteWorkspace,
  ProjectInspection,
  Session,
  SessionSeat,
} from "../types/agent";
import type { SessionPersonalizationMode } from "../types/personalization";
import type { SaveSshConnectionInput, SshConnection } from "../types/ssh-connection";
import type { SessionAgentMode } from "./session-agent-mode-selector";
import type { WorkspaceMode } from "./create-session-workspace-sections";

interface SubmissionLifecycle {
  loading: boolean;
  error: string | null;
  createOperationId: string | null;
  handledCreateOperationId: string | null;
}

const initialLifecycle: SubmissionLifecycle = {
  loading: false,
  error: null,
  createOperationId: null,
  handledCreateOperationId: null,
};

/**
 * Owns the create-session draft (task 11.1's reducer), the reference data it is validated and
 * populated against, and its submission lifecycle -- the three concerns task 11.1 calls out --
 * so `CreateSessionDialog` is left with only wiring this model's fields onto
 * `CreateSessionDialogContent`'s existing props.
 */
export function useCreateSessionDraft({
  agents,
  onCreated,
  open,
}: {
  agents: AgentRegistryEntry[];
  onCreated: (session: Session) => void;
  open: boolean;
}) {
  const { t } = useTranslation();
  const availableAgents = useMemo(() => selectSessionAgents(agents), [agents]);
  const [draft, dispatch] = useReducer(createSessionDraftReducer, createInitialCreateSessionDraft());
  const selectedAgent =
    availableAgents.find((candidate) => candidate.id === draft.agentId) ?? availableAgents[0] ?? null;

  const [expertRoles, setExpertRoles] = useState<ExpertRole[]>([]);
  const [knownProjects, setKnownProjects] = useState<KnownProject[]>([]);
  const [knownRemoteWorkspaces, setKnownRemoteWorkspaces] = useState<KnownRemoteWorkspace[]>([]);
  const [sshConnections, setSshConnections] = useState<SshConnection[]>([]);
  const [inspection, setInspection] = useState<ProjectInspection | null>(null);

  const [lifecycle, setLifecycle] = useState<SubmissionLifecycle>(initialLifecycle);
  const patchLifecycle = (patch: Partial<SubmissionLifecycle>) =>
    setLifecycle((previous) => ({ ...previous, ...patch }));

  useEffect(() => {
    if (!open) return;
    const agent = defaultSessionAgent(
      availableAgents,
      window.localStorage.getItem(previousSessionAgentStorageKey),
    );
    dispatch({ type: "reset", agentId: agent?.id ?? "", interactionMode: firstMode(agent) });
    patchLifecycle({ error: null });
    void agentService.listExpertRoles().then(setExpertRoles).catch(() => setExpertRoles([]));
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
    if (!lifecycle.createOperationId || lifecycle.handledCreateOperationId === lifecycle.createOperationId) {
      return;
    }
    const operationId = lifecycle.createOperationId;
    let cancelled = false;
    let timer: number | undefined;

    async function pollOperation() {
      try {
        const operation = await operationService.getOperationStatus(operationId);
        if (cancelled) return;
        if (operation.status === "queued" || operation.status === "running") {
          timer = window.setTimeout(() => void pollOperation(), 600);
          return;
        }
        patchLifecycle({ handledCreateOperationId: operation.id, loading: false });
        if (operation.status === "failed") {
          patchLifecycle({ error: operation.error ?? t("createSession.error.command") });
          return;
        }
        const session = await resolveCreatedSession(operation.result);
        if (!session) {
          patchLifecycle({ error: t("createSession.error.command") });
          return;
        }
        onCreated(session);
      } catch (operationError) {
        if (!cancelled) {
          patchLifecycle({ loading: false, error: conciseError(operationError, t) });
        }
      }
    }

    void pollOperation();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
  }, [lifecycle.createOperationId, lifecycle.handledCreateOperationId, onCreated, t]);

  useEffect(() => {
    if (!selectedAgent) return;
    if (!selectedAgent.supportedInteractionModes.includes(draft.interactionMode)) {
      dispatch({ type: "set-interaction-mode", interactionMode: firstMode(selectedAgent) });
    }
  }, [draft.interactionMode, selectedAgent]);

  async function inspectPath(path: string) {
    const trimmed = normalizeDisplayPath(path.trim());
    dispatch({ type: "begin-project-path-inspection", path: trimmed });
    setInspection(null);
    patchLifecycle({ error: null });
    if (!trimmed) return;
    try {
      setInspection(await agentService.inspectProject(trimmed));
    } catch (inspectionError) {
      patchLifecycle({ error: conciseError(inspectionError, t) });
    }
  }

  async function browseProject() {
    patchLifecycle({ error: null });
    try {
      const selectedPath = await agentService.selectProjectDirectory();
      if (selectedPath) await inspectPath(selectedPath);
    } catch (browseError) {
      patchLifecycle({ error: conciseError(browseError, t) });
    }
  }

  function selectAgent(agent: AgentRegistryEntry) {
    dispatch({ type: "select-agent", agentId: agent.id, interactionMode: firstMode(agent) });
    window.localStorage.setItem(previousSessionAgentStorageKey, agent.id);
  }

  function setAgentMode(mode: SessionAgentMode) {
    const first = availableAgents[0]?.id ?? "";
    dispatch({
      type: "set-agent-mode",
      mode,
      seedSeats: [
        { agentId: first, roleId: null },
        { agentId: availableAgents[1]?.id ?? first, roleId: null },
      ],
    });
  }

  function setWorkspaceMode(mode: WorkspaceMode) {
    dispatch({ type: "set-workspace-mode", mode });
    patchLifecycle({ error: null });
  }

  const hasWorkspace =
    draft.workspaceMode === "local"
      ? draft.projectPath.trim() !== ""
      : draft.remotePath.trim() !== "";
  const effectivePersonalizationMode = modeForWorkspace(draft.personalizationMode, hasWorkspace);
  const validation = validateCreateSessionDraft(draft, selectedAgent, availableAgents);

  function submit() {
    void submitCreateSession({
      agentMode: draft.agentMode,
      multiSeats: draft.multiSeats.map((seat) => snapshotSeat(seat, agents, expertRoles)),
      interactionMode: draft.interactionMode,
      projectPath: draft.projectPath,
      remoteDisplayName: draft.remoteDisplayName,
      remoteHost: draft.remoteHost,
      remotePath: draft.remotePath,
      remotePort: draft.remotePort,
      remoteUser: draft.remoteUser,
      saveSshConnection: draft.saveSshConnection,
      selectedSshConnectionId: draft.selectedSshConnectionId,
      selectedAgent,
      setCreateOperationId: (value) => patchLifecycle({ createOperationId: value }),
      setError: (value) => patchLifecycle({ error: value }),
      setHandledCreateOperationId: (value) => patchLifecycle({ handledCreateOperationId: value }),
      setLoading: (value) => patchLifecycle({ loading: value }),
      sshConnectionDraft: draft.sshConnectionDraft,
      title: draft.title,
      t,
      personalizationMode: effectivePersonalizationMode,
      workspaceMode: draft.workspaceMode,
      worktreeEnabled: draft.worktreeEnabled,
      worktreeName: draft.worktreeName,
    });
  }

  return {
    availableAgents,
    selectedAgent,
    draft,
    referenceData: { expertRoles, knownProjects, knownRemoteWorkspaces, sshConnections, inspection },
    lifecycle,
    gitCapable: inspection?.isGit ?? false,
    hasWorkspace,
    effectivePersonalizationMode,
    validation,
    actions: {
      selectAgent,
      setAgentMode,
      setSeats: (seats: SessionSeat[]) => dispatch({ type: "set-seats", seats }),
      setTitle: (title: string) => dispatch({ type: "set-title", title }),
      setWorkspaceMode,
      setPersonalizationMode: (mode: SessionPersonalizationMode) =>
        dispatch({ type: "set-personalization-mode", mode }),
      setWorktreeEnabled: (enabled: boolean) => dispatch({ type: "set-worktree-enabled", enabled }),
      setWorktreeName: (name: string) => dispatch({ type: "set-worktree-name", name }),
      setProjectPath: (path: string) => dispatch({ type: "set-project-path", path }),
      setRemoteHost: (value: string) => dispatch({ type: "set-remote-host", value }),
      setRemotePort: (value: string) => dispatch({ type: "set-remote-port", value }),
      setRemoteUser: (value: string) => dispatch({ type: "set-remote-user", value }),
      setRemotePath: (value: string) => dispatch({ type: "set-remote-path", value }),
      setRemoteDisplayName: (value: string) => dispatch({ type: "set-remote-display-name", value }),
      setSaveSshConnection: (value: boolean) => dispatch({ type: "set-save-ssh-connection", value }),
      setSelectedSshConnectionId: (value: string) =>
        dispatch({ type: "set-selected-ssh-connection-id", value }),
      setSshConnectionDraft: (sshDraft: SaveSshConnectionInput) =>
        dispatch({ type: "set-ssh-connection-draft", draft: sshDraft }),
      inspectPath: (path: string) => void inspectPath(path),
      browseProject: () => void browseProject(),
      submit,
    },
  };
}
