import { CheckCircle2, ExternalLink, Github, Plug, RefreshCw, Search, ShieldAlert } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import type { PluginIntegrationService } from "../../services/plugin-integration-service";
import { pluginIntegrationService } from "../../services/runtime-plugin-integration-client";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationState,
  PluginIntegrationStatus,
  PluginIntegrationTestResult,
} from "../../types/plugin-integration";
import { PageHeader, SectionPanel, StatCard } from "./page-parts";

const overviewKey = ["plugin-integrations", "overview"] as const;
const emptyDefinitions: PluginIntegrationDefinition[] = [];
const emptyStates: PluginIntegrationState[] = [];

function statusKey(status: PluginIntegrationStatus) {
  return `plugins.status.${status}`;
}

function statusTone(status: PluginIntegrationStatus): "success" | "warning" | "danger" | "muted" {
  if (status === "configured") return "success";
  if (status === "not-configured" || status === "missing-cli" || status === "unavailable") return "warning";
  return "danger";
}

export function filterPluginIntegrations(
  definitions: PluginIntegrationDefinition[],
  states: PluginIntegrationState[],
  searchTerm: string,
  translate: (key: string) => string,
) {
  const query = searchTerm.trim().toLowerCase();
  if (!query) return definitions;
  return definitions.filter((definition) => {
    const state = states.find((item) => item.integrationId === definition.id);
    const values = [
      definition.id,
      definition.provider,
      definition.version,
      translate(definition.nameKey),
      translate(definition.descriptionKey),
      ...definition.setupSteps.map((step) => translate(step.labelKey)),
      state ? translate(statusKey(state.status)) : "",
      state?.statusReasonKey ? translate(state.statusReasonKey) : "",
    ];
    return values.some((value) => value.toLowerCase().includes(query));
  });
}

export function PluginIntegrationsPage({
  searchTerm,
  service = pluginIntegrationService,
}: {
  searchTerm: string;
  service?: PluginIntegrationService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const overviewQuery = useQuery({ queryKey: overviewKey, queryFn: () => service.getOverview() });
  const testMutation = useMutation({
    mutationFn: (definition: PluginIntegrationDefinition) => service.testReadiness({ integrationId: definition.id }),
    onSuccess: async (result) => {
      setNotice(t("plugins.notice.testCompleted", { status: t(statusKey(result.status)) }));
      setError(null);
      await queryClient.invalidateQueries({ queryKey: overviewKey });
    },
    onError: (reason) => {
      setNotice(null);
      setError(reason instanceof Error ? reason.message : String(reason));
    },
  });

  const overview = overviewQuery.data;
  const definitions = overview?.definitions ?? emptyDefinitions;
  const states = overview?.states ?? emptyStates;
  const visibleDefinitions = useMemo(
    () => filterPluginIntegrations(definitions, states, searchTerm, t),
    [definitions, states, searchTerm, t],
  );
  const configuredCount = states.filter((state) => state.configured).length;
  const attentionCount = states.filter((state) => state.status !== "configured").length;
  const nativeChecksAvailable = overview?.environment.nativeChecksAvailable === true;

  async function refresh() {
    setError(null);
    setNotice(null);
    await overviewQuery.refetch();
  }

  function stateFor(definition: PluginIntegrationDefinition): PluginIntegrationState {
    return (
      states.find((state) => state.integrationId === definition.id) ?? {
        integrationId: definition.id,
        status: "not-configured",
        configured: false,
        canTest: nativeChecksAvailable,
        lastCheckedAt: null,
        statusReasonKey: "plugins.statusReason.notChecked",
        message: null,
      }
    );
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button disabled={overviewQuery.isFetching} onClick={() => void refresh()} variant="outline">
            <RefreshCw className={overviewQuery.isFetching ? "animate-spin" : ""} />
            {overviewQuery.isFetching ? t("plugins.refreshing") : t("plugins.refresh")}
          </Button>
        }
        description={t("plugins.description")}
        icon={Plug}
        title={t("plugins.title")}
      />

      {overview && !nativeChecksAvailable ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("plugins.environment.desktopOnly")}</div>
      ) : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}
      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}

      <div className="grid gap-3 md:grid-cols-3">
        <StatCard hint={t("plugins.stats.totalHint")} icon={Plug} label={t("plugins.stats.total")} value={String(definitions.length)} />
        <StatCard hint={t("plugins.stats.configuredHint")} icon={CheckCircle2} label={t("plugins.stats.configured")} value={String(configuredCount)} />
        <StatCard hint={t("plugins.stats.attentionHint")} icon={ShieldAlert} label={t("plugins.stats.attention")} value={String(attentionCount)} />
      </div>

      <SectionPanel description={t("plugins.list.description")} title={t("plugins.list.title")}>
        {overviewQuery.isLoading ? <div className="text-sm text-muted-foreground">{t("plugins.loading")}</div> : null}
        {searchTerm.trim() ? (
          <div className="mb-3 flex items-center gap-2 text-xs text-muted-foreground">
            <Search className="h-3.5 w-3.5" aria-hidden="true" />
            {t("plugins.search.active", { term: searchTerm })}
          </div>
        ) : null}
        <div className="grid gap-3">{visibleDefinitions.map((definition) => renderCard(definition, stateFor(definition), nativeChecksAvailable, testMutation.data, testMutation.isPending && testMutation.variables?.id === definition.id, () => testMutation.mutate(definition), t))}</div>
        {!overviewQuery.isLoading && visibleDefinitions.length === 0 ? <div className="text-sm text-muted-foreground">{t("plugins.empty")}</div> : null}
      </SectionPanel>
    </div>
  );
}

