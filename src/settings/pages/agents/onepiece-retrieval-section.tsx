import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LoaderCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { ApplicationDialog } from "../../../components/ui/application-dialog";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { OnePieceProviderProfile } from "../../../types/agent";

const inputClass = "ucd-input h-9 rounded px-3 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";
const configKey = ["agents", "onepiece-retrieval-configuration"] as const;
// Status and rebuild are global, like the configuration singleton they sit next to: retrieval
// applies to every agent, so there is nothing per-agent to key this cache entry by.
const statusKey = ["agents", "onepiece-retrieval-status"] as const;

// Design doc §8.2: the backend only ever returns a category, never raw provider error text (which
// may echo credentials or request content). Rendering goes through this fixed lookup — the
// category value itself is never interpolated into the DOM, even for an unrecognized category.
const failureCategoryKeys: Record<string, string> = {
  auth: "onepiece.retrieval.failureCategory.auth",
  invalid_request: "onepiece.retrieval.failureCategory.invalidRequest",
  rate_limit: "onepiece.retrieval.failureCategory.rateLimit",
  network: "onepiece.retrieval.failureCategory.network",
};

export function OnePieceRetrievalSection({ profiles, service = defaultAgentService }: {
  profiles: OnePieceProviderProfile[];
  service?: AgentService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  // Only openai-compatible profiles can serve as an embedding source (Anthropic has no embeddings API).
  const sourceProfiles = profiles.filter((profile) => profile.interfaceFormat === "openai-compatible");
  const [profileOverride, setProfileOverride] = useState<string | null | undefined>(undefined);
  const [modelOverride, setModelOverride] = useState<string | null | undefined>(undefined);
  const [confirmingRebuild, setConfirmingRebuild] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);

  const configQuery = useQuery({ queryKey: configKey, queryFn: () => service.getRetrievalConfiguration() });
  const statusQuery = useQuery({ queryKey: statusKey, queryFn: () => service.getRetrievalIndexStatus() });
  const configuration = configQuery.data;
  // Selection follows the saved configuration until the user picks something different locally.
  const selectedProfileId = profileOverride !== undefined ? profileOverride : (configuration?.sourceProfileId ?? null);
  const selectedModelId = modelOverride !== undefined ? modelOverride : (configuration?.embeddingModel ?? null);
  const modelsQuery = useQuery({
    queryKey: ["agents", "onepiece-retrieval-models", selectedProfileId] as const,
    queryFn: () => {
      if (!selectedProfileId) throw new Error("No source Profile selected.");
      return service.listEmbeddingModels(selectedProfileId);
    },
    enabled: Boolean(selectedProfileId),
  });

  const saveMutation = useMutation({
    mutationFn: () => {
      if (!selectedProfileId || !selectedModelId) throw new Error("Select a source Profile and embedding model first.");
      return service.saveRetrievalConfiguration(selectedProfileId, selectedModelId);
    },
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: configKey });
      setProfileOverride(undefined);
      setModelOverride(undefined);
      setNotice(t("onepiece.retrieval.saved"));
    },
  });
  const rebuildMutation = useMutation({
    mutationFn: () => service.rebuildRetrievalIndex(),
    onSuccess: () => {
      void queryClient.invalidateQueries({ queryKey: statusKey });
      setConfirmingRebuild(false);
    },
  });

  const operationError = configQuery.error ?? statusQuery.error ?? modelsQuery.error ?? saveMutation.error ?? rebuildMutation.error;
  const isConfigured = Boolean(configuration?.sourceProfileId && configuration?.embeddingModel);

  if (configQuery.isLoading || statusQuery.isLoading) {
    return <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  const status = statusQuery.data ?? { indexed: 0, pending: 0, failed: 0, lastFailureCategory: null };

  return (
    <section aria-labelledby="onepiece-retrieval-heading" className="space-y-4 rounded-xl border border-border bg-muted/10 p-3 sm:p-4">
      <div><h3 className="text-sm font-semibold" id="onepiece-retrieval-heading">{t("onepiece.retrieval.title")}</h3><p className="mt-1 text-xs leading-5 text-muted-foreground">{t("onepiece.retrieval.description")}</p></div>

      {sourceProfiles.length === 0 ? <p className="text-sm ucd-status-warning">{t("onepiece.retrieval.noSourceProfile")}</p> : (
        <div className="grid gap-4 sm:grid-cols-2">
          <label className="flex flex-col gap-1 text-sm">{t("onepiece.retrieval.sourceProfile")}
            <select aria-label={t("onepiece.retrieval.sourceProfile")} className={inputClass} onChange={(event) => { setProfileOverride(event.target.value || null); setModelOverride(null); }} value={selectedProfileId ?? ""}>
              <option value="">{t("onepiece.retrieval.selectProfilePlaceholder")}</option>
              {sourceProfiles.map((profile) => <option key={profile.id} value={profile.id}>{profile.name}</option>)}
            </select>
          </label>
          {selectedProfileId ? (
            <label className="flex flex-col gap-1 text-sm">{t("onepiece.retrieval.embeddingModel")}
              <select aria-label={t("onepiece.retrieval.embeddingModel")} className={inputClass} disabled={modelsQuery.isLoading} onChange={(event) => setModelOverride(event.target.value || null)} value={selectedModelId ?? ""}>
                <option value="">{t("onepiece.retrieval.selectModelPlaceholder")}</option>
                {(modelsQuery.data ?? []).map((model) => <option key={model.id} value={model.id}>{model.displayName}</option>)}
              </select>
            </label>
          ) : null}
          <div className="sm:col-span-2">
            <Button disabled={!selectedProfileId || !selectedModelId || saveMutation.isPending} onClick={() => saveMutation.mutate()}>{saveMutation.isPending ? t("agents.edit.saving") : t("onepiece.retrieval.save")}</Button>
          </div>
        </div>
      )}

      <div className="border-t border-border pt-3">
        <div className="flex flex-wrap items-center justify-between gap-2">
          <h4 className="text-sm font-semibold">{t("onepiece.retrieval.status.title")}</h4>
          <Button disabled={!isConfigured || rebuildMutation.isPending} onClick={() => setConfirmingRebuild(true)} size="sm" variant="outline"><RefreshCw className={`h-4 w-4 ${rebuildMutation.isPending ? "animate-spin" : ""}`} />{t("onepiece.retrieval.rebuild")}</Button>
        </div>
        <div className="mt-3 grid grid-cols-3 gap-3 text-center">
          <div className="rounded-lg border border-border bg-background p-3"><p className="text-xs text-muted-foreground">{t("onepiece.retrieval.status.indexed")}</p><p className="text-lg font-semibold">{status.indexed}</p></div>
          <div className="rounded-lg border border-border bg-background p-3"><p className="text-xs text-muted-foreground">{t("onepiece.retrieval.status.pending")}</p><p className="text-lg font-semibold">{status.pending}</p></div>
          <div className="rounded-lg border border-border bg-background p-3"><p className="text-xs text-muted-foreground">{t("onepiece.retrieval.status.failed")}</p><p className="text-lg font-semibold">{status.failed}</p></div>
        </div>
        {status.lastFailureCategory ? <p className="mt-2 text-xs ucd-status-warning">{t("onepiece.retrieval.status.lastFailure")}: {t(failureCategoryKeys[status.lastFailureCategory] ?? "onepiece.retrieval.failureCategory.unknown")}</p> : null}
      </div>

      {operationError ? <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{operationError instanceof Error ? operationError.message : String(operationError)}</p> : null}
      {notice ? <p className="rounded-md border p-3 text-sm ucd-status-success" role="status">{notice}</p> : null}

      {confirmingRebuild ? (
        <ApplicationDialog closeDisabled={rebuildMutation.isPending} description={t("onepiece.retrieval.rebuildConfirm.description")} onClose={() => setConfirmingRebuild(false)} title={t("onepiece.retrieval.rebuildConfirm.title")}>
          <div className="mt-2 flex flex-col-reverse gap-2 sm:flex-row sm:justify-end">
            <Button disabled={rebuildMutation.isPending} onClick={() => setConfirmingRebuild(false)} variant="outline">{t("agents.edit.cancel")}</Button>
            <Button data-dialog-autofocus disabled={rebuildMutation.isPending} onClick={() => rebuildMutation.mutate()}>{rebuildMutation.isPending ? t("agentConfigurations.dialog.pending") : t("onepiece.retrieval.rebuildConfirm.confirm")}</Button>
          </div>
        </ApplicationDialog>
      ) : null}
    </section>
  );
}
