import { Activity, Box, CheckCircle2, Cpu, Download, Play, RefreshCw, Square, Trash2 } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { ExtensionService } from "../../services/extension-service";
import { operationService } from "../../services/runtime-operation-client";
import { extensionService } from "../../services/runtime-extension-client";
import type {
  ExtensionFrameworkDefinition,
  ExtensionFrameworkId,
  ExtensionFrameworkStatus,
  ExtensionInstallPreview,
} from "../../types/extension";
import type { OperationTask } from "../../types/operation";
import { PageHeader, SectionPanel, StatCard, StatusPill, TagList } from "./page-parts";

const overviewKey = ["extensions", "overview"] as const;

function statusKey(status: ExtensionFrameworkStatus["status"]) {
  return `extensions.status.${status}`;
}

export function filterExtensionDefinitions(
  definitions: ExtensionFrameworkDefinition[],
  statuses: ExtensionFrameworkStatus[],
  searchTerm: string,
  translate: (key: string) => string,
) {
  const query = searchTerm.trim().toLowerCase();
  if (!query) return definitions;
  return definitions.filter((definition) => {
    const status = statuses.find((item) => item.frameworkId === definition.id);
    const values = [
      definition.id,
      definition.capabilityId,
      translate(`extensions.capability.${definition.capabilityId}`),
      translate(definition.nameKey),
      translate(definition.descriptionKey),
      definition.requirement.runtime,
      ...definition.requirement.packages,
      status ? translate(statusKey(status.status)) : "",
    ];
    return values.some((value) => value.toLowerCase().includes(query));
  });
}

