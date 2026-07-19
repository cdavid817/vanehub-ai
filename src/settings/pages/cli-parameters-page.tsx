import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcw, Save, SlidersHorizontal, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import { buildCliParameterPreviewFromDefinitions } from "../../services/cli-parameter-catalog";
import { agentService } from "../../services/runtime-agent-client";
import type {
  CliParameterDefinition,
  CliParameterSelections,
  CliParameterValue,
  ManagedCliAgentId,
  SaveCliParameterProfileInput,
} from "../../types/agent";
import { PageHeader, SectionPanel } from "./page-parts";

const profilesQueryKey = ["cli-parameter-profiles"] as const;

function ParameterControl({
  definition,
  value,
  onChange,
}: {
  definition: CliParameterDefinition;
  value: CliParameterValue;
  onChange: (value: CliParameterValue) => void;
}) {
  const { t } = useTranslation();
  if (definition.control === "boolean") {
    const checked = value === true;
    return (
      <Button
        aria-checked={checked}
        aria-label={t(definition.labelKey)}
        onClick={() => onChange(!checked)}
        size="sm"
        type="button"
        role="switch"
        variant={checked ? "default" : "outline"}
      >
        {t(checked ? "cliParameters.common.enabled" : "cliParameters.common.disabled")}
      </Button>
    );
  }

  const multiple = definition.control === "multi-enum";
  const selectValue = multiple ? (Array.isArray(value) ? value : []) : typeof value === "string" ? value : "default";
  return (
    <div className="space-y-2">
      <select
        aria-label={t(definition.labelKey)}
        className="min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        multiple={multiple}
        onChange={(event) => {
          if (multiple) {
            onChange(Array.from(event.currentTarget.selectedOptions, (option) => option.value));
          } else {
            onChange(event.currentTarget.value);
          }
        }}
        value={selectValue}
      >
        {definition.options.map((option) => (
          <option key={option.value} value={option.value}>
            {t(option.labelKey)}
          </option>
        ))}
      </select>
      {!multiple && typeof selectValue === "string" ? (
        <p className="text-xs leading-5 text-muted-foreground">
          {t(definition.options.find((option) => option.value === selectValue)?.descriptionKey ?? "cliParameters.values.default.description")}
        </p>
      ) : null}
    </div>
  );
}

