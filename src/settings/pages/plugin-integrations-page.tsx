import { CheckCircle2, Plug, RefreshCw, Search, ShieldAlert } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { PluginIntegrationService } from "../../services/plugin-integration-service";
import { pluginIntegrationService } from "../../services/runtime-plugin-integration-client";
import { AsyncBoundary } from "../../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../../ui/async/async-view-state";
import type { MutationState } from "../../ui/async/mutation-state";
import { PageHeader } from "../../ui/page-header/PageHeader";
import type {
  PluginIntegrationDefinition,
  PluginIntegrationOverview,
  PluginIntegrationState,
} from "../../types/plugin-integration";
import { pickPageStatus } from "../settings-page-status";
import type { SettingsPageStatus } from "../settings-page-types";
import { StatCard } from "./page-parts";
import { PluginIntegrationCard } from "./plugins/plugin-integration-card";
import { errorMessage, statusKey } from "./plugins/plugin-integration-utils";

const overviewKey = ["plugin-integrations", "overview"] as const;
const emptyDefinitions: PluginIntegrationDefinition[] = [];
const emptyStates: PluginIntegrationState[] = [];

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
  onStatusChange,
  searchTerm,
  service = pluginIntegrationService,
}: {
  onStatusChange?: (status: SettingsPageStatus | null) => void;
  searchTerm: string;
  service?: PluginIntegrationService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [notice, setNotice] = useState<string | null>(null);
  const overviewQuery = useQuery({ queryKey: overviewKey, queryFn: () => service.getOverview() });
  const testMutation = useMutation({
    mutationFn: (integrationId: PluginIntegrationDefinition["id"]) => service.testReadiness({ integrationId }),
    onSuccess: async (result) => {
      setNotice(t("plugins.notice.testCompleted", { status: t(statusKey(result.status)) }));
      await queryClient.invalidateQueries({ queryKey: overviewKey });
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

  // task 12.18: this page's own overviewQuery projected into the shared AsyncBoundary's
  // AsyncViewState shape -- src/ui/ primitives cannot import this service's own error type
  // (ARCH-FE-005), so the projection lives here rather than in the primitive.
  const asyncState: AsyncViewState<PluginIntegrationOverview> = {
    data: overview,
    error: overviewQuery.isError
      ? { kind: "error", message: errorMessage(overviewQuery.error), retryable: true }
      : undefined,
    initialLoading: overviewQuery.isLoading,
    refreshing: overviewQuery.isFetching && !overviewQuery.isLoading,
    stale: overviewQuery.isStale,
  };

  // Task 12.16: `overviewQuery.isError` is a new condition made real (and now visibly rendered
  // below via AsyncBoundary) by this same migration; `testMutation.isError` is the original,
  // already-tested condition this nav dot reported before -- kept so no prior signal regresses.
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      overviewQuery.isError || testMutation.isError
        ? { kind: "error", labelKey: "plugins.pageStatus.error" }
        : null,
      overview && !nativeChecksAvailable
        ? { kind: "dependency-unavailable", labelKey: "plugins.status.nativeUnavailable" }
        : null,
    ]));
    return () => onStatusChange?.(null);
  }, [nativeChecksAvailable, onStatusChange, overview, overviewQuery.isError, testMutation.isError]);

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

  /** Task 12.18: projects this page's own single-in-flight `useMutation` (react-query already
   *  tracks `variables`/`isPending`/`error` for its own most recent call) into the shared
   *  `MutationState` shape, keyed to one integration at a time -- a second registry alongside
   *  `useMutation` would just be a second source of truth for the same fact. */
  function testStateFor(definition: PluginIntegrationDefinition): MutationState | undefined {
    if (testMutation.variables !== definition.id) return undefined;
    if (testMutation.isPending) return { pending: true, targetKey: definition.id };
    if (testMutation.isError) {
      return { error: { kind: "error", message: errorMessage(testMutation.error), retryable: true }, pending: false, targetKey: definition.id };
    }
    return undefined;
  }

  // Same message whether there are genuinely no built-in integrations yet or a search just
  // matched none, matching this page's own pre-existing behavior (no create action either way --
  // these are fixed built-in integrations, not user-created records).
  const emptyStateSlot = { title: t("plugins.empty") };

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("plugins.description")}
        icon={Plug}
        primaryAction={
          <Button disabled={overviewQuery.isFetching} onClick={() => void overviewQuery.refetch()} variant="outline">
            <RefreshCw aria-hidden="true" className={overviewQuery.isFetching ? "animate-spin" : ""} />
            {overviewQuery.isFetching ? t("plugins.refreshing") : t("plugins.refresh")}
          </Button>
        }
        title={t("plugins.title")}
      />

      {overview && !nativeChecksAvailable ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("plugins.environment.desktopOnly")}</div>
      ) : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      <div className="grid gap-3 md:grid-cols-3">
        <StatCard hint={t("plugins.stats.totalHint")} icon={Plug} label={t("plugins.stats.total")} value={String(definitions.length)} />
        <StatCard hint={t("plugins.stats.configuredHint")} icon={CheckCircle2} label={t("plugins.stats.configured")} value={String(configuredCount)} />
        <StatCard hint={t("plugins.stats.attentionHint")} icon={ShieldAlert} label={t("plugins.stats.attention")} value={String(attentionCount)} />
      </div>

      {searchTerm.trim() ? (
        <div className="flex items-center gap-2 text-xs text-muted-foreground">
          <Search className="h-3.5 w-3.5" aria-hidden="true" />
          {t("plugins.search.active", { term: searchTerm })}
        </div>
      ) : null}

      <AsyncBoundary
        emptyState={emptyStateSlot}
        filtered={Boolean(searchTerm.trim())}
        filteredEmptyState={emptyStateSlot}
        isEmpty={() => visibleDefinitions.length === 0}
        onRetry={() => void overviewQuery.refetch()}
        state={asyncState}
      >
        {() => (
          <div className="grid gap-3">
            {visibleDefinitions.map((definition) => (
              <PluginIntegrationCard
                definition={definition}
                key={definition.id}
                lastResult={testMutation.data}
                nativeChecksAvailable={nativeChecksAvailable}
                onTest={(item) => testMutation.mutate(item.id)}
                state={stateFor(definition)}
                testState={testStateFor(definition)}
              />
            ))}
          </div>
        )}
      </AsyncBoundary>
    </div>
  );
}