export function ExtensionsPage({
  searchTerm,
  service = extensionService,
}: {
  searchTerm: string;
  service?: ExtensionService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [preview, setPreview] = useState<ExtensionInstallPreview | null>(null);
  const [activeOperation, setActiveOperation] = useState<OperationTask | null>(null);
  const [error, setError] = useState<string | null>(null);
  const overviewQuery = useQuery({ queryKey: overviewKey, queryFn: () => service.getOverview() });
  const operationQuery = useQuery({
    queryKey: ["operation", activeOperation?.id],
    queryFn: () => operationService.getOperationStatus(activeOperation?.id ?? ""),
    enabled: activeOperation !== null,
    refetchInterval: (query) =>
      query.state.data?.status === "queued" || query.state.data?.status === "running" ? 600 : false,
  });

  useEffect(() => {
    const operation = operationQuery.data;
    if (!operation) return;
    setActiveOperation(operation);
    if (operation.status === "failed") setError(operation.error ?? t("extensions.error.operationFailed"));
    if (operation.status === "succeeded" || operation.status === "failed") {
      void queryClient.invalidateQueries({ queryKey: overviewKey });
    }
  }, [operationQuery.data, queryClient, t]);

  const operationMutation = useMutation({
    mutationFn: async ({ action, frameworkId }: { action: string; frameworkId: ExtensionFrameworkId }) => {
      if (action === "install") return service.install({ frameworkId });
      if (action === "uninstall") return service.uninstall({ frameworkId });
      if (action === "start") return service.start({ frameworkId });
      if (action === "stop") return service.stop({ frameworkId });
      if (action === "self-test") return service.selfTest({ frameworkId });
      return service.setEnabled({ frameworkId, enabled: action === "enable" });
    },
    onSuccess: (operation) => setActiveOperation(operation),
    onError: (reason) => setError(reason instanceof Error ? reason.message : String(reason)),
  });

  const overview = overviewQuery.data;
  const visibleDefinitions = useMemo(
    () => filterExtensionDefinitions(overview?.definitions ?? [], overview?.statuses ?? [], searchTerm, t),
    [overview, searchTerm, t],
  );
  const installed = overview?.statuses.filter((status) => status.installed).length ?? 0;
  const running = overview?.statuses.filter((status) => status.running).length ?? 0;
  const errors = overview?.statuses.filter((status) => status.status === "error").length ?? 0;

  async function openPreview(frameworkId: ExtensionFrameworkId) {
    setError(null);
    try {
      setPreview(await service.getInstallPreview({ frameworkId }));
    } catch (reason) {
      setError(reason instanceof Error ? reason.message : String(reason));
    }
  }

  async function runAction(action: string, frameworkId: ExtensionFrameworkId) {
    if (action === "uninstall" && !window.confirm(t("extensions.confirm.uninstall"))) return;
    setError(null);
    await operationMutation.mutateAsync({ action, frameworkId }).catch(() => undefined);
  }

  function renderCard(definition: ExtensionFrameworkDefinition) {
    const status = overview?.statuses.find((item) => item.frameworkId === definition.id);
    if (!status) return null;
    const nativeAvailable = overview?.environment.nativeOperationsAvailable === true;
    const busy = activeOperation?.relatedEntityId === definition.id &&
      (activeOperation.status === "queued" || activeOperation.status === "running");
    return (
      <article className="ucd-panel ucd-interactive rounded-lg p-4" data-testid={`extension-card-${definition.id}`} key={definition.id}>
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0">
            <div className="text-xs font-semibold uppercase tracking-[0.14em] text-primary">
              {t(`extensions.capability.${definition.capabilityId}`)}
            </div>
            <h3 className="mt-1 text-base font-semibold">{t(definition.nameKey)}</h3>
            <p className="mt-1 text-sm leading-6 text-muted-foreground">{t(definition.descriptionKey)}</p>
          </div>
          <StatusPill status={t(statusKey(status.status))} />
        </div>
        <div className="mt-3 grid gap-3 text-xs text-muted-foreground md:grid-cols-3">
          <div><span className="block">{t("extensions.runtime")}</span><strong className="text-foreground">{definition.requirement.runtime}</strong></div>
          <div><span className="block">{t("extensions.port")}</span><strong className="text-foreground">{status.port}</strong></div>
          <div><span className="block">{t("extensions.disk")}</span><strong className="text-foreground">~{definition.requirement.estimatedDiskMb} MB</strong></div>
        </div>
        <div className="mt-3"><TagList tags={definition.requirement.packages} /></div>
        {status.lastError ? <div className="mt-3 rounded border p-2 text-xs ucd-status-warning">{t(status.lastError)}</div> : null}
        <div className="mt-4 flex flex-wrap gap-2">
          <Button onClick={() => void openPreview(definition.id)} size="sm" variant="outline"><Box />{t("extensions.action.requirements")}</Button>
          {!status.installed ? <Button disabled={!nativeAvailable || busy} onClick={() => void openPreview(definition.id)} size="sm"><Download />{t("extensions.action.install")}</Button> : null}
          {status.installed && !status.running ? <Button disabled={!nativeAvailable || busy} onClick={() => void runAction("start", definition.id)} size="sm"><Play />{t("extensions.action.start")}</Button> : null}
          {status.running ? <Button disabled={!nativeAvailable || busy} onClick={() => void runAction("stop", definition.id)} size="sm" variant="outline"><Square />{t("extensions.action.stop")}</Button> : null}
          {status.installed ? <Button disabled={!nativeAvailable || busy} onClick={() => void runAction("self-test", definition.id)} size="sm" variant="outline"><CheckCircle2 />{t("extensions.action.selfTest")}</Button> : null}
          {status.installed ? <Button disabled={!nativeAvailable || busy} onClick={() => void runAction(status.enabled ? "disable" : "enable", definition.id)} size="sm" variant="outline">{status.enabled ? t("extensions.action.disable") : t("extensions.action.enable")}</Button> : null}
          {status.installed ? <Button disabled={!nativeAvailable || busy || status.running} onClick={() => void runAction("uninstall", definition.id)} size="sm" variant="ghost"><Trash2 />{t("extensions.action.uninstall")}</Button> : null}
        </div>
        {busy || activeOperation?.relatedEntityId === definition.id ? (
          <div className="mt-3 rounded-md border border-border bg-[hsl(var(--panel-muted))] p-3 text-xs">
            <div className="font-medium">{t("extensions.logs.title")}</div>
            <div className="mt-2 grid gap-1 font-mono text-muted-foreground">
              {(activeOperation?.logs ?? []).map((log) => <div key={`${log.timestamp}-${log.line}`}>{log.line}</div>)}
            </div>
          </div>
        ) : null}
      </article>
    );
  }

  return (
    <div className="space-y-4">
      <PageHeader actions={<Button disabled={overviewQuery.isFetching} onClick={() => void overviewQuery.refetch()} variant="outline"><RefreshCw className={overviewQuery.isFetching ? "animate-spin" : ""} />{t("extensions.refresh")}</Button>} description={t("extensions.description")} icon={Cpu} title={t("extensions.title")} />
      {overview && !overview.environment.nativeOperationsAvailable ? <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("extensions.environment.desktopOnly")}</div> : null}
      {error ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{error}</div> : null}
      <div className="grid gap-3 md:grid-cols-3">
        <StatCard hint={t("extensions.stats.installedHint")} icon={Box} label={t("extensions.stats.installed")} value={String(installed)} />
        <StatCard hint={t("extensions.stats.runningHint")} icon={Activity} label={t("extensions.stats.running")} value={String(running)} />
        <StatCard hint={t("extensions.stats.errorsHint")} icon={Cpu} label={t("extensions.stats.errors")} value={String(errors)} />
      </div>
      <SectionPanel description={t("extensions.list.description")} title={t("extensions.list.title")}>
        {overviewQuery.isLoading ? <div className="text-sm text-muted-foreground">{t("extensions.loading")}</div> : null}
        <div className="grid gap-3">{visibleDefinitions.map(renderCard)}</div>
        {!overviewQuery.isLoading && visibleDefinitions.length === 0 ? <div className="text-sm text-muted-foreground">{t("extensions.empty")}</div> : null}
      </SectionPanel>
      {preview ? <InstallPreview preview={preview} onClose={() => setPreview(null)} onInstall={() => { setPreview(null); void runAction("install", preview.frameworkId); }} nativeAvailable={overview?.environment.nativeOperationsAvailable === true} /> : null}
    </div>
  );
}

