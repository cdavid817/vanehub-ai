import { useEffect, useMemo, useState } from "react";
import { useMutation, useQueries, useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, ChevronDown, ChevronRight, Clipboard, Download, RefreshCw, Terminal } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../components/ui/badge";
import { Button } from "../../components/ui/button";
import { agentService } from "../../services/runtime-agent-client";
import { operationService } from "../../services/runtime-operation-client";
import type { CliToolStatus } from "../../types/agent";
import type { OperationTask } from "../../types/operation";
import { deriveCliVersionAction, type CliVersionAction } from "./cli-management-utils";
import { PageHeader, StatCard } from "./page-parts";

const cliToolsQueryKey = ["cli-tools"] as const;

function actionLabelKey(action: CliVersionAction) {
  return `cli.action.${action}`;
}

function statusTone(tool: CliToolStatus): "success" | "warning" | "muted" {
  if (tool.installed === true) return "success";
  if (tool.installed === false || tool.versionCheckStatus === "failed") return "warning";
  return "muted";
}

function statusLabelKey(tool: CliToolStatus) {
  if (tool.installed === true) return "cli.status.installed";
  if (tool.installed === false) return "cli.status.missing";
  if (tool.versionCheckStatus === "unsupported") return "cli.status.unsupported";
  return "cli.status.undetected";
}

function versionOptions(tool: CliToolStatus) {
  const options = [...tool.availableVersions];
  if (tool.latestVersion && !options.includes(tool.latestVersion)) options.unshift(tool.latestVersion);
  if (tool.currentVersion && !options.includes(tool.currentVersion)) options.push(tool.currentVersion);
  return [...new Set(options)];
}

