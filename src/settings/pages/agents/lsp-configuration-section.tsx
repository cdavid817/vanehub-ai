import { useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Braces, LoaderCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../../components/ui/button";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import { lspLanguageIds, type LspConfiguration } from "../../../types/lsp";
import { SectionPanel } from "../page-parts";
import {
  createLspConfigurationDraft,
  validateLspConfigurationDraft,
  type LspConfigurationDraft,
  type LspInitializationErrors,
} from "./lsp-configuration-form";
import { LspLanguageConfigurationCard } from "./lsp-language-configuration-card";

export const lspConfigurationQueryKey = ["agents", "lsp-configuration"] as const;
export const lspDiscoveryQueryKey = ["agents", "lsp-discovery"] as const;
export const lspServerStatusQueryKey = ["agents", "lsp-server-status"] as const;

function ConfigurationEditor({
  configuration,
  discoveries,
  discoveryPending,
  onRefreshDiscovery,
  onSave,
  pending,
}: {
  configuration: LspConfiguration;
  discoveries: Awaited<ReturnType<AgentService["discoverLspServers"]>>;
  discoveryPending: boolean;
  onRefreshDiscovery: () => void;
  onSave: (configuration: LspConfiguration) => void;
  pending: boolean;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState<LspConfigurationDraft>(
    () => createLspConfigurationDraft(configuration),
  );
  const [errors, setErrors] = useState<LspInitializationErrors>({});

  useEffect(() => {
    setDraft(createLspConfigurationDraft(configuration));
    setErrors({});
  }, [configuration]);

  function submit(): void {
    const validation = validateLspConfigurationDraft(draft);
    setErrors(validation.errors);
    if (validation.configuration) onSave(validation.configuration);
  }

  return (
    <form className="space-y-5 p-5 sm:p-6" onSubmit={(event) => { event.preventDefault(); submit(); }}>
      <label className="flex items-start gap-3 rounded-lg border border-border bg-muted/15 p-4">
        <input
          checked={draft.enabled}
          className="mt-0.5 h-4 w-4 accent-primary"
          disabled={pending}
          onChange={(event) => setDraft((current) => ({ ...current, enabled: event.target.checked }))}
          type="checkbox"
        />
        <span>
          <span className="block text-sm font-medium">{t("lspSettings.configuration.master")}</span>
          <span className="mt-1 block text-xs leading-5 text-muted-foreground">{t("lspSettings.configuration.masterHint")}</span>
        </span>
      </label>

      <div className="flex flex-wrap items-start justify-between gap-3">
        <div>
          <h4 className="text-sm font-semibold">{t("lspSettings.discovery.title")}</h4>
          <p className="mt-1 text-xs leading-5 text-muted-foreground">{t("lspSettings.discovery.description")}</p>
        </div>
        <Button disabled={discoveryPending || pending} onClick={onRefreshDiscovery} size="sm" type="button" variant="outline">
          <RefreshCw className={discoveryPending ? "animate-spin" : ""} />
          {t("lspSettings.discovery.refresh")}
        </Button>
      </div>

      <div className="grid gap-4 xl:grid-cols-2">
        {lspLanguageIds.map((language) => (
          <LspLanguageConfigurationCard
            discovery={discoveries.find((entry) => entry.language === language)}
            draft={draft.languages[language]}
            errorKey={errors[language]}
            key={language}
            language={language}
            onChange={(languageDraft) => setDraft((current) => ({
              ...current,
              languages: { ...current.languages, [language]: languageDraft },
            }))}
            pending={pending}
          />
        ))}
      </div>

      <div className="flex justify-end">
        <Button disabled={pending} type="submit">
          {pending ? t("lspSettings.configuration.saving") : t("lspSettings.configuration.save")}
        </Button>
      </div>
    </form>
  );
}

export function LspConfigurationSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<string | null>(null);
  const configurationQuery = useQuery({
    queryKey: lspConfigurationQueryKey,
    queryFn: () => service.getLspConfiguration(),
  });
  const discoveryQuery = useQuery({
    queryKey: lspDiscoveryQueryKey,
    queryFn: () => service.discoverLspServers(),
  });
  const saveMutation = useMutation({
    mutationFn: (configuration: LspConfiguration) => service.saveLspConfiguration(configuration),
    onMutate: () => setNotice(null),
    onSuccess: async () => {
      setNotice(t("lspSettings.configuration.saved"));
      await Promise.all([
        queryClient.invalidateQueries({ queryKey: lspConfigurationQueryKey }),
        queryClient.invalidateQueries({ queryKey: lspDiscoveryQueryKey }),
        queryClient.invalidateQueries({ queryKey: lspServerStatusQueryKey }),
      ]);
    },
  });
  const loading = configurationQuery.isLoading || discoveryQuery.isLoading;
  const loadError = configurationQuery.error ?? discoveryQuery.error;

  return (
    <SectionPanel description={t("lspSettings.description")} icon={Braces} title={t("lspSettings.title")} variant="settings">
      {loading ? (
        <div className="flex min-h-32 items-center justify-center gap-2 p-5 text-sm text-muted-foreground">
          <LoaderCircle className="h-4 w-4 animate-spin" />{t("lspSettings.loading")}
        </div>
      ) : null}
      {!loading && loadError ? (
        <div className="p-5">
          <p className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">{t("lspSettings.loadError")}</p>
          <Button className="mt-3" onClick={() => { void configurationQuery.refetch(); void discoveryQuery.refetch(); }} size="sm" variant="outline">{t("lspSettings.retry")}</Button>
        </div>
      ) : null}
      {!loading && !loadError && configurationQuery.data ? (
        <ConfigurationEditor
          configuration={configurationQuery.data}
          discoveries={discoveryQuery.data ?? []}
          discoveryPending={discoveryQuery.isFetching}
          onRefreshDiscovery={() => { void discoveryQuery.refetch(); }}
          onSave={(configuration) => saveMutation.mutate(configuration)}
          pending={saveMutation.isPending}
        />
      ) : null}
      {saveMutation.error ? <p className="mx-5 mb-5 rounded-md border p-3 text-sm ucd-status-danger" role="alert">{t("lspSettings.configuration.saveError")}</p> : null}
      {notice ? <p className="mx-5 mb-5 rounded-md border p-3 text-sm ucd-status-success" role="status">{notice}</p> : null}
    </SectionPanel>
  );
}