function InstallPreview({ preview, nativeAvailable, onClose, onInstall }: { preview: ExtensionInstallPreview; nativeAvailable: boolean; onClose: () => void; onInstall: () => void }) {
  const { t } = useTranslation();
  return <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50 p-4"><div className="ucd-panel max-h-[85vh] w-full max-w-xl overflow-y-auto rounded-lg p-5" role="dialog" aria-modal="true"><h3 className="text-lg font-semibold">{t("extensions.preview.title")}</h3><p className="mt-1 text-sm text-muted-foreground">{t("extensions.preview.description")}</p><dl className="mt-4 grid gap-3 text-sm md:grid-cols-2"><div><dt className="text-muted-foreground">{t("extensions.preview.path")}</dt><dd className="break-all font-medium">{preview.installPath}</dd></div><div><dt className="text-muted-foreground">{t("extensions.preview.download")}</dt><dd className="font-medium">~{preview.estimatedDownloadMb} MB</dd></div><div><dt className="text-muted-foreground">{t("extensions.preview.disk")}</dt><dd className="font-medium">~{preview.estimatedDiskMb} MB</dd></div><div><dt className="text-muted-foreground">{t("extensions.preview.network")}</dt><dd className="font-medium">{t("extensions.preview.installOnly")}</dd></div></dl><div className="mt-4"><TagList tags={preview.packages} /></div>{preview.reason ? <div className="mt-4 rounded border p-3 text-sm ucd-status-warning">{t(preview.reason)}</div> : null}<div className="mt-5 flex justify-end gap-2"><Button onClick={onClose} variant="outline">{t("extensions.action.cancel")}</Button><Button disabled={!nativeAvailable || !preview.supported} onClick={onInstall}>{t("extensions.action.confirmInstall")}</Button></div></div></div>;
}