export function ProvidersPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({});
  const [expandedLogs, setExpandedLogs] = useState<Record<string, boolean>>({});
  const [activeOperationIds, setActiveOperationIds] = useState<Record<string, string>>({});

  const toolsQuery = useQuery({
    queryKey: cliToolsQueryKey,
    queryFn: () => agentService.listCliTools(),
  });
  const tools = toolsQuery.data ?? [];

  useEffect(() => {
    setSelectedVersions((current) => {
      const next = { ...current };
      for (const tool of tools) {
        if (!next[tool.agentId]) {
          next[tool.agentId] = tool.latestVersion ?? tool.availableVersions[0] ?? tool.currentVersion ?? "";
        }
      }
      return next;
    });
  }, [tools]);

  const operationIds = useMemo(
    () => [...new Set([...Object.values(activeOperationIds), ...tools.map((tool) => tool.lastOperationId).filter(Boolean) as string[]])],
    [activeOperationIds, tools],
  );

  const operationQueries = useQueries({
    queries: operationIds.map((operationId) => ({
      queryKey: ["operation", operationId],
      queryFn: () => operationService.getOperationStatus(operationId),
      refetchInterval: (query: { state: { data?: OperationTask } }) =>
        query.state.data?.status === "running" || query.state.data?.status === "queued" ? 1500 : false,
      enabled: Boolean(operationId),
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
    const running = Object.values(operationsById).some((operation) => operation.status === "running" || operation.status === "queued");
    if (!running && Object.keys(activeOperationIds).length > 0) {
      setActiveOperationIds({});
      void queryClient.invalidateQueries({ queryKey: cliToolsQueryKey });
    }
  }, [activeOperationIds, operationsById, queryClient]);

  const refreshMutation = useMutation({
    mutationFn: () => agentService.refreshCliDetections(),
    onSuccess: (operation) => {
      const next = Object.fromEntries(tools.map((tool) => [tool.agentId, operation.id]));
      setActiveOperationIds(next);
    },
  });

  const installMutation = useMutation({
    mutationFn: ({ agentId, targetVersion }: { agentId: string; targetVersion: string }) =>
      agentService.installCliVersion({ agentId, targetVersion }),
    onSuccess: (operation, variables) => {
      setActiveOperationIds((current) => ({ ...current, [variables.agentId]: operation.id }));
    },
  });

  const filteredTools = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    if (!query) return tools;
    return tools.filter((tool) =>
      [tool.displayName, tool.provider, tool.executableName, tool.packageName].some((value) => value.toLowerCase().includes(query)),
    );
  }, [searchTerm, tools]);

  const installedCount = tools.filter((tool) => tool.installed === true).length;
  const missingCount = tools.filter((tool) => tool.installed === false).length;

  async function copyInstallCommand(command: string) {
    await navigator.clipboard?.writeText(command);
  }

  function operationFor(tool: CliToolStatus) {
    const operationId = activeOperationIds[tool.agentId] ?? tool.lastOperationId;
    return operationId ? operationsById[operationId] : undefined;
  }

  function isOperationRunning(operation?: OperationTask) {
    return operation?.status === "running" || operation?.status === "queued";
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <Button disabled={refreshMutation.isPending} variant="outline" onClick={() => refreshMutation.mutate()}>
            <RefreshCw className="h-4 w-4" aria-hidden="true" />
            {t(refreshMutation.isPending ? "cli.refreshing" : "cli.refresh")}
          </Button>
        }
        description={t("cli.description")}
        title={t("cli.title")}
      />

      <div className="grid gap-4 sm:grid-cols-2">
        <StatCard label={t("cli.stats.installed")} value={`${installedCount} / ${tools.length}`} hint={t("cli.stats.installedHint")} />
        <StatCard label={t("cli.stats.missing")} value={`${missingCount} / ${tools.length}`} hint={t("cli.stats.missingHint")} />
      </div>

      {toolsQuery.error ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{String(toolsQuery.error)}</div> : null}

      <div className="grid gap-4">
        {filteredTools.map((tool) => {
          const options = versionOptions(tool);
          const selectedVersion = selectedVersions[tool.agentId] || "";
          const action = deriveCliVersionAction(tool, selectedVersion || null);
          const operation = operationFor(tool);
          const running = isOperationRunning(operation);
          const disabled = running || action === "current" || action === "unavailable" || !selectedVersion;
          return (
            <section className="ucd-panel rounded-lg p-4" key={tool.agentId}>
              <div className="flex flex-wrap items-start justify-between gap-3">
                <div className="min-w-0">
                  <div className="flex items-center gap-2">
                    <Terminal className="h-4 w-4 text-primary" aria-hidden="true" />
                    <h3 className="truncate font-semibold">{tool.displayName}</h3>
                  </div>
                  <p className="mt-1 text-sm text-muted-foreground">{tool.packageName}</p>
                </div>
                <div className="flex flex-wrap items-center gap-2">
                  <Badge tone={statusTone(tool)}>{t(statusLabelKey(tool))}</Badge>
                  <Badge tone="muted">{tool.currentVersion ?? t("cli.versionUnknown")}</Badge>
                </div>
              </div>

              <dl className="mt-4 grid gap-3 text-sm md:grid-cols-2">
                <div>
                  <dt className="text-muted-foreground">{t("cli.currentVersion")}</dt>
                  <dd className="font-medium">{tool.currentVersion ?? t("cli.notAvailable")}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t("cli.latestVersion")}</dt>
                  <dd className="font-medium">{tool.latestVersion ?? t("cli.notAvailable")}</dd>
                </div>
                <div className="md:col-span-2">
                  <dt className="text-muted-foreground">{t("cli.installPath")}</dt>
                  <dd className="break-all font-medium">{tool.detectedPath ?? t("cli.notAvailable")}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t("cli.lastChecked")}</dt>
                  <dd className="font-medium">{tool.lastCheckedAt ?? t("cli.neverChecked")}</dd>
                </div>
                <div>
                  <dt className="text-muted-foreground">{t("cli.executable")}</dt>
                  <dd className="font-medium">{tool.executableName}</dd>
                </div>
              </dl>

              {tool.lastError ? <div className="mt-4 rounded-md border p-3 text-sm ucd-status-warning">{tool.lastError}</div> : null}

              <div className="mt-4 flex flex-wrap items-center gap-2">
                <select
                  className="ucd-input h-9 min-w-44 rounded px-3 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
                  disabled={running || options.length === 0}
                  value={selectedVersion}
                  onChange={(event) => setSelectedVersions((current) => ({ ...current, [tool.agentId]: event.target.value }))}
                >
                  {options.length === 0 ? <option value="">{t("cli.noVersions")}</option> : null}
                  {options.map((version) => (
                    <option key={version} value={version}>
                      {version}
                    </option>
                  ))}
                </select>
                <Button
                  disabled={disabled}
                  onClick={() => installMutation.mutate({ agentId: tool.agentId, targetVersion: selectedVersion })}
                >
                  {action === "current" ? <CheckCircle2 className="h-4 w-4" aria-hidden="true" /> : <Download className="h-4 w-4" aria-hidden="true" />}
                  {t(actionLabelKey(action))}
                </Button>
                <Button variant="outline" onClick={() => void copyInstallCommand(tool.installCommand)}>
                  <Clipboard className="h-4 w-4" aria-hidden="true" />
                  {t("cli.copyInstall")}
                </Button>
              </div>

              {operation ? (
                <div className="mt-4 rounded-md border border-border p-3 text-sm">
                  <button
                    className="flex w-full items-center justify-between gap-3 text-left"
                    type="button"
                    onClick={() => setExpandedLogs((current) => ({ ...current, [tool.agentId]: !current[tool.agentId] }))}
                  >
                    <span className="font-medium">
                      {t("cli.operation")}: {t(`cli.operationStatus.${operation.status}`)}
                    </span>
                    {expandedLogs[tool.agentId] ? <ChevronDown className="h-4 w-4" /> : <ChevronRight className="h-4 w-4" />}
                  </button>
                  {expandedLogs[tool.agentId] ? (
                    <div className="mt-3 max-h-44 overflow-auto rounded border border-border bg-muted p-2 font-mono text-xs">
                      {operation.logs.length === 0 ? <div>{t("cli.noLogs")}</div> : null}
                      {operation.logs.map((log, index) => (
                        <div className="whitespace-pre-wrap" key={`${log.timestamp}-${index}`}>
                          {log.line}
                        </div>
                      ))}
                      {operation.error ? <div className="mt-2 text-red-600">{operation.error}</div> : null}
                    </div>
                  ) : null}
                </div>
              ) : null}
            </section>
          );
        })}
      </div>
    </div>
  );
}
