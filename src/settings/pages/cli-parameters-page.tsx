import { useEffect, useMemo, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { RotateCcw, Save, SlidersHorizontal, TriangleAlert } from "lucide-react";
import { useTranslation } from "react-i18next";
import { AgentBrandIcon } from "../../components/agent-brand-icon";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { useConfirmation } from "../../components/ui/use-confirmation";
import { getAgentVisualIdentity } from "../../lib/agent-visual-identity";
import { orderByAgentPriority } from "../../lib/agent-display-order";
import { agentService } from "../../services/runtime-agent-client";
import type { ManagedCliAgentId } from "../../types/agent";
import type { CliParameterSelection, CliParameterSelections } from "../../types/cli-parameter";
import type {
  CliParameterProfile,
  SaveCliParameterProfileInput,
} from "../../types/cli-parameter-profile";
import { CliParameterControl } from "./cli-parameter-control";
import {
  asCliParameterServiceError,
  cliParameterDisplayFlag,
  cliParameterErrorMessageKey,
  cliParameterSearchText,
} from "./cli-parameter-view-model";
import { OnePieceParametersPanel } from "./onepiece-parameters-panel";
import { PageHeader, SectionPanel } from "./page-parts";

const profilesQueryKey = ["cli-parameter-profiles"] as const;
const emptyProfiles: CliParameterProfile[] = [];

type ParameterPageId = ManagedCliAgentId | "onepiece";

export function CliParametersPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const { confirm, confirmationDialog } = useConfirmation();
  const queryClient = useQueryClient();
  const [activeAgentId, setActiveAgentId] = useState<ParameterPageId>("claude-code");
  const [drafts, setDrafts] = useState<Partial<Record<ManagedCliAgentId, CliParameterSelections>>>({});
  const [notice, setNotice] = useState<string | null>(null);

  const profilesQuery = useQuery({
    queryKey: profilesQueryKey,
    queryFn: () => agentService.listCliParameterProfiles(),
  });
  const profiles = useMemo<readonly CliParameterProfile[]>(
    () => orderByAgentPriority(profilesQuery.data ?? emptyProfiles, (profile) => profile.agentId),
    [profilesQuery.data],
  );

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
    mutationFn: (profile: CliParameterProfile) =>
      agentService.resetCliParameterProfile({
        agentId: profile.agentId,
        expectedRevision: profile.revision,
        catalogVersion: profile.catalogVersion,
      }),
    onSuccess: async (profile) => {
      setDrafts((current) => ({ ...current, [profile.agentId]: profile.selections }));
      setNotice(t("cliParameters.notice.reset"));
      await queryClient.invalidateQueries({ queryKey: profilesQueryKey });
    },
  });

  const activeProfile = profiles.find((profile) => profile.agentId === activeAgentId);
  const activeDraft = activeProfile ? (drafts[activeProfile.agentId] ?? activeProfile.selections) : {};
  const query = searchTerm.trim().toLocaleLowerCase();
  const visibleFields = useMemo(() => {
    const fields = activeProfile?.fields ?? [];
    if (!query) return fields;
    return fields.filter((field) => cliParameterSearchText(field.definition, t).includes(query));
  }, [activeProfile, query, t]);

  // Read-only, and keyed by the draft itself: a response for an older draft belongs to an older
  // key, so react-query discards it rather than the page racing to.
  const previewQuery = useQuery({
    enabled: Boolean(activeProfile),
    queryKey: ["cli-parameter-preview", activeAgentId, JSON.stringify(activeDraft)],
    queryFn: () =>
      agentService.previewCliParameterProfile({
        agentId: activeProfile!.agentId,
        catalogVersion: activeProfile!.catalogVersion,
        scope: "chat",
        selections: activeDraft,
      }),
  });
  const previewArgs = previewQuery.data
    ? [...previewQuery.data.segments.global, ...previewQuery.data.segments.invocation].map(
        (token) => token.value,
      )
    : [];
  const dirty = activeProfile
    ? JSON.stringify(activeDraft) !== JSON.stringify(activeProfile.selections)
    : false;

  function updateParameter(id: string, value: CliParameterSelection) {
    if (!activeProfile) return;
    setNotice(null);
    setDrafts((current) => ({
      ...current,
      [activeProfile.agentId]: { ...activeDraft, [id]: value },
    }));
  }

  async function resetActiveProfile() {
    if (!activeProfile) return;
    if (!(await confirm({ title: t("cliParameters.confirmReset"), tone: "danger" }))) return;
    resetMutation.mutate(activeProfile);
  }

  const error = profilesQuery.error ?? saveMutation.error ?? resetMutation.error ?? previewQuery.error;
  const structured = asCliParameterServiceError(error);
  const errorMessage = structured
    ? t(cliParameterErrorMessageKey(structured.code), {
        parameter: structured.parameterId ?? "",
        ...structured.details,
      })
    : error
      ? t("cliParameters.error.requestFailed", { message: String(error) })
      : null;
  return (
    <div className="space-y-4">
      {confirmationDialog}
      <PageHeader
        actions={
          <>
            {activeAgentId !== "onepiece" && dirty ? <Badge tone="warning">{t("cliParameters.common.unsaved")}</Badge> : null}
            {activeAgentId !== "onepiece" ? <>
            <Button disabled={!activeProfile || resetMutation.isPending} onClick={() => void resetActiveProfile()} variant="outline">
              <RotateCcw aria-hidden="true" /> {t("cliParameters.actions.reset")}
            </Button>
            </> : null}
            <Button
              disabled={!activeProfile || !dirty || saveMutation.isPending}
              onClick={() =>
                activeProfile &&
                saveMutation.mutate({
                  agentId: activeProfile.agentId,
                  expectedRevision: activeProfile.revision,
                  catalogVersion: activeProfile.catalogVersion,
                  selections: activeDraft,
                })
              }
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
        <SectionPanel className="sticky top-4 self-start" description={t("cliParameters.agents.description")} title={t("cliParameters.agents.title")}>
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
            <Button className="w-full justify-start gap-2" onClick={() => setActiveAgentId("onepiece")} variant={activeAgentId === "onepiece" ? "default" : "ghost"}>
              <span className={`flex h-6 w-6 shrink-0 items-center justify-center rounded border ${getAgentVisualIdentity("onepiece").tone}`}><AgentBrandIcon agentId="onepiece" className="h-3.5 w-3.5" /></span>
              <span className="truncate">{t("cliParameters.agents.onepiece")}</span>
            </Button>
          </div>
        </SectionPanel>

        <div className="space-y-4">
          {activeAgentId === "onepiece" ? <OnePieceParametersPanel /> : <>
          {errorMessage ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{errorMessage}</div> : null}
          {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
          {activeProfile ? <p className="text-xs leading-5 text-muted-foreground">{t("cliParameters.policyPrecedenceNotice")}</p> : null}
          {visibleFields.map(({ definition }) => (
            <section className="ucd-panel ucd-interactive rounded-lg p-4" key={definition.id}>
              <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)] md:items-start">
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <h3 className="text-sm font-semibold">{t(definition.labelKey)}</h3>
                    <Badge tone="muted">{cliParameterDisplayFlag(definition)}</Badge>
                    {definition.risk === "warning" ? (
                      <Badge tone="warning"><TriangleAlert aria-hidden="true" className="mr-1 h-3 w-3" />{t("cliParameters.common.warning")}</Badge>
                    ) : null}
                  </div>
                  <p className="mt-2 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
                  <p className="mt-2 text-xs text-muted-foreground">
                    {t("cliParameters.common.scope", { scope: definition.launchScopes.map((scope) => t(`cliParameters.scope.${scope}`)).join(" / ") })}
                  </p>
                </div>
                <CliParameterControl definition={definition} onChange={(value) => updateParameter(definition.id, value)} value={activeDraft[definition.id] ?? definition.defaultSelection} />
              </div>
            </section>
          ))}
          {activeProfile && visibleFields.length === 0 ? <div className="ucd-panel rounded-lg p-6 text-sm text-muted-foreground">{t("cliParameters.empty")}</div> : null}
          <SectionPanel description={t("cliParameters.preview.description")} title={t("cliParameters.preview.title")}>
            <code className="block break-all rounded-md border border-border bg-muted p-3 text-xs leading-6 text-foreground">
              {previewArgs.length ? previewArgs.join(" ") : t("cliParameters.preview.empty")}
            </code>
          </SectionPanel>
          </>}
        </div>
      </div>
    </div>
  );
}
