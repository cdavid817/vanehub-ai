import { useTranslation } from "react-i18next";
import type { AgentPersonalizationCapability, MemoryScopeKind, MemoryType } from "../../../types/personalization";
import type { MemoryCandidate } from "../../../types/personalization-memory";
import type { CandidateEdits } from "./use-candidate-review";
import type { WorkspaceOption } from "./use-scope-options";

const TYPES: Exclude<MemoryType, "untyped">[] = ["user", "feedback", "project", "reference"];

/**
 * The fields a reviewer may change before approving.
 *
 * Scope and audience start unset, and unset means "keep what was proposed". Preselecting global
 * would make an edit to the wording quietly widen a workspace memory to every project -- a change
 * the reviewer would have no reason to think they had made.
 */
export function CandidateEditor({
  agents,
  candidate,
  edits,
  onChange,
  workspaces,
}: {
  agents: readonly AgentPersonalizationCapability[];
  candidate: MemoryCandidate;
  edits: CandidateEdits;
  onChange: (patch: Partial<CandidateEdits>) => void;
  workspaces: readonly WorkspaceOption[];
}) {
  const { t } = useTranslation();
  const id = candidate.id;

  return (
    <div className="mt-3 grid gap-3" data-testid={`personalization-candidate-editor-${id}`}>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.name")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid={`personalization-candidate-name-${id}`}
          onChange={(event) => onChange({ name: event.target.value })}
          value={edits.name}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.description_field")}
        <input
          className="ucd-input h-9 rounded-md px-2 text-sm"
          data-testid={`personalization-candidate-description-${id}`}
          onChange={(event) => onChange({ description: event.target.value })}
          value={edits.description}
        />
      </label>
      <label className="flex flex-col gap-1 text-xs font-medium">
        {t("personalization.detail.body")}
        <textarea
          className="ucd-input min-h-24 rounded-md p-2 text-sm"
          data-testid={`personalization-candidate-content-${id}`}
          onChange={(event) => onChange({ content: event.target.value })}
          value={edits.content}
        />
      </label>

      <div className="grid gap-3 sm:grid-cols-2">
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.memoryList.filters.type")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid={`personalization-candidate-type-${id}`}
            onChange={(event) => onChange({ memoryType: event.target.value as (typeof TYPES)[number] })}
            value={edits.memoryType}
          >
            {TYPES.map((type) => (
              <option key={type} value={type}>
                {t(`personalization.memory.type.${type}`)}
              </option>
            ))}
          </select>
        </label>

        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.title")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid={`personalization-candidate-scope-${id}`}
            onChange={(event) =>
              onChange({
                scopeKind: event.target.value as MemoryScopeKind | "",
                workspaceKey:
                  event.target.value === "workspace"
                    ? (edits.workspaceKey || workspaces[0]?.workspaceKey || "")
                    : "",
              })
            }
            value={edits.scopeKind}
          >
            <option value="">{t("personalization.review.keepProposedScope")}</option>
            <option value="global">{t("personalization.overview.source.global")}</option>
            <option value="workspace">{t("personalization.overview.source.workspace")}</option>
          </select>
        </label>
      </div>

      {edits.scopeKind === "workspace" ? (
        <label className="flex flex-col gap-1 text-xs font-medium">
          {t("personalization.scope.workspace")}
          <select
            className="ucd-input h-9 rounded-md px-2 text-sm"
            data-testid={`personalization-candidate-workspace-${id}`}
            onChange={(event) => onChange({ workspaceKey: event.target.value })}
            value={edits.workspaceKey}
          >
            <option value="">{t("personalization.scope.chooseWorkspace")}</option>
            {workspaces.map((workspace) => (
              <option key={workspace.workspaceKey} value={workspace.workspaceKey}>
                {workspace.displayName}
              </option>
            ))}
          </select>
        </label>
      ) : null}

      <fieldset className="flex flex-col gap-2 text-xs font-medium">
        <legend>{t("personalization.detail.audience")}</legend>
        <label className="flex items-center gap-2 font-normal">
          <input
            checked={edits.audienceAgentIds === null}
            data-testid={`personalization-candidate-audience-keep-${id}`}
            onChange={() => onChange({ audienceAgentIds: null })}
            type="radio"
          />
          {t("personalization.review.keepProposedAudience")}
        </label>
        {agents.map((agent) => (
          <label className="flex items-center gap-2 font-normal" key={agent.agentId}>
            <input
              checked={edits.audienceAgentIds?.includes(agent.agentId) ?? false}
              data-testid={`personalization-candidate-audience-${agent.agentId}-${id}`}
              onChange={(event) => {
                const current = edits.audienceAgentIds ?? [];
                onChange({
                  audienceAgentIds: event.target.checked
                    ? [...current, agent.agentId]
                    : current.filter((value) => value !== agent.agentId),
                });
              }}
              type="checkbox"
            />
            {agent.displayName}
          </label>
        ))}
      </fieldset>
    </div>
  );
}
