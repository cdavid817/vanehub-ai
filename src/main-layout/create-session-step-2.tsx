import { UsersRound } from "lucide-react";
import { useTranslation } from "react-i18next";
import { SessionSeatAssignment } from "./session-seat-assignment";
import { withModelFamily } from "../services/agent-model-family";
import { CreateSessionAgentSection } from "./create-session-agent-section";
import { CreateSessionSection } from "./create-session-section";
import { SessionPersonalizationModeSelector } from "./session-personalization-mode-selector";
import type { CreateSessionValidation } from "./create-session-validation";
import type { SessionAgentMode } from "./session-agent-mode-selector";
import type { SessionPersonalizationMode } from "../types/personalization";
import type { AgentRegistryEntry, SessionSeat } from "../types/agent";
import type { ExpertRole } from "../types/expert-role";

/**
 * Step 2 (task 11.4): participants, roles, Agent identity, model-family compatibility (all
 * already implemented inside `SessionSeatAssignment`/`CreateSessionAgentSection`, reused
 * verbatim — this step only re-homes them), and personalization (moved here from the old
 * single-screen dialog's "Workspace" section, since it is a statement about the participant's
 * memory/instructions, not the workspace path itself).
 *
 * "Skill summaries" (also named by 11.4) is deliberately not built here: `SkillCatalogService
 * .listSkills` needs a `scope` and optionally a `workspacePath`
 * (`src/types/skill.ts`), and Step 3 — which comes *after* this one — is where a workspace is
 * chosen. A global-scope summary (skills with no project binding) is representable but would
 * likely mislead a reader into thinking it is the complete set of skills that will actually
 * apply, which cannot be known until Step 3. Left for a dedicated follow-up rather than guessed.
 */
export function CreateSessionStep2({
  agentMode,
  availableAgents,
  expertRoles,
  hasWorkspace,
  multiSeats,
  onAgentSelect,
  onConfigureOnePiece,
  onPersonalizationModeChange,
  onSeatsChange,
  personalizationMode,
  selectedAgent,
  validation,
}: {
  agentMode: SessionAgentMode;
  availableAgents: AgentRegistryEntry[];
  expertRoles: ExpertRole[];
  hasWorkspace: boolean;
  multiSeats: SessionSeat[];
  onAgentSelect: (agent: AgentRegistryEntry) => void;
  onConfigureOnePiece: () => void;
  onPersonalizationModeChange: (mode: SessionPersonalizationMode) => void;
  onSeatsChange: (seats: SessionSeat[]) => void;
  personalizationMode: SessionPersonalizationMode;
  selectedAgent: AgentRegistryEntry | null;
  /** Task 11.10: the field this step owns is `agent` in single mode, `seats` in multi mode --
   *  never both, since only one of the two sections is ever mounted at once. */
  validation: CreateSessionValidation;
}) {
  const { t } = useTranslation();
  const fieldError = agentMode === "multi" ? validation.seats : validation.agent;
  return (
    <CreateSessionSection hint={t("createSession.section.participantsHint")} icon={UsersRound} title={t("createSession.section.participants")}>
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
      {fieldError ? <p className="text-xs text-destructive" role="alert">{t(`createSession.validation.${fieldError}`)}</p> : null}
      <SessionPersonalizationModeSelector
        hasWorkspace={hasWorkspace}
        mode={personalizationMode}
        onChange={onPersonalizationModeChange}
      />
    </CreateSessionSection>
  );
}