export function CliParametersPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [activeAgentId, setActiveAgentId] = useState<ManagedCliAgentId>("claude-code");
  const [drafts, setDrafts] = useState<Partial<Record<ManagedCliAgentId, CliParameterSelections>>>({});
  const [notice, setNotice] = useState<string | null>(null);

  const profilesQuery = useQuery({
    queryKey: profilesQueryKey,
    queryFn: () => agentService.listCliParameterProfiles(),
  });
  const profiles = profilesQuery.data ?? [];

  useEffect(() => {
    if (profiles.length === 0) return;
    setDrafts((current) => {
      const next = { ...current };
      for (const profile of profiles) next[profile.agentId] ??= profile.selections;
      return next;
    });
  }, [profiles]);

  const saveMutation = useMutation({
    mutationFn: (input: SaveCliParameterProfileInput) => agentService.saveCliParameterProfile(input),
    onSuccess: async (profile) => {
      setDrafts((current) => ({ ...current, [profile.agentId]: profile.selections }));
      setNotice(t("cliParameters.notice.saved"));
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  });
  const resetMutation = useMutation({
    mutationFn: (agentId: ManagedCliAgentId) => agentService.resetCliParameterProfile(agentId),
    onSuccess: async (profile) => {
      setDrafts((current) => ({ ...current, [profile.agentId]: profile.selections }));
      setNotice(t("cliParameters.notice.reset"));
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  });

  const activeProfile = profiles.find((profile) => profile.agentId === activeAgentId) ?? profiles[0];
  const activeDraft = activeProfile ? (drafts[activeProfile.agentId] ?? activeProfile.selections) : {};
  const query = searchTerm.trim().toLocaleLowerCase();
  const visibleDefinitions = useMemo(() => {
    if (!activeProfile || !query) return activeProfile?.definitions ?? [];
    return activeProfile.definitions.filter((definition) =>
      [definition.flag, t(definition.labelKey), t(definition.descriptionKey), ...definition.options.flatMap((option) => [t(option.labelKey), t(option.descriptionKey)])]
        .join(" ")
        .toLocaleLowerCase()
        .includes(query),
    );
  }, [activeProfile, query, t]);
  const previewArgs = activeProfile
    ? buildCliParameterPreviewFromDefinitions(activeProfile.definitions, activeDraft)
    : [];
  const dirty = activeProfile ? JSON.stringify(activeDraft) !== JSON.stringify(activeProfile.selections) : false;

  function updateParameter(id: string, value: CliParameterValue) {
    if (!activeProfile) return;
    setNotice(null);
    setDrafts((current) => ({
      ...current,
      [activeProfile.agentId]: { ...activeDraft, [id]: value },
    }));
  }

  function resetActiveProfile() {
    if (!activeProfile || !window.confirm(t("cliParameters.confirmReset"))) return;
    resetMutation.mutate(activeProfile.agentId);
  }

  const error = profilesQuery.error ?? saveMutation.error ?? resetMutation.error;
  const rawError = error ? String(error) : null;
  const invalidValueMatch = rawError?.match(/Invalid value for CLI parameter: ([\w-]+)/i)
    ?? rawError?.match(/invalid value for CLI parameter '([^']+)'/i);
  const unknownParameterMatch = rawError?.match(/Unknown CLI parameter: ([\w-]+)/i)
    ?? rawError?.match(/unknown CLI parameter '([^']+)'/i);
  const errorMessage = invalidValueMatch
    ? t("cliParameters.error.invalidValue", { parameter: invalidValueMatch[1] })
    : unknownParameterMatch
      ? t("cliParameters.error.unknownParameter", { parameter: unknownParameterMatch[1] })
      : rawError
        ? t("cliParameters.error.requestFailed", { message: rawError })
        : null;
  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            {dirty ? <Badge tone="warning">{t("cliParameters.common.unsaved")}</Badge> : null}
            <Button disabled={!activeProfile || resetMutation.isPending} onClick={resetActiveProfile} variant="outline">
              <RotateCcw aria-hidden="true" /> {t("cliParameters.actions.reset")}
            </Button>
            <Button
              disabled={!activeProfile || !dirty || saveMutation.isPending}
              onClick={() => activeProfile && saveMutation.mutate({ agentId: activeProfile.agentId, selections: activeDraft })}
            >
              <Save aria-hidden="true" /> {t(saveMutation.isPending ? "cliParameters.actions.saving" : "cliParameters.actions.save")}
            </Button>
          </>
        }
        description={t("cliParameters.description")}
        icon={SlidersHorizontal}
        title={t("cliParameters.title")}
      />

      <div className="grid gap-4 lg:grid-cols-[220px_minmax(0,1fr)]">
        <SectionPanel description={t("cliParameters.agents.description")} title={t("cliParameters.agents.title")}>
          <div className="space-y-2">
            {profiles.map((profile) => (
              <Button
                className="w-full justify-start gap-2"
                key={profile.agentId}
                onClick={() => setActiveAgentId(profile.agentId)}
                variant={activeProfile?.agentId === profile.agentId ? "default" : "ghost"}
              >
                <span className={`flex h-6 w-6 shrink-0 items-center justify-center rounded border ${getAgentVisualIdentity(profile.agentId).tone}`}>
                  <AgentBrandIcon agentId={profile.agentId} className="h-3.5 w-3.5" />
                </span>
                <span className="truncate">{t(`cliParameters.agents.${profile.agentId}`)}</span>
              </Button>
            ))}
          </div>
        </SectionPanel>

        <div className="space-y-4">
          {errorMessage ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{errorMessage}</div> : null}
          {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
          {visibleDefinitions.map((definition) => (
            <section className="ucd-panel ucd-interactive rounded-lg p-4" key={definition.id}>
              <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)] md:items-start">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="text-sm font-semibold">{t(definition.labelKey)}</h3>
                    <Badge tone="muted">{definition.flag}</Badge>
                    {definition.risk === "warning" ? (
                      <Badge tone="warning"><TriangleAlert aria-hidden="true" className="mr-1 h-3 w-3" />{t("cliParameters.common.warning")}</Badge>
                    ) : null}
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    {t("cliParameters.common.scope", { scope: definition.launchScopes.map((scope) => t(`cliParameters.scope.${scope}`)).join(" / ") })}
                  </p>
                </div>
                <ParameterControl definition={definition} onChange={(value) => updateParameter(definition.id, value)} value={activeDraft[definition.id] ?? definition.defaultValue} />
              </div>
            </section>
          ))}
          {activeProfile && visibleDefinitions.length === 0 ? <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("cliParameters.empty")}</div> : null}
          <SectionPanel description={t("cliParameters.preview.description")} title={t("cliParameters.preview.title")}>
            <code className="block break-all rounded-md border border-border bg-muted p-3 text-xs leading-6 text-foreground">
              {previewArgs.length ? previewArgs.join(" ") : t("cliParameters.preview.empty")}
            </code>
          </SectionPanel>
        </div>
      </div>
    </div>
  );
}
