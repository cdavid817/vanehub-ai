import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Box, Check, Globe2, KeyRound, LoaderCircle, Pencil, Plus, RefreshCw, Search, ShieldCheck, Trash2 } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { ProviderBrandIcon } from "../../../components/provider-brand-icon";
import { ProviderCredentialValidation } from "../../../components/provider-directory/provider-credential-validation";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { OnePieceProviderProfile, OnePieceProviderProfiles } from "../../../types/agent";
import { OnePieceProfileActionDialog, type OnePieceProfileAction } from "./onepiece-profile-action-dialog";
import { OnePieceProviderDialog } from "./onepiece-provider-dialog";
import { OnePieceRetrievalSection } from "./onepiece-retrieval-section";

const queryKey = ["agents", "onepiece-provider-profiles"] as const;

export function OnePieceConfigurationPanel({ onChanged, searchTerm = "", service = defaultAgentService }: { onChanged: () => Promise<void>; searchTerm?: string; service?: AgentService }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editingProfile, setEditingProfile] = useState<OnePieceProviderProfile | null | undefined>(undefined);
  const [pendingAction, setPendingAction] = useState<OnePieceProfileAction | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [profileSearch, setProfileSearch] = useState("");
  const profilesQuery = useQuery({ queryKey, queryFn: async () => {
    const [overview, presets] = await Promise.all([
      service.listOnePieceProviderProfiles(),
      service.listOnePieceProviderPresets(),
    ]);
    return { overview, presets };
  } });
  const overview = profilesQuery.data?.overview ?? { profiles: [], activeProfileId: null };
  const presets = profilesQuery.data?.presets ?? [];
  const searchQueries = [searchTerm, profileSearch].map((value) => value.trim().toLocaleLowerCase()).filter(Boolean);
  const filteredProfiles = searchQueries.length
    ? overview.profiles.filter((profile) => {
      const searchable = [profile.name, profile.provider, profile.modelId, profile.baseUrl ?? ""].join(" ").toLocaleLowerCase();
      return searchQueries.every((query) => searchable.includes(query));
    })
    : overview.profiles;
  const activeProfile = overview.profiles.find((profile) => profile.active) ?? null;
  const ready = Boolean(activeProfile?.credentialPresent);

  async function updateOverview(value: OnePieceProviderProfiles, message: string) {
    queryClient.setQueryData(queryKey, { overview: value, presets });
    setNotice(message);
    setEditingProfile(undefined);
    setPendingAction(null);
    await onChanged();
  }
  const activateMutation = useMutation({ mutationFn: (profileId: string) => service.activateOnePieceProviderProfile(profileId), onSuccess: (value) => updateOverview(value, t("onepiece.activated")) });
  const deleteMutation = useMutation({ mutationFn: (profileId: string) => service.deleteOnePieceProviderProfile(profileId), onSuccess: (value) => updateOverview(value, t("onepiece.deleted")) });
  const operationError = profilesQuery.error ?? activateMutation.error ?? deleteMutation.error;

  if (profilesQuery.isLoading) {
    return <div className="flex min-h-48 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-center">
        <div className="relative min-w-0 flex-1">
          <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
          <input
            aria-label={t("agentConfigurations.toolbar.search")}
            className="ucd-input h-10 w-full rounded-lg pl-9 pr-3 text-sm"
            onChange={(event) => setProfileSearch(event.target.value)}
            placeholder={t("agentConfigurations.toolbar.search")}
            value={profileSearch}
          />
        </div>
        <div className="grid grid-cols-2 gap-2 sm:flex">
          <Button onClick={() => setEditingProfile(null)}><Plus className="h-4 w-4" />{t("onepiece.addProvider")}</Button>
          <Button aria-label={t("onepiece.refresh")} disabled={profilesQuery.isFetching} onClick={() => void profilesQuery.refetch()} variant="outline"><RefreshCw className={`h-4 w-4 ${profilesQuery.isFetching ? "animate-spin" : ""}`} /><span className="sm:hidden lg:inline">{t("onepiece.refresh")}</span></Button>
        </div>
      </div>

      <section aria-label={t("onepiece.status.title")} className="rounded-lg border border-border bg-muted/35 px-4 py-3">
        <div className="flex flex-wrap items-center gap-2">
          <div className="flex items-center gap-2 text-sm font-semibold"><ShieldCheck className="h-4 w-4 text-primary" />{t("onepiece.status.title")}</div>
          <Badge tone={ready ? "success" : "warning"}>{ready ? t("onepiece.status.ready") : t("onepiece.status.incomplete")}</Badge>
          <p className="text-xs text-muted-foreground sm:ml-auto">{ready ? t("onepiece.ready") : t("onepiece.notReady")}</p>
        </div>
      </section>

      <section aria-labelledby="onepiece-providers-heading">
        <div className="mb-3"><h3 className="font-semibold" id="onepiece-providers-heading">{t("onepiece.providers.title")}</h3><p className="mt-1 text-sm leading-6 text-muted-foreground">{t("onepiece.providers.description")}</p></div>
        {overview.profiles.length ? filteredProfiles.length ? <div className="space-y-3">{filteredProfiles.map((profile) => (
          <article className={`group overflow-hidden rounded-xl border bg-background p-4 transition-all hover:shadow-sm ${profile.active ? "border-primary/55 bg-primary/[0.035] ring-1 ring-primary/10" : "border-border hover:border-primary/30"}`} key={profile.id}>
            <div className="flex flex-col gap-4 sm:flex-row sm:items-center">
              <div className="flex min-w-0 flex-1 items-start gap-3">
                <ProviderBrandIcon iconKey={presets.find((preset) => preset.id === profile.sourceProviderId)?.iconKey ?? "unknown"} label={profile.provider} />
                <div className="min-w-0 flex-1">
                  <div className="flex min-h-6 flex-wrap items-center gap-2">
                    <h4 className="truncate font-semibold">{profile.name}</h4>
                    {profile.active ? <Badge tone="success">{t("onepiece.profileActive")}</Badge> : <Badge tone="muted">{t("onepiece.profileInactive")}</Badge>}
                  </div>
                  <p className="mt-0.5 text-sm text-muted-foreground">{profile.provider} · {profile.interfaceFormat === "anthropic" ? "Anthropic" : "OpenAI-compatible"}</p>
                  <div className="mt-3 grid min-w-0 gap-x-4 gap-y-2 text-xs text-muted-foreground md:grid-cols-2 xl:grid-cols-[minmax(0,1.35fr)_minmax(0,1fr)_auto]">
                    <p className="flex min-w-0 items-center gap-1.5"><Globe2 className="h-3.5 w-3.5 shrink-0" /><span className="truncate" title={profile.baseUrl ?? ""}>{profile.baseUrl || t("onepiece.endpointNotRequired")}</span></p>
                    <p className="flex min-w-0 items-center gap-1.5"><Box className="h-3.5 w-3.5 shrink-0" /><span className="truncate" title={profile.modelId}>{profile.modelId}</span></p>
                    <p className="flex items-center gap-1.5"><KeyRound className="h-3.5 w-3.5 shrink-0" />{profile.credentialPresent ? t("onepiece.credentialConfigured") : t("onepiece.credentialMissing")}</p>
                  </div>
                </div>
              </div>
              <div className="flex shrink-0 flex-wrap items-center gap-2 border-t border-border/60 pt-3 sm:border-l sm:border-t-0 sm:pl-4 sm:pt-0">
                <ProviderCredentialValidation
                  disabled={!profile.credentialPresent || !profile.sourceProviderId || !profile.sourceEndpointType}
                  onValidate={() => service.validateOnePieceProviderCredential({
                    providerId: profile.sourceProviderId ?? "",
                    endpointType: profile.sourceEndpointType ?? "openai-chat-completions",
                    modelId: profile.modelId,
                    profileId: profile.id,
                    apiKey: null,
                  })}
                  resetKey={`${profile.id}:${profile.modelId}:${profile.credentialPresent}`}
                  size="sm"
                />
                {!profile.active ? <Button disabled={!profile.credentialPresent} onClick={() => setPendingAction({ kind: "activate", profile })} size="sm"><Check className="h-4 w-4" />{t("onepiece.activate")}</Button> : null}
                <Button onClick={() => setEditingProfile(profile)} size="sm" variant="outline"><Pencil className="h-4 w-4" />{t("onepiece.editProvider")}</Button>
                <Button onClick={() => setPendingAction({ kind: "delete", profile })} size="sm" variant="ghost"><Trash2 className="h-4 w-4" />{t("onepiece.deleteProfile")}</Button>
              </div>
            </div>
          </article>
        ))}</div> : <div className="rounded-xl border border-dashed border-border bg-muted/20 p-8 text-center"><p className="text-sm leading-6 text-muted-foreground">{t("onepiece.providers.noMatches")}</p></div> : <div className="rounded-xl border border-dashed border-border bg-muted/20 p-8 text-center"><p className="text-sm leading-6 text-muted-foreground">{t("onepiece.providers.empty")}</p><Button className="mt-4" onClick={() => setEditingProfile(null)}><Plus className="h-4 w-4" />{t("onepiece.addProvider")}</Button></div>}
      </section>

      <OnePieceRetrievalSection agentId="onepiece" profiles={overview.profiles} service={service} />

      {operationError ? <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{operationError instanceof Error ? operationError.message : String(operationError)}</p> : null}
      {notice ? <p className="rounded-md border p-3 text-sm ucd-status-success" role="status">{notice}</p> : null}
      {editingProfile !== undefined ? <OnePieceProviderDialog profile={editingProfile} presets={presets} onClose={() => setEditingProfile(undefined)} onSaved={(value) => void updateOverview(value, t("onepiece.saved"))} service={service} /> : null}
      {pendingAction ? <OnePieceProfileActionDialog action={pendingAction} pending={activateMutation.isPending || deleteMutation.isPending} onClose={() => setPendingAction(null)} onConfirm={() => pendingAction.kind === "activate" ? activateMutation.mutate(pendingAction.profile.id) : deleteMutation.mutate(pendingAction.profile.id)} /> : null}
    </div>
  );
}
