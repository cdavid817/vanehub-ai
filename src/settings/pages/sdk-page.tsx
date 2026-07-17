import { AlertTriangle, CheckCircle2, Download, PackageCheck, RefreshCw, RotateCcw, Trash2, XCircle } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import { operationService } from "../../services/runtime-operation-client";
import { sdkService } from "../../services/runtime-sdk-client";
import { buildSdkVersionOptions, getSdkVersionAction, normalizeSdkVersion } from "../../services/sdk-versioning";
import type { OperationLogEntry, OperationTask } from "../../types/operation";
import type {
  SdkEnvironmentStatus,
  SdkDefinition,
  SdkId,
  SdkOperationResult,
  SdkStatus,
  SdkStatusMap,
  SdkVersionMap,
} from "../../types/sdk";
import { PageHeader, SectionPanel, StatCard, StatusPill, TagList } from "./page-parts";

type SelectedVersions = Partial<Record<SdkId, string>>;
type SdkOverview = {
  statuses: SdkStatusMap;
  versions: SdkVersionMap;
  environment: SdkEnvironmentStatus;
};

const sdkOverviewQueryKey = ["sdk", "overview"] as const;

export function SdkPage({ searchTerm }: { searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [selectedVersions, setSelectedVersions] = useState<SelectedVersions>({});
  const [logs, setLogs] = useState<OperationLogEntry[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [activeOperationId, setActiveOperationId] = useState<string | null>(null);
  const [handledOperationId, setHandledOperationId] = useState<string | null>(null);

  const sdkOverviewQuery = useQuery({
    queryKey: sdkOverviewQueryKey,
    queryFn: async (): Promise<SdkOverview> => {
      const [definitions, nextStatuses] = await Promise.all([
        sdkService.listDefinitions(),
        sdkService.listStatuses(),
      ]);
      const environment = await sdkService.checkEnvironment();

      return {
        statuses: nextStatuses,
        versions: fallbackVersionsFromDefinitions(definitions),
        environment,
      };
    },
  });

  const statuses = sdkOverviewQuery.data?.statuses ?? null;
  const versions = sdkOverviewQuery.data?.versions ?? ({} as SdkVersionMap);
  const environment = sdkOverviewQuery.data?.environment ?? null;

  useEffect(() => {
    if (!statuses) return;

    setSelectedVersions((current) => {
      const next = { ...current };
      for (const [id, status] of Object.entries(statuses) as [SdkId, SdkStatus][]) {
        const versionInfo = versions[id];
        const options = buildSdkVersionOptions({
          availableVersions: versionInfo?.versions,
          fallbackVersions: versionInfo?.fallbackVersions,
          installedVersion: status.installedVersion,
        });
        if (!next[id] || !options.includes(next[id] ?? "")) {
          next[id] = status.installedVersion ?? versionInfo?.latestVersion ?? options[0] ?? "";
        }
      }
      return next;
    });
  }, [statuses, versions]);

  const checkUpdatesMutation = useMutation({
    mutationFn: () => sdkService.checkUpdates(),
    onSuccess: (updates) => {
      queryClient.setQueryData<SdkOverview>(sdkOverviewQueryKey, (current) => {
        if (!current) return current;
        const nextStatuses = { ...current.statuses };
        for (const [id, update] of Object.entries(updates) as [SdkId, (typeof updates)[SdkId]][]) {
          nextStatuses[id] = {
            ...nextStatuses[id],
            latestVersion: update.latestVersion,
            hasUpdate: update.hasUpdate,
            errorMessage: update.errorMessage,
            lastChecked: new Date().toISOString(),
          };
        }
        return { ...current, statuses: nextStatuses };
      });
      setNotice(t("sdk.notice.updatesRefreshed"));
    },
  });

  const runOperationMutation = useMutation({
    mutationFn: async ({ sdk, requestedVersion }: { sdk: SdkStatus; requestedVersion?: string }) => {
      const installed = sdk.status === "installed";
      const action = getSdkVersionAction({
        installed,
        installedVersion: sdk.installedVersion,
        requestedVersion,
      });
      if (action === "install") return sdkService.install({ sdkId: sdk.id, version: requestedVersion });
      if (action === "update") return sdkService.update({ sdkId: sdk.id, version: requestedVersion });
      return sdkService.rollback({ sdkId: sdk.id, version: requestedVersion });
    },
    onSuccess: (operation) => handleOperationStarted(operation),
  });

  const uninstallMutation = useMutation({
    mutationFn: (sdk: SdkStatus) => sdkService.uninstall(sdk.id),
    onSuccess: (operation) => handleOperationStarted(operation),
  });

  const activeOperationQuery = useQuery({
    queryKey: ["operation", activeOperationId],
    queryFn: () => operationService.getOperationStatus(activeOperationId ?? ""),
    enabled: activeOperationId !== null,
    refetchInterval: (query) => (query.state.data?.status === "queued" || query.state.data?.status === "running" ? 600 : false),
  });

  const activeOperationStatus = activeOperationQuery.data?.status;
  const activeOperation = runOperationMutation.isPending
    ? runOperationMutation.variables?.sdk.id ?? null
    : uninstallMutation.isPending
      ? uninstallMutation.variables?.id ?? null
      : activeOperationStatus === "queued" || activeOperationStatus === "running"
        ? activeOperationQuery.data?.relatedEntityId ?? null
        : null;
  const refreshing = sdkOverviewQuery.isFetching;
  const checkingUpdates = checkUpdatesMutation.isPending;
  const queryError = sdkOverviewQuery.error instanceof Error ? sdkOverviewQuery.error.message : sdkOverviewQuery.error ? String(sdkOverviewQuery.error) : null;
  const visibleError = error ?? queryError;

  useEffect(() => {
    const operation = activeOperationQuery.data;
    if (!operation || operation.id === handledOperationId) return;
    setLogs(operation.logs);
    if (operation.status === "queued" || operation.status === "running") return;
    setHandledOperationId(operation.id);
    handleOperationFinished(operation);
  }, [activeOperationQuery.data, handledOperationId]);

  const sdkList = useMemo(() => {
    const query = searchTerm.trim().toLowerCase();
    const all = Object.values(statuses ?? {});
    if (!query) return all;
    return all.filter((sdk) =>
      [sdk.displayName, sdk.npmPackage, sdk.description, ...sdk.relatedProviders].some((value) =>
        value.toLowerCase().includes(query),
      ),
    );
  }, [searchTerm, statuses]);

  const installedCount = Object.values(statuses ?? {}).filter((sdk) => sdk.status === "installed").length;
  const updateCount = Object.values(statuses ?? {}).filter((sdk) => sdk.hasUpdate).length;
  const missingCount = Object.values(statuses ?? {}).filter((sdk) => sdk.status === "not-installed").length;
  const errorCount = Object.values(statuses ?? {}).filter((sdk) => sdk.status === "error").length;

  async function checkUpdates() {
    setError(null);
    await checkUpdatesMutation.mutateAsync().catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function runOperation(sdk: SdkStatus) {
    const requestedVersion = normalizeSdkVersion(selectedVersions[sdk.id]);
    const installed = sdk.status === "installed";
    const action = getSdkVersionAction({
      installed,
      installedVersion: sdk.installedVersion,
      requestedVersion,
    });
    if (action === "current") return;
    setError(null);
    setNotice(null);
    setLogs([]);
    await runOperationMutation.mutateAsync({ sdk, requestedVersion }).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  async function uninstall(sdk: SdkStatus) {
    if (!window.confirm(t("sdk.confirm.uninstall", { name: sdk.displayName }))) return;
    setError(null);
    setNotice(null);
    setLogs([]);
    await uninstallMutation.mutateAsync(sdk).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  function handleOperationStarted(operation: OperationTask) {
    setActiveOperationId(operation.id);
    setHandledOperationId(null);
    setLogs(operation.logs);
  }

  function handleOperationFinished(operation: OperationTask) {
    if (operation.status === "failed") {
      setError(operation.error ?? t("sdk.error.operationFailed"));
      return;
    }
    const result = sdkOperationResult(operation.result);
    if (!result) {
      setError(t("sdk.error.operationFailed"));
      return;
    }
    if (!result.success) {
      setError(result.error ?? t("sdk.error.operationFailed"));
      return;
    }
    setNotice(t("sdk.notice.operationCompleted"));
    void queryClient.invalidateQueries({ queryKey: sdkOverviewQueryKey });
  }

  function actionLabel(sdk: SdkStatus) {
    const requestedVersion = normalizeSdkVersion(selectedVersions[sdk.id]);
    const action = getSdkVersionAction({
      installed: sdk.status === "installed",
      installedVersion: sdk.installedVersion,
      requestedVersion,
    });
    const versionLabel = requestedVersion ? ` v${requestedVersion}` : "";
    if (action === "install") return t("sdk.action.install", { version: versionLabel });
    if (action === "update") return t("sdk.action.update", { version: versionLabel });
    if (action === "rollback") return t("sdk.action.rollback", { version: versionLabel });
    return t("sdk.action.current");
  }

  function renderSdkCard(sdk: SdkStatus) {
    const versionInfo = versions[sdk.id];
    const options = buildSdkVersionOptions({
      availableVersions: versionInfo?.versions,
      fallbackVersions: versionInfo?.fallbackVersions,
      installedVersion: sdk.installedVersion,
    });
    const selectedVersion = selectedVersions[sdk.id] ?? "";
    const action = getSdkVersionAction({
      installed: sdk.status === "installed",
      installedVersion: sdk.installedVersion,
      requestedVersion: selectedVersion,
    });
    const busy = activeOperation !== null;
    const operationBusy = activeOperation === sdk.id;
    const environmentUnavailable = environment?.available === false;

    return (
      <div className="ucd-panel rounded-lg p-4" key={sdk.id}>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0 space-y-2">
            <div className="flex flex-wrap items-center gap-2">
              <h3 className="text-sm font-semibold">{sdk.displayName}</h3>
              <StatusPill status={t(`sdk.status.${sdk.status === "not-installed" ? "notInstalled" : sdk.status}`)} />
              {sdk.hasUpdate ? <StatusPill status={t("sdk.status.updateAvailable")} /> : null}
            </div>
            <p className="text-sm text-muted-foreground">{sdk.description}</p>
            <TagList tags={sdk.relatedProviders} />
          </div>
          <div className="text-right text-xs text-muted-foreground">
            <div>{sdk.npmPackage}</div>
            <div>{sdk.installPath ?? "~/.vanehub/dependencies"}</div>
          </div>
        </div>

        <div className="mt-4 grid gap-3 text-sm md:grid-cols-3">
          <div className="rounded border border-border p-3">
            {t("sdk.currentVersion")}
            <strong className="block">{sdk.installedVersion ? `v${sdk.installedVersion}` : t("sdk.status.notInstalled")}</strong>
          </div>
          <div className="rounded border border-border p-3">
            {t("sdk.latestVersion")}
            <strong className="block">{versionInfo?.latestVersion ? `v${versionInfo.latestVersion}` : t("sdk.source.unknown")}</strong>
          </div>
          <div className="rounded border border-border p-3">
            {t("sdk.versionSource")}
            <strong className="block">{versionInfo?.source === "fallback" ? t("sdk.source.fallback") : t("sdk.source.remote")}</strong>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <label className="text-sm text-muted-foreground" htmlFor={`sdk-version-${sdk.id}`}>
            {t("sdk.targetVersion")}
          </label>
          <select
            className="h-9 rounded-md border border-input bg-background px-3 text-sm"
            disabled={busy || !options.length}
            id={`sdk-version-${sdk.id}`}
            onChange={(event) => setSelectedVersions((current) => ({ ...current, [sdk.id]: event.target.value }))}
            value={selectedVersion}
          >
            {options.map((version) => (
              <option key={version} value={version}>
                v{version}
              </option>
            ))}
          </select>
          <Button disabled={busy || action === "current" || environmentUnavailable} onClick={() => void runOperation(sdk)}>
            {operationBusy ? (
              <RefreshCw className="h-4 w-4 animate-spin" aria-hidden="true" />
            ) : action === "rollback" ? (
              <RotateCcw className="h-4 w-4" aria-hidden="true" />
            ) : (
              <Download className="h-4 w-4" aria-hidden="true" />
            )}
            {operationBusy ? t("sdk.action.running") : actionLabel(sdk)}
          </Button>
          {sdk.status === "installed" ? (
            <Button disabled={busy} onClick={() => void uninstall(sdk)} variant="outline">
              <Trash2 className="h-4 w-4" aria-hidden="true" />
              {t("sdk.action.uninstall")}
            </Button>
          ) : null}
        </div>
        {sdk.errorMessage ? <div className="mt-3 rounded-md border p-3 text-sm ucd-status-danger">{sdk.errorMessage}</div> : null}
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <PageHeader
        actions={
          <>
            <Button disabled={refreshing} variant="outline" onClick={() => void sdkOverviewQuery.refetch()}>
              <RefreshCw className={refreshing ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
              {refreshing ? t("sdk.refreshing") : t("sdk.refresh")}
            </Button>
            <Button disabled={checkingUpdates} onClick={() => void checkUpdates()}>
              <RefreshCw className={checkingUpdates ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
              {checkingUpdates ? t("sdk.checking") : t("sdk.checkUpdates")}
            </Button>
          </>
        }
        description={t("sdk.description")}
        icon={PackageCheck}
        title={t("sdk.title")}
      />

      <div className="grid gap-4 md:grid-cols-4">
        <StatCard icon={CheckCircle2} label={t("sdk.stats.installed")} value={String(installedCount)} hint={t("sdk.stats.installedHint")} />
        <StatCard icon={RefreshCw} label={t("sdk.stats.updates")} value={String(updateCount)} hint={t("sdk.stats.updatesHint")} />
        <StatCard icon={XCircle} label={t("sdk.stats.missing")} value={String(missingCount)} hint={t("sdk.stats.missingHint")} />
        <StatCard icon={AlertTriangle} label={t("sdk.stats.errors")} value={String(errorCount)} hint={t("sdk.stats.errorsHint")} />
      </div>

      {environment?.available === false ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning">{environment.error ?? t("sdk.error.environmentUnavailable")}</div>
      ) : null}
      {visibleError ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{visibleError}</div> : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      <SectionPanel title={t("sdk.list.title")} description={t("sdk.list.description")}>
        <div className="grid gap-4 xl:grid-cols-2">{sdkList.map(renderSdkCard)}</div>
        {!statuses ? <div className="py-8 text-center text-sm text-muted-foreground">{t("sdk.list.loading")}</div> : null}
        {statuses && !sdkList.length ? <div className="py-8 text-center text-sm text-muted-foreground">{t("sdk.list.empty")}</div> : null}
      </SectionPanel>

      {logs.length ? (
        <SectionPanel title={t("sdk.logs.title")} description={t("sdk.logs.description")}>
          <pre className="max-h-72 overflow-auto rounded-md border border-border bg-muted/30 p-3 text-xs leading-5">
            {logs.map((entry) => `[${entry.operationId}] ${entry.line}`).join("\n")}
          </pre>
        </SectionPanel>
      ) : null}
    </div>
  );
}

function sdkOperationResult(result: OperationTask["result"]): SdkOperationResult | null {
  if (!result || typeof result !== "object") return null;
  if (typeof result.success !== "boolean") return null;
  if (typeof result.sdkId !== "string") return null;
  if (typeof result.operation !== "string") return null;
  return result as unknown as SdkOperationResult;
}

function fallbackVersionsFromDefinitions(definitions: SdkDefinition[]): SdkVersionMap {
  return definitions.reduce<SdkVersionMap>((versions, definition) => {
    versions[definition.id] = {
      sdkId: definition.id,
      versions: definition.fallbackVersions,
      fallbackVersions: definition.fallbackVersions,
      source: "fallback",
      latestVersion: definition.fallbackVersions[0] ?? null,
    };
    return versions;
  }, {} as SdkVersionMap);
}
