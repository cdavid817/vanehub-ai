import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, RefreshCw, TerminalSquare, XCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import { operationService } from "../../services/runtime-operation-client";
import { settingsService } from "../../services/runtime-settings-client";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";
import { deriveCliVersionAction } from "./cli-management-utils";
import { CliConflictDialog } from "./cli-conflict-dialog";
import { CliEnvironmentCard } from "./cli-environment-card";
import { PageHeader, StatCard } from "./page-parts";

const cliToolsQueryKey = ["cli-tools"] as const;

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

type PendingPackageAction = {
  tool: CliToolStatus;
  targetVersion: string;
};

export function ProvidersPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({});
  const [expandedDiagnostics, setExpandedDiagnostics] = useState<Record<string, boolean>>({});
  const [expandedLogs, setExpandedLogs] = useState<Record<string, boolean>>({});
  const [activeOperationIds, setActiveOperationIds] = useState<Record<string, string>>({});
  const [packageOperationIds, setPackageOperationIds] = useState<Record<string, string>>({});
  const [refreshOperationId, setRefreshOperationId] = useState<string | null>(null);
  const [pendingPackageAction, setPendingPackageAction] = useState<PendingPackageAction | null>(null);

  const toolsQuery = useQuery({ queryKey: cliToolsQueryKey, queryFn: () => agentService.listCliTools() });
  const tools = toolsQuery.data ?? [];

  useEffect(() => {
    setSelectedVersions((current) => {
      const next = { ...current };
      for (const tool of tools) {
        if (!next[tool.agentId]) next[tool.agentId] = tool.latestVersion ?? tool.availableVersions[0] ?? tool.currentVersion ?? "";
      }
      return next;
    });
  }, [tools]);

  const operationIds = useMemo(
    () => [...new Set([
      ...tools.flatMap((tool) => tool.lastOperationId ? [tool.lastOperationId] : []),
      ...Object.values(activeOperationIds),
      ...Object.values(packageOperationIds),
      ...(refreshOperationId ? [refreshOperationId] : []),
    ])],
    [activeOperationIds, packageOperationIds, refreshOperationId, tools],
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
    setPackageOperationIds((current) => Object.fromEntries(Object.entries(current).filter(([, id]) => !finishedIds.has(id))));
    if (refreshOperationId && finishedIds.has(refreshOperationId)) setRefreshOperationId(null);
    void queryClient.invalidateQueries({ queryKey: cliToolsQueryKey });
  }, [operationIds, operationsById, queryClient, refreshOperationId]);

  function reportCliStartFailure(source: string, error: unknown, details?: Record<string, string>) {
    void settingsService.reportClientLogEvent({ level: "error", kind: "critical-operation-failure", message: String(error), source, details });
  }

  const refreshMutation = useMutation({
    mutationFn: (agentId: string | null) => agentService.refreshCliDetections(agentId ?? undefined),
    onSuccess: (operation, agentId) => {
      if (agentId) {
        setActiveOperationIds((current) => ({ ...current, [agentId]: operation.id }));
      } else {
        setRefreshOperationId(operation.id);
        setActiveOperationIds(Object.fromEntries(tools.map((tool) => [tool.agentId, operation.id])));
      }
    },
    onError: (error, agentId) => reportCliStartFailure("ProvidersPage.refreshCliDetections", error, agentId ? { agentId } : undefined),
  });

  const installMutation = useMutation({
    mutationFn: ({ tool, targetVersion, confirmedActivePath }: PendingPackageAction & { confirmedActivePath?: string | null }) =>
      agentService.installCliVersion({ agentId: tool.agentId, targetVersion, confirmedActivePath }),
    onSuccess: (operation, variables) => {
      setActiveOperationIds((current) => ({ ...current, [variables.tool.agentId]: operation.id }));
      setPackageOperationIds((current) => ({ ...current, [variables.tool.agentId]: operation.id }));
    },
    onError: (error, variables) => reportCliStartFailure("ProvidersPage.installCliVersion", error, {
      agentId: variables.tool.agentId,
      targetVersion: variables.targetVersion,
    }),
  });

  const filteredTools = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return tools;
    return tools.filter((tool) => [tool.displayName, tool.provider, tool.executableName, tool.packageName].some((value) => value.toLowerCase().includes(query)));
  }, [searchTerm, tools]);
  const installedCount = tools.filter((tool) => tool.installed === true).length;
  const missingCount = tools.filter((tool) => tool.installed === false).length;
  const packageBusy = installMutation.isPending || Object.values(packageOperationIds).some((id) => !operationsById[id] || isOperationRunning(operationsById[id]));
  const refreshOperation = refreshOperationId ? operationsById[refreshOperationId] : undefined;
  const refreshState = refreshButtonState(refreshMutation.isPending && refreshMutation.variables === null, refreshOperation);

  function requestPackageAction(tool: CliToolStatus, targetVersion: string) {
    if (tool.installations.length > 1) {
      setPendingPackageAction({ tool, targetVersion });
      return;
    }
    installMutation.mutate({ tool, targetVersion });
  }

  function confirmPackageAction() {
    if (!pendingPackageAction) return;
    installMutation.mutate({
      ...pendingPackageAction,
      confirmedActivePath: pendingPackageAction.tool.activeInstallationPath,
    });
    setPendingPackageAction(null);
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={<Button disabled={refreshState.disabled} variant="outline" onClick={() => refreshMutation.mutate(null)}>
          <RefreshCw className={refreshState.iconClassName} aria-hidden="true" />{t(refreshState.labelKey)}
        </Button>}
        description={t("cli.description")}
        icon={TerminalSquare}
        title={t("cli.title")}
      />
      <div className="grid gap-4 sm:grid-cols-2">
        <StatCard icon={CheckCircle2} label={t("cli.stats.installed")} value={`${installedCount} / ${tools.length}`} hint={t("cli.stats.installedHint")} />
        <StatCard icon={XCircle} label={t("cli.stats.missing")} value={`${missingCount} / ${tools.length}`} hint={t("cli.stats.missingHint")} />
      </div>
      {toolsQuery.error ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{String(toolsQuery.error)}</div> : null}
      <div className="grid gap-4 xl:grid-cols-2">
        {filteredTools.map((tool) => {
          const selectedVersion = selectedVersions[tool.agentId] ?? "";
          const operationId = activeOperationIds[tool.agentId] ?? tool.lastOperationId;
          const operation = operationId ? operationsById[operationId] : undefined;
          const refreshing = refreshMutation.isPending && refreshMutation.variables === tool.agentId || Boolean(operation && isOperationRunning(operation) && !packageOperationIds[tool.agentId]);
          return <CliEnvironmentCard
            key={tool.agentId}
            tool={tool}
            selectedVersion={selectedVersion}
            action={deriveCliVersionAction(tool, selectedVersion || null)}
            operation={operation}
            diagnosticsExpanded={Boolean(expandedDiagnostics[tool.agentId])}
            operationExpanded={Boolean(expandedLogs[tool.agentId])}
            refreshing={refreshing}
            packageBusy={packageBusy}
            onSelectedVersionChange={(version) => setSelectedVersions((current) => ({ ...current, [tool.agentId]: version }))}
            onRefresh={() => refreshMutation.mutate(tool.agentId)}
            onRunAction={() => requestPackageAction(tool, selectedVersion)}
            onCopyInstallCommand={() => void navigator.clipboard?.writeText(tool.installCommand)}
            onToggleDiagnostics={() => setExpandedDiagnostics((current) => ({ ...current, [tool.agentId]: !current[tool.agentId] }))}
            onToggleOperation={() => setExpandedLogs((current) => ({ ...current, [tool.agentId]: !current[tool.agentId] }))}
          />;
        })}
      </div>
      <CliConflictDialog tool={pendingPackageAction?.tool ?? null} onCancel={() => setPendingPackageAction(null)} onConfirm={confirmPackageAction} />
    </div>
  );
}