function renderCard(
  definition: PluginIntegrationDefinition,
  state: PluginIntegrationState,
  nativeChecksAvailable: boolean,
  lastResult: PluginIntegrationTestResult | undefined,
  testing: boolean,
  onTest: () => void,
  t: (key: string, options?: Record<string, string>) => string,
) {
  const messageKey = lastResult?.integrationId === definition.id ? lastResult.message : state.statusReasonKey;
  return (
    <article className="ucd-panel ucd-interactive rounded-lg p-4" data-testid={`plugin-card-${definition.id}`} key={definition.id}>
      <div className="flex flex-wrap items-start justify-between gap-3">
        <div className="flex min-w-0 items-start gap-3">
          <span className="flex h-10 w-10 shrink-0 items-center justify-center rounded-md border border-border bg-[hsl(var(--panel-muted))] text-foreground">
            <Github className="h-5 w-5" aria-hidden="true" />
          </span>
          <div className="min-w-0">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-base font-semibold">{t(definition.nameKey)}</h3>
              <Badge tone="muted">v{definition.version}</Badge>
            </div>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
          </div>
        </div>
        <Badge tone={statusTone(state.status)}>{t(statusKey(state.status))}</Badge>
      </div>

      <div className="mt-4 grid gap-3 md:grid-cols-[minmax(0,1fr)_auto]">
        <div className="space-y-2">
          {definition.setupSteps.map((step, index) => (
            <div className="flex gap-2 text-sm" key={step.id}>
              <span className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-sm bg-muted text-xs font-semibold text-muted-foreground">{index + 1}</span>
              <span className="text-muted-foreground">{t(step.labelKey)}</span>
            </div>
          ))}
          {messageKey ? <div className="text-xs text-muted-foreground">{t(messageKey)}</div> : null}
          {state.lastCheckedAt ? <div className="text-xs text-muted-foreground">{t("plugins.lastChecked", { time: state.lastCheckedAt })}</div> : null}
        </div>
        <div className="flex flex-wrap items-start justify-end gap-2">
          <Button asChild size="sm" variant="outline">
            <a href={definition.docsUrl} rel="noreferrer" target="_blank">
              <ExternalLink />
              {t("plugins.action.docs")}
            </a>
          </Button>
          <Button disabled={!nativeChecksAvailable || !state.canTest || testing} onClick={onTest} size="sm">
            <CheckCircle2 />
            {testing ? t("plugins.action.testing") : t("plugins.action.test")}
          </Button>
        </div>
      </div>
    </article>
  );
}
