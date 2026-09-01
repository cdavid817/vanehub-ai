import { Blend } from "lucide-react";
import { useTranslation } from "react-i18next";
import { CreateSessionSection } from "./create-session-section";
import { InteractionModeSelector } from "./create-session-interaction-mode-selector";
import { agentSupportsRemoteWorkspace } from "./create-session-draft-model";
import { isSessionAgentSelectable } from "./create-session-agents";
import { WorkspaceModeSelector, type WorkspaceMode } from "./create-session-workspace-sections";
import { SessionAgentModeSelector, type SessionAgentMode } from "./session-agent-mode-selector";
import type { AgentRegistryEntry, InteractionMode } from "../types/agent";

/**
 * Step 1 (task 11.3): Single/Multi, CLI/API, and Local/Remote, stated as intent before any
 * specific Agent is chosen (Step 2, task 11.4). CLI/API disables a whole mode only when *no*
 * available Agent could ever honor it — a per-Agent mismatch is not this step's problem to solve,
 * since no Agent is chosen yet; `useCreateSessionDraft`'s own pre-existing reconciliation effect
 * already corrects `interactionMode` the moment a Step 2 choice cannot honor it, so this step's
 * choice is a preference, not a hard commitment.
 */
export function CreateSessionStep1({
  agentMode,
  availableAgents,
  interactionMode,
  onAgentModeChange,
  onInteractionModeChange,
  onWorkspaceModeChange,
  selectedAgent,
  workspaceMode,
}: {
  agentMode: SessionAgentMode;
  availableAgents: AgentRegistryEntry[];
  interactionMode: InteractionMode;
  onAgentModeChange: (mode: SessionAgentMode) => void;
  onInteractionModeChange: (mode: InteractionMode) => void;
  onWorkspaceModeChange: (mode: WorkspaceMode) => void;
  selectedAgent: AgentRegistryEntry | null;
  workspaceMode: WorkspaceMode;
}) {
  const { t } = useTranslation();
  const cliDisabled = !availableAgents.some((agent) => agent.supportedInteractionModes.includes("cli") && isSessionAgentSelectable(agent));
  const apiDisabled = !availableAgents.some((agent) => agent.supportedInteractionModes.includes("api") && isSessionAgentSelectable(agent));
  const remoteDisabled = !agentSupportsRemoteWorkspace(selectedAgent);

  return (
    <CreateSessionSection hint={t("createSession.step1Hint")} icon={Blend} title={t("createSession.step1Title")}>
      <SessionAgentModeSelector mode={agentMode} onModeChange={onAgentModeChange} />
      <InteractionModeSelector
        apiDisabled={apiDisabled}
        cliDisabled={cliDisabled}
        mode={interactionMode}
        onModeChange={onInteractionModeChange}
      />
      {cliDisabled || apiDisabled ? (
        <p className="text-xs text-muted-foreground">
          {t(cliDisabled ? "createSession.interactionMode.cliUnavailable" : "createSession.interactionMode.apiUnavailable")}
        </p>
      ) : null}
      <WorkspaceModeSelector mode={workspaceMode} onModeChange={onWorkspaceModeChange} remoteDisabled={remoteDisabled} />
      {/* This copy names OnePiece specifically because it is currently the only entry in
          `AGENTS_WITHOUT_REMOTE_WORKSPACE_SUPPORT` — see create-session-draft-model.ts. */}
      {remoteDisabled ? <p className="text-xs text-muted-foreground">{t("onepiece.localOnly")}</p> : null}
    </CreateSessionSection>
  );
}
