import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { ArrowUpCircle, CheckCircle2, RefreshCw, Stethoscope, TerminalSquare } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { orderByAgentPriority } from "../../lib/agent-display-order";
import { agentService } from "../../services/runtime-agent-client";
import { operationService } from "../../services/runtime-operation-client";
import { settingsService } from "../../services/runtime-settings-client";
import type { CliEnvironmentSnapshot } from "../../types/cli-environment-snapshot";
import type { OperationTask } from "../../types/operation";
import { bulkUpgradeEligible, recommendedSourceId, targetVersionOptions } from "./cli-action-selection";
import { CliEnvironmentCard } from "./cli-environment-card";
import { PageHeader, StatCard } from "./page-parts";

const cliEnvironmentsQueryKey = ["cli-environments"] as const;

export function isOperationRunning(operation?: OperationTask) {
  return operation?.status === "running" || operation?.status === "queued";
}

export function refreshButtonState(isPending: boolean, operation?: OperationTask) {
  const running = isPending || isOperationRunning(operation);
  return {
    disabled: running,
    labelKey: running ? "cli.refreshing" : "cli.refresh",
    iconClassName: `h-4 w-4 ${running ? "animate-spin" : ""}`,
  };
}

export function ProvidersPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({});
  const [expandedDiagnostics, setExpandedDiagnostics] = useState<Record<string, boolean>>({});
  const [expandedLogs, setExpandedLogs] = useState<Record<string, boolean>>({});
  const [activeOperationIds, setActiveOperationIds] = useState<Record<string, string>>({});
  const [mutatingAgentIds, setMutatingAgentIds] = useState<Record<string, string>>({});
  const [refreshOperationId, setRefreshOperationId] = useState<string | null>(null);

  const snapshotsQuery = useQuery({
    queryKey: cliEnvironmentsQueryKey,
    queryFn: () => agentService.listCliEnvironments(),
    // Cached data stays on screen while a background refresh runs; a blank card would read as
    // "nothing is installed" for as long as the probe takes.
    placeholderData: (previous) => previous,
  });
  const snapshots = useMemo(
    () => orderByAgentPriority(snapshotsQuery.data ?? [], (snapshot) => snapshot.agentId),
    [snapshotsQuery.data],
  );

  useEffect(() => {
    setSelectedVersions((current) => {
      const next = { ...current };
      for (const snapshot of snapshots) {
        // The backend's own default for the offered action. Never a version this page picked.
        if (!next[snapshot.agentId]) {
          next[snapshot.agentId] = snapshot.allowedActions[0]?.defaultTarget
            ?? targetVersionOptions(snapshot)[0]
            ?? "";
        }
      }
      return next;
    });
  }, [snapshots]);

  const operationIds = useMemo(
    () => [...new Set([
      ...snapshots.flatMap((snapshot) => snapshot.lastOperationId ? [snapshot.lastOperationId] : []),
      ...Object.values(activeOperationIds),
      ...Object.values(mutatingAgentIds),
      ...(refreshOperationId ? [refreshOperationId] : []),
    ])],
    [activeOperationIds, mutatingAgentIds, refreshOperationId, snapshots],
  );
  const operationQueries = useQueries({
    queries: operationIds.map((operationId) => ({
      queryKey: ["operation", operationId],
      queryFn: () => operationService.getOperationStatus(operationId),
      refetchInterval: (query: { state: { data?: OperationTask } }) => isOperationRunning(query.state.data) ? 1200 : false,
    })),
  });
  const operationsById = useMemo(() => {
    const entries: Array<[string, OperationTask]> = [];
    operationQueries.forEach((query, index) => {
      if (query.data) entries.push([operationIds[index], query.data]);
    });
    return Object.fromEntries(entries);
  }, [operationIds, operationQueries]);

  useEffect(() => {
    const finishedIds = new Set(
      operationIds.filter((operationId) => operationsById[operationId] && !isOperationRunning(operationsById[operationId])),
    );
    if (finishedIds.size === 0) return;
    setActiveOperationIds((current) => Object.fromEntries(Object.entries(current).filter(([, id]) => !finishedIds.has(id))));
    setMutatingAgentIds((current) => Object.fromEntries(Object.entries(current).filter(([, id]) => !finishedIds.has(id))));
    if (refreshOperationId && finishedIds.has(refreshOperationId)) setRefreshOperationId(null);
    // Only the CLI environment list; an unrelated query has no reason to refetch because a CLI
    // operation ended.
    void queryClient.invalidateQueries({ queryKey: cliEnvironmentsQueryKey });
  }, [operationIds, operationsById, queryClient, refreshOperationId]);

  function reportCliStartFailure(source: string, error: unknown, details?: Record<string, string>) {
    void settingsService.reportClientLogEvent({ level: "error", kind: "critical-operation-failure", message: String(error), source, details });
  }

  const refreshMutation = useMutation({
    mutationFn: (agentId: string | null) =>
      agentService.refreshCliEnvironments(agentId ? [agentId] : [], false),
    onSuccess: (operation, agentId) => {
      if (agentId) {
        setActiveOperationIds((current) => ({ ...current, [agentId]: operation.id }));
      } else {
        setRefreshOperationId(operation.id);
        setActiveOperationIds(Object.fromEntries(snapshots.map((snapshot) => [snapshot.agentId, operation.id])));
      }
    },
    onError: (error, agentId) => reportCliStartFailure("ProvidersPage.refreshCliEnvironments", error, agentId ? { agentId } : undefined),
  });

  const prepareActionMutation = useMutation({
    mutationFn: ({ snapshot, targetVersion }: { snapshot: CliEnvironmentSnapshot; targetVersion: string }) =>
      agentService.prepareCliAction({
        agentId: snapshot.agentId,
        // No action: the backend derives install/upgrade/downgrade from the target and what is
        // installed, so this page never compares two versions.
        action: null,
        sourceId: recommendedSourceId(snapshot) ?? "",
        // Exactly what the user selected. Substituting a "latest" here is the defect this replaces.
        targetVersion,
        channel: null,
      }),
    onSuccess: (operation, variables) => {
      setActiveOperationIds((current) => ({ ...current, [variables.snapshot.agentId]: operation.id }));
      setMutatingAgentIds((current) => ({ ...current, [variables.snapshot.agentId]: operation.id }));
    },
    onError: (error, variables) => reportCliStartFailure("ProvidersPage.prepareCliAction", error, {
      agentId: variables.snapshot.agentId,
      targetVersion: variables.targetVersion,
    }),
  });

  const bulkUpgradeMutation = useMutation({
    mutationFn: (agentIds: string[]) => agentService.prepareCliBulkUpgrade(agentIds),
    onSuccess: (operation, agentIds) => {
      setActiveOperationIds(Object.fromEntries(agentIds.map((agentId) => [agentId, operation.id])));
      setMutatingAgentIds(Object.fromEntries(agentIds.map((agentId) => [agentId, operation.id])));
    },
    onError: (error) => reportCliStartFailure("ProvidersPage.prepareCliBulkUpgrade", error),
  });

  const filtered = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return snapshots;
    return snapshots.filter((snapshot) => [snapshot.displayName, snapshot.provider, ...snapshot.executableNames]
      .some((value) => value?.toLowerCase().includes(query)));
  }, [searchTerm, snapshots]);
  const installedCount = snapshots.filter((snapshot) => snapshot.installations.length > 0).length;
  const bulkEligible = snapshots.filter(bulkUpgradeEligible);
  const refreshOperation = refreshOperationId ? operationsById[refreshOperationId] : undefined;
  const refreshState = refreshButtonState(refreshMutation.isPending && refreshMutation.variables === null, refreshOperation);

  function diagnoseInstallConflicts() {
    setExpandedDiagnostics(Object.fromEntries(snapshots.map((snapshot) => [snapshot.agentId, true])));
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={<div className="flex flex-wrap gap-2">
          <Button variant="outline" onClick={diagnoseInstallConflicts}>
            <Stethoscope className="h-4 w-4" aria-hidden="true" />{t("cli.diagnoseConflicts")}
          </Button>
          <Button disabled={refreshState.disabled} variant="outline" onClick={() => refreshMutation.mutate(null)}>
            <RefreshCw className={refreshState.iconClassName} aria-hidden="true" />{t(refreshState.labelKey)}
          </Button>
          <Button
            disabled={bulkUpgradeMutation.isPending || bulkEligible.length === 0}
            onClick={() => bulkUpgradeMutation.mutate(bulkEligible.map((snapshot) => snapshot.agentId))}
          >
            <ArrowUpCircle className={bulkUpgradeMutation.isPending ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
            {t("cli.upgradeAll", { count: bulkEligible.length })}
          </Button>
        </div>}
        description={t("cli.description")}
        icon={TerminalSquare}
        title={t("cli.title")}
      />
      <section className="ucd-panel rounded-lg p-3">
        <h2 className="text-sm font-semibold">{t("cli.localEnvironmentCheck")}</h2>
        <p className="mt-1 text-xs text-muted-foreground">{t("cli.localEnvironmentHint")}</p>
      </section>
      <div data-testid="cli-installation-summary">
        <StatCard icon={CheckCircle2} label={t("cli.stats.installed")} value={`${installedCount} / ${snapshots.length}`} hint={t("cli.stats.installedHint")} />
      </div>
      {snapshotsQuery.error ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{String(snapshotsQuery.error)}</div> : null}
      <div className="grid gap-4 xl:grid-cols-2">
        {filtered.map((snapshot) => {
          const operationId = activeOperationIds[snapshot.agentId] ?? snapshot.lastOperationId;
          const operation = operationId ? operationsById[operationId] : undefined;
          const mutating = Boolean(mutatingAgentIds[snapshot.agentId] && (!operation || isOperationRunning(operation)));
          const refreshing = refreshMutation.isPending && refreshMutation.variables === snapshot.agentId
            || Boolean(operation && isOperationRunning(operation) && !mutatingAgentIds[snapshot.agentId]);
          return <CliEnvironmentCard
            key={snapshot.agentId}
            snapshot={snapshot}
            selectedVersion={selectedVersions[snapshot.agentId] ?? ""}
            operation={operation}
            diagnosticsExpanded={Boolean(expandedDiagnostics[snapshot.agentId])}
            operationExpanded={Boolean(expandedLogs[snapshot.agentId])}
            refreshing={refreshing}
            // Per tool, not global: one tool's mutation has no bearing on another's buttons.
            mutating={mutating || prepareActionMutation.isPending}
            onSelectedVersionChange={(version) => setSelectedVersions((current) => ({ ...current, [snapshot.agentId]: version }))}
            onRefresh={() => refreshMutation.mutate(snapshot.agentId)}
            onRequestChange={(targetVersion) => prepareActionMutation.mutate({ snapshot, targetVersion })}
            onToggleDiagnostics={() => setExpandedDiagnostics((current) => ({ ...current, [snapshot.agentId]: !current[snapshot.agentId] }))}
            onToggleOperation={() => setExpandedLogs((current) => ({ ...current, [snapshot.agentId]: !current[snapshot.agentId] }))}
          />;
        })}
      </div>
    </div>
  );
}
