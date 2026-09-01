import type {
  AgentRegistryEntry,
  InteractionMode,
  SessionSeat,
} from "../types/agent";
import type { SessionPersonalizationMode } from "../types/personalization";
import type { SaveSshConnectionInput } from "../types/ssh-connection";
import { defaultSessionTitleFromPath } from "../lib/session-path";
import type { SessionAgentMode } from "./session-agent-mode-selector";
import type { WorkspaceMode } from "./create-session-workspace-sections";

/**
 * The create-session draft: every field the dialog lets a user edit before submitting (task
 * 11.1). Deliberately excludes fetched reference data (known projects, SSH connections, expert
 * roles, path inspection) and submission lifecycle (loading/error/operation id) -- neither is
 * something a user edits, and both live in `use-create-session-draft.ts` instead. Independent of
 * dialog presentation: nothing here imports React or any component.
 */
export interface CreateSessionDraft {
  agentId: string;
  interactionMode: InteractionMode;
  agentMode: SessionAgentMode;
  multiSeats: SessionSeat[];
  title: string;
  titleUserEdited: boolean;
  workspaceMode: WorkspaceMode;
  projectPath: string;
  personalizationMode: SessionPersonalizationMode;
  worktreeEnabled: boolean;
  worktreeName: string;
  remoteHost: string;
  remotePort: string;
  remoteUser: string;
  remotePath: string;
  remoteDisplayName: string;
  selectedSshConnectionId: string;
  saveSshConnection: boolean;
  sshConnectionDraft: SaveSshConnectionInput;
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

export function createInitialCreateSessionDraft(): CreateSessionDraft {
  return {
    agentId: "",
    interactionMode: "cli",
    agentMode: "single",
    multiSeats: [],
    title: "",
    titleUserEdited: false,
    workspaceMode: "local",
    projectPath: "",
    personalizationMode: "standard",
    worktreeEnabled: false,
    worktreeName: "",
    remoteHost: "",
    remotePort: "22",
    remoteUser: "",
    remotePath: "",
    remoteDisplayName: "",
    selectedSshConnectionId: "",
    saveSshConnection: false,
    sshConnectionDraft: defaultSshConnectionDraft,
  };
}

/**
 * Agents this dialog will not pair with a remote workspace (task 11.2).
 *
 * No backend capability field expresses this today: `AgentRegistryEntry.capabilityTags` is a
 * closed, already-populated vocabulary (`"coding"`, `"cli"`, `"api"`, `"agent"`, `"native"`,
 * `"browser"`, `"open-source"`) describing what KIND of Agent this is, not a workspace-mode
 * compatibility signal -- verified against every seed/test `capability_tags` value in
 * `src-tauri/src/contexts/agent_runtime/infrastructure/schema.rs` (the real builtin catalog) and
 * the wider Rust tree; none carry a remote/workspace-mode value, and the frontend mirror in
 * `src/services/mock-agent-data.ts` agrees. The constraint itself is real and already enforced,
 * just imperatively rather than declaratively: `SessionApplicationService::
 * create_new_session_record` (`src-tauri/src/contexts/sessions/application/service.rs`) rejects
 * `agent_id == "onepiece"` paired with a remote workspace with a validation error. This table
 * exists so that one rule has exactly one place to live on the frontend instead of being
 * duplicated inline (as it was, in `canCreateSession` and again in
 * `create-session-dialog-content.tsx`) -- it is a frontend stand-in for real service-driven
 * capability negotiation, which does not exist yet.
 */
const AGENTS_WITHOUT_REMOTE_WORKSPACE_SUPPORT = new Set(["onepiece"]);

export function agentSupportsRemoteWorkspace(agent: AgentRegistryEntry | null): boolean {
  if (!agent) return true;
  return !AGENTS_WITHOUT_REMOTE_WORKSPACE_SUPPORT.has(agent.id);
}

export type CreateSessionDraftAction =
  | { type: "reset"; agentId: string; interactionMode: InteractionMode }
  | { type: "select-agent"; agentId: string; interactionMode: InteractionMode }
  | { type: "set-interaction-mode"; interactionMode: InteractionMode }
  | { type: "set-agent-mode"; mode: SessionAgentMode; seedSeats: SessionSeat[] }
  | { type: "set-seats"; seats: SessionSeat[] }
  | { type: "set-title"; title: string }
  | { type: "set-workspace-mode"; mode: WorkspaceMode }
  | { type: "set-personalization-mode"; mode: SessionPersonalizationMode }
  | { type: "set-worktree-enabled"; enabled: boolean }
  | { type: "set-worktree-name"; name: string }
  | { type: "set-project-path"; path: string }
  | { type: "begin-project-path-inspection"; path: string }
  | { type: "set-remote-host"; value: string }
  | { type: "set-remote-port"; value: string }
  | { type: "set-remote-user"; value: string }
  | { type: "set-remote-path"; value: string }
  | { type: "set-remote-display-name"; value: string }
  | { type: "set-save-ssh-connection"; value: boolean }
  | { type: "set-selected-ssh-connection-id"; value: string }
  | { type: "set-ssh-connection-draft"; draft: SaveSshConnectionInput };

/**
 * Re-derives the title from the workspace path whenever the user has not typed their own,
 * matching the dialog's previous `useEffect` exactly but as a synchronous reducer-level
 * invariant applied after every action rather than a separate effect reacting to state after the
 * fact.
 */
function withDerivedTitle(draft: CreateSessionDraft): CreateSessionDraft {
  if (draft.titleUserEdited) return draft;
  const source =
    draft.workspaceMode === "local"
      ? draft.projectPath
      : draft.remoteDisplayName || draft.remotePath;
  return { ...draft, title: defaultSessionTitleFromPath(source) };
}

export function createSessionDraftReducer(
  state: CreateSessionDraft,
  action: CreateSessionDraftAction,
): CreateSessionDraft {
  switch (action.type) {
    case "reset":
      // Deliberately does not touch multiSeats/worktreeEnabled/worktreeName: the dialog this was
      // extracted from never reset them on reopen either, so a faithful extraction preserves that
      // gap rather than quietly closing it.
      return withDerivedTitle({
        ...state,
        agentId: action.agentId,
        interactionMode: action.interactionMode,
        agentMode: "single",
        title: "",
        titleUserEdited: false,
        workspaceMode: "local",
        personalizationMode: "standard",
        projectPath: "",
        remoteHost: "",
        remotePort: "22",
        remoteUser: "",
        remotePath: "",
        remoteDisplayName: "",
        selectedSshConnectionId: "",
        saveSshConnection: false,
        sshConnectionDraft: defaultSshConnectionDraft,
      });
    case "select-agent":
      return withDerivedTitle({
        ...state,
        agentId: action.agentId,
        interactionMode: action.interactionMode,
      });
    case "set-interaction-mode":
      return withDerivedTitle({ ...state, interactionMode: action.interactionMode });
    case "set-agent-mode":
      // Seeds two seats on the first switch to multi so the editor opens usable rather than
      // empty; re-checks state.multiSeats itself rather than trusting the dispatcher's snapshot.
      return withDerivedTitle({
        ...state,
        agentMode: action.mode,
        multiSeats:
          action.mode === "multi" && state.multiSeats.length === 0
            ? action.seedSeats
            : state.multiSeats,
      });
    case "set-seats":
      return withDerivedTitle({ ...state, multiSeats: action.seats });
    case "set-title":
      return withDerivedTitle({ ...state, title: action.title, titleUserEdited: true });
    case "set-workspace-mode":
      return withDerivedTitle({ ...state, workspaceMode: action.mode, worktreeEnabled: false });
    case "set-personalization-mode":
      return withDerivedTitle({ ...state, personalizationMode: action.mode });
    case "set-worktree-enabled":
      return withDerivedTitle({ ...state, worktreeEnabled: action.enabled });
    case "set-worktree-name":
      return withDerivedTitle({ ...state, worktreeName: action.name });
    case "set-project-path":
      return withDerivedTitle({ ...state, projectPath: action.path });
    case "begin-project-path-inspection":
      return withDerivedTitle({
        ...state,
        projectPath: action.path,
        worktreeEnabled: false,
        worktreeName: "",
      });
    case "set-remote-host":
      return withDerivedTitle({ ...state, remoteHost: action.value });
    case "set-remote-port":
      return withDerivedTitle({ ...state, remotePort: action.value });
    case "set-remote-user":
      return withDerivedTitle({ ...state, remoteUser: action.value });
    case "set-remote-path":
      return withDerivedTitle({ ...state, remotePath: action.value });
    case "set-remote-display-name":
      return withDerivedTitle({ ...state, remoteDisplayName: action.value });
    case "set-save-ssh-connection":
      return withDerivedTitle({ ...state, saveSshConnection: action.value });
    case "set-selected-ssh-connection-id":
      return withDerivedTitle({ ...state, selectedSshConnectionId: action.value });
    case "set-ssh-connection-draft":
      return withDerivedTitle({ ...state, sshConnectionDraft: action.draft });
  }
}
