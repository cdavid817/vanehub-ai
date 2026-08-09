import { useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type { OnePieceProviderProfile } from "../../../types/agent";
import type { CodeIndexAutomaticMode } from "../../../types/code-index";

const inputClass = "min-h-9 w-full rounded-md border border-border bg-background px-3 py-2 text-sm focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring";
const configKey = ["agents", "onepiece-retrieval-configuration"] as const;

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
  const [automaticModeOverride, setAutomaticModeOverride] = useState<CodeIndexAutomaticMode>();
  const [notice, setNotice] = useState<string | null>(null);

  const configQuery = useQuery({ queryKey: configKey, queryFn: () => service.getRetrievalConfiguration() });
  const configuration = configQuery.data;
  const selectedAutomaticMode = automaticModeOverride ?? configuration?.automaticCodeIndexMode;
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
  const policyMutation = useMutation({
    mutationFn: (mode: CodeIndexAutomaticMode) => service.saveCodeIndexAutomaticMode(mode),
    onSuccess: async () => {
      await queryClient.invalidateQueries({ queryKey: configKey });
      setAutomaticModeOverride(undefined);
    },
    onError: () => setAutomaticModeOverride(undefined),
  });
  const operationError = configQuery.error ?? modelsQuery.error ?? saveMutation.error ?? policyMutation.error;

  if (configQuery.isLoading) {
    return <div className="flex min-h-24 items-center justify-center gap-2 text-sm text-muted-foreground"><LoaderCircle className="h-4 w-4 animate-spin" />{t("agents.globalConfig.loading")}</div>;
  }

  return (
    <section aria-label={t("onepiece.retrieval.title")} className="space-y-4">
      <section className="ucd-panel ucd-interactive rounded-lg p-4">
        <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)] md:items-start">
          <div>
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-sm font-semibold">{t("onepiece.retrieval.automaticMode.label")}</h3>
              <Badge tone="muted">OnePiece</Badge>
            </div>
            <p className="mt-2 text-sm leading-6 text-muted-foreground">{t("onepiece.retrieval.automaticMode.description")}</p>
            {selectedAutomaticMode ? <p className="mt-2 text-xs leading-5 text-muted-foreground">{t(`onepiece.retrieval.automaticMode.${selectedAutomaticMode}.description`)}</p> : null}
          </div>
          <label className="flex flex-col gap-2 text-sm">
            <span className="font-medium">{t("onepiece.retrieval.automaticMode.label")}</span>
            <select
              aria-label={t("onepiece.retrieval.automaticMode.label")}
              className={inputClass}
              disabled={policyMutation.isPending}
              onChange={(event) => {
                const mode = event.currentTarget.value as CodeIndexAutomaticMode;
                setAutomaticModeOverride(mode);
                policyMutation.mutate(mode);
              }}
              value={selectedAutomaticMode ?? "disabled"}
            >
              {(["disabled", "local", "semantic"] as const).map((mode) => <option key={mode} value={mode}>{t(`onepiece.retrieval.automaticMode.${mode}.label`)}</option>)}
            </select>
          </label>
        </div>
      </section>

      {selectedAutomaticMode === "semantic" ? (
        <section className="ucd-panel ucd-interactive rounded-lg p-4">
          <div className="grid gap-4 md:grid-cols-[minmax(0,1fr)_minmax(220px,320px)] md:items-start">
            <div>
              <div className="flex flex-wrap items-center gap-2">
                <h3 className="text-sm font-semibold">{t("onepiece.retrieval.title")}</h3>
                <Badge tone="muted">Embedding</Badge>
              </div>
              <p className="mt-2 text-sm leading-6 text-muted-foreground">{t("onepiece.retrieval.description")}</p>
              {sourceProfiles.length === 0 ? <p className="mt-2 text-sm ucd-status-warning">{t("onepiece.retrieval.noSourceProfile")}</p> : null}
            </div>
            {sourceProfiles.length > 0 ? <div className="space-y-3">
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
              <Button disabled={!selectedProfileId || !selectedModelId || saveMutation.isPending} onClick={() => saveMutation.mutate()}>{saveMutation.isPending ? t("agents.edit.saving") : t("onepiece.retrieval.save")}</Button>
            </div> : null}
          </div>
        </section>
      ) : null}

      {operationError ? <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{operationError instanceof Error ? operationError.message : String(operationError)}</p> : null}
      {notice ? <p className="rounded-md border p-3 text-sm ucd-status-success" role="status">{notice}</p> : null}
    </section>
  );
}
