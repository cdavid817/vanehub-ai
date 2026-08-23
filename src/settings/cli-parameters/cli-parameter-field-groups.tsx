import { useTranslation } from "react-i18next";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliLaunchScope } from "../../types/cli-parameter";
import type { CliParameterFieldView, CliParameterProfile } from "../../types/cli-parameter-profile";
import { CliParameterField } from "./cli-parameter-field";
import type { CliParameterDraftsApi } from "./use-cli-parameter-drafts";
import { cliParameterCategories, fieldMatches, type CliParameterFilter } from "./view-model";

export interface CliParameterFieldGroupsProps {
  agentId: ManagedCliAgentId;
  profile: CliParameterProfile;
  drafts: CliParameterDraftsApi;
  query: string;
  filter: CliParameterFilter;
  scope: CliLaunchScope;
}

/** Groups by the registry's own categories and omits the empty ones, so the page never shows a
 * heading with nothing under it. */
export function CliParameterFieldGroups({
  agentId,
  profile,
  drafts,
  query,
  filter,
  scope,
}: CliParameterFieldGroupsProps) {
  const { t } = useTranslation();
  const dirty = new Set(drafts.dirtyIdsFor(agentId));
  const selections = drafts.selectionsFor(agentId);

  const visible = profile.fields.filter((field) =>
    fieldMatches({
      field,
      dirty: dirty.has(field.definition.id),
      diagnostics: profile.diagnostics,
      query,
      filter,
      scope,
      translate: t,
    }),
  );

  if (visible.length === 0) {
    return (
      <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">
        {query ? t("cliParameters.empty") : t("cliParameters.empty.filtered")}
      </div>
    );
  }

  function render(field: CliParameterFieldView) {
    const id = field.definition.id;
    return (
      <CliParameterField
        customInput={drafts.customInputFor(agentId, id)}
        customMode={drafts.isCustomMode(agentId, id)}
        diagnostics={profile.diagnostics.filter((entry) => entry.parameterId === id)}
        dirty={dirty.has(id)}
        field={field}
        invalid={drafts.isInvalid(agentId, id)}
        key={id}
        onOpenCustom={(seed) => drafts.openCustom(agentId, id, seed)}
        onSelect={(selection) => drafts.select(agentId, id, selection)}
        onTypeCustom={(text) => drafts.typeCustom(agentId, id, text)}
        selections={selections}
        value={selections[id] ?? field.definition.defaultSelection}
      />
    );
  }

  return (
    <div className="space-y-6">
      {cliParameterCategories.map((category) => {
        const fields = visible.filter((field) => field.definition.category === category);
        if (fields.length === 0) return null;
        return (
          <section aria-labelledby={`cli-parameter-group-${category}`} key={category}>
            <h3
              className="mb-2 text-sm font-semibold tracking-tight"
              id={`cli-parameter-group-${category}`}
            >
              {t(`cliParameters.category.${category}`)}
            </h3>
            <div className="space-y-3">{fields.map(render)}</div>
          </section>
        );
      })}
    </div>
  );
}
