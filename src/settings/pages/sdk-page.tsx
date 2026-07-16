import { Download, RefreshCw, RotateCcw, Trash2 } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../../components/ui/button";
import { sdkService } from "../../services/runtime-sdk-client";
import { buildSdkVersionOptions, getSdkVersionAction, normalizeSdkVersion } from "../../services/sdk-versioning";
import type {
  SdkEnvironmentStatus,
  SdkDefinition,
  SdkId,
  SdkOperationLog,
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

const statusText: Record<SdkStatus["status"], string> = {
  installed: "Installed",
  "not-installed": "Not installed",
  installing: "Installing",
  uninstalling: "Uninstalling",
  error: "Error",
};

export function SdkPage({ searchTerm }: { searchTerm: string }) {
  const queryClient = useQueryClient();
  const [selectedVersions, setSelectedVersions] = useState<SelectedVersions>({});
  const [logs, setLogs] = useState<SdkOperationLog[]>([]);
  const [notice, setNotice] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

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
      setNotice("SDK update status refreshed");
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
    onSuccess: (result) => handleOperationResult(result),
  });

  const uninstallMutation = useMutation({
    mutationFn: (sdk: SdkStatus) => sdkService.uninstall(sdk.id),
    onSuccess: (result) => handleOperationResult(result),
  });

  const activeOperation = runOperationMutation.isPending
    ? runOperationMutation.variables?.sdk.id ?? null
    : uninstallMutation.isPending
      ? uninstallMutation.variables?.id ?? null
      : null;
  const refreshing = sdkOverviewQuery.isFetching;
  const checkingUpdates = checkUpdatesMutation.isPending;
  const queryError = sdkOverviewQuery.error instanceof Error ? sdkOverviewQuery.error.message : sdkOverviewQuery.error ? String(sdkOverviewQuery.error) : null;
  const visibleError = error ?? queryError;

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
    if (!window.confirm(`Uninstall ${sdk.displayName}?`)) return;
    setError(null);
    setNotice(null);
    setLogs([]);
    await uninstallMutation.mutateAsync(sdk).catch((err) => setError(err instanceof Error ? err.message : String(err)));
  }

  function handleOperationResult(result: SdkOperationResult) {
    setLogs(result.logs);
    if (!result.success) {
      setError(result.error ?? "SDK operation failed");
      return;
    }
    setNotice("SDK operation completed");
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
    if (action === "install") return `Install${versionLabel}`;
    if (action === "update") return `Update to${versionLabel}`;
    if (action === "rollback") return `Rollback to${versionLabel}`;
    return "Current version";
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
              <StatusPill status={statusText[sdk.status]} />
              {sdk.hasUpdate ? <StatusPill status="Update available" /> : null}
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
            Current version
            <strong className="block">{sdk.installedVersion ? `v${sdk.installedVersion}` : "Not installed"}</strong>
          </div>
          <div className="rounded border border-border p-3">
            Latest version
            <strong className="block">{versionInfo?.latestVersion ? `v${versionInfo.latestVersion}` : "Unknown"}</strong>
          </div>
          <div className="rounded border border-border p-3">
            Version source
            <strong className="block">{versionInfo?.source === "fallback" ? "Fallback" : "Remote"}</strong>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <label className="text-sm text-muted-foreground" htmlFor={`sdk-version-${sdk.id}`}>
            Target version
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
            {operationBusy ? "Running" : actionLabel(sdk)}
          </Button>
          {sdk.status === "installed" ? (
            <Button disabled={busy} onClick={() => void uninstall(sdk)} variant="outline">
              <Trash2 className="h-4 w-4" aria-hidden="true" />
              Uninstall
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
              {refreshing ? "Refreshing" : "Refresh"}
            </Button>
            <Button disabled={checkingUpdates} onClick={() => void checkUpdates()}>
              <RefreshCw className={checkingUpdates ? "h-4 w-4 animate-spin" : "h-4 w-4"} aria-hidden="true" />
              {checkingUpdates ? "Checking" : "Check Updates"}
            </Button>
          </>
        }
        description="Manage local AI SDK installation, versions, and update status"
        title="SDK Dependencies"
      />

      <div className="grid gap-4 md:grid-cols-4">
        <StatCard label="SDK Installed" value={String(installedCount)} hint="VaneHub local dependency directory" />
        <StatCard label="SDK Updates" value={String(updateCount)} hint="From version checks" />
        <StatCard label="SDK Missing" value={String(missingCount)} hint="Can be installed by selected version" />
        <StatCard label="SDK Errors" value={String(errorCount)} hint="Review logs" />
      </div>

      {environment?.available === false ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning">{environment.error ?? "Node.js or npm is unavailable"}</div>
      ) : null}
      {visibleError ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{visibleError}</div> : null}
      {notice ? <div className="rounded-md border p-3 text-sm ucd-status-success">{notice}</div> : null}

      <SectionPanel title="SDK List" description="Install directory is fixed to ~/.vanehub/dependencies/">
        <div className="grid gap-4 xl:grid-cols-2">{sdkList.map(renderSdkCard)}</div>
        {!statuses ? <div className="py-8 text-center text-sm text-muted-foreground">Loading SDK status</div> : null}
        {statuses && !sdkList.length ? <div className="py-8 text-center text-sm text-muted-foreground">No matching SDKs</div> : null}
      </SectionPanel>

      {logs.length ? (
        <SectionPanel title="Operation Logs" description="Output from the most recent SDK operation">
          <pre className="max-h-72 overflow-auto rounded-md border border-border bg-muted/30 p-3 text-xs leading-5">
            {logs.map((entry) => `[${entry.sdkId}] ${entry.line}`).join("\n")}
          </pre>
        </SectionPanel>
      ) : null}
    </div>
  );
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
