import { Activity, Box, Cpu, RefreshCw } from "lucide-react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../../components/ui/button";
import type { ExtensionService } from "../../services/extension-service";
import { operationService } from "../../services/runtime-operation-client";
import { extensionService } from "../../services/runtime-extension-client";
import { AsyncBoundary } from "../../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../../ui/async/async-view-state";
import type { MutationState } from "../../ui/async/mutation-state";
import { PageHeader } from "../../ui/page-header/PageHeader";
import type {
  ExtensionFrameworkDefinition,
  ExtensionFrameworkId,
  ExtensionFrameworkStatus,
  ExtensionInstallPreview,
  ExtensionOverview,
} from "../../types/extension";
import type { OperationTask } from "../../types/operation";
import { pickPageStatus } from "../settings-page-status";
import type { SettingsPageStatus } from "../settings-page-types";
import { ExtensionFrameworkCard } from "./extensions/extension-framework-card";
import { ExtensionInstallPreviewDialog } from "./extensions/extension-install-preview";
import { statusKey } from "./extensions/extension-status";
import { SectionPanel, StatCard } from "./page-parts";

const overviewKey = ["extensions", "overview"] as const;

function errorMessage(reason: unknown) {
  return reason instanceof Error ? reason.message : String(reason);
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
  onStatusChange,
  searchTerm,
  service = extensionService,
}: {
  onStatusChange?: (status: SettingsPageStatus | null) => void;
  searchTerm: string;
  service?: ExtensionService;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [preview, setPreview] = useState<ExtensionInstallPreview | null>(null);
  const [activeOperation, setActiveOperation] = useState<OperationTask | null>(null);
  const [previewError, setPreviewError] = useState<string | null>(null);
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
    if (operation.status === "succeeded" || operation.status === "failed") {
      void queryClient.invalidateQueries({ queryKey: overviewKey });
    }
  }, [operationQuery.data, queryClient]);

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
  });

  const overview = overviewQuery.data;
  const visibleDefinitions = useMemo(
    () => filterExtensionDefinitions(overview?.definitions ?? [], overview?.statuses ?? [], searchTerm, t),
    [overview, searchTerm, t],
  );
  const installed = overview?.statuses.filter((status) => status.installed).length ?? 0;
  const running = overview?.statuses.filter((status) => status.running).length ?? 0;
  const errors = overview?.statuses.filter((status) => status.status === "error").length ?? 0;
  const nativeOperationsAvailable = overview?.environment.nativeOperationsAvailable;
  const nativeAvailable = nativeOperationsAvailable === true;

  // Task 12.18: this page's own overviewQuery projected into the shared AsyncBoundary's
  // AsyncViewState shape -- src/ui/ primitives cannot import this service's own error type
  // (ARCH-FE-005), so the projection lives here rather than in the primitive.
  const asyncState: AsyncViewState<ExtensionOverview> = {
    data: overview,
    error: overviewQuery.isError
      ? { kind: "error", message: errorMessage(overviewQuery.error), retryable: true }
      : undefined,
    initialLoading: overviewQuery.isLoading,
    refreshing: overviewQuery.isFetching && !overviewQuery.isLoading,
    stale: overviewQuery.isStale,
  };

  // Task 12.16: the same conditions already rendered on screen -- the native-availability banner
  // and the error stat card -- reported so this page's own nav entry can flag them while the user
  // looks at a different page. `overviewQuery.isError` replaces the old page-level `error` string
  // here: a per-action failure now surfaces through `stateFor`/`MutationStatus` on its own card
  // (task 12.18) instead of a shared banner, and `errors` (a framework's own persisted "error"
  // status) already eventually reflects an operation failure too, once the invalidate below
  // refetches it.
  useEffect(() => {
    onStatusChange?.(pickPageStatus([
      // "extensions.status.error" already names a per-framework status label -- this is the
      // different, page-level condition, so it uses its own "pageStatus" namespace instead of colliding.
      overviewQuery.isError || errors > 0 ? { kind: "error", labelKey: "extensions.pageStatus.error" } : null,
      nativeOperationsAvailable === false
        ? { kind: "dependency-unavailable", labelKey: "extensions.status.nativeUnavailable" }
        : null,
    ]));
    return () => onStatusChange?.(null);
  }, [overviewQuery.isError, errors, nativeOperationsAvailable, onStatusChange]);

  async function openPreview(frameworkId: ExtensionFrameworkId) {
    setPreviewError(null);
    try {
      setPreview(await service.getInstallPreview({ frameworkId }));
    } catch (reason) {
      setPreviewError(errorMessage(reason));
    }
  }

  async function runAction(action: string, frameworkId: ExtensionFrameworkId) {
    await operationMutation.mutateAsync({ action, frameworkId }).catch(() => undefined);
  }

  /** Task 12.18: projects this page's own single, shared `operationMutation` (react-query already
   *  tracks `variables`/`isPending`/`error` for its own most recent call) plus the polled
   *  `activeOperation` lifecycle it kicks off, into the shared `MutationState` shape, keyed to one
   *  framework at a time -- a registry alongside these two would just be a third source of truth
   *  for the same fact, and this page never tracks more than one operation in flight at once (a
   *  single `activeOperation` slot is all it has ever held). */
  function stateFor(frameworkId: ExtensionFrameworkId): MutationState | undefined {
    if (operationMutation.variables?.frameworkId === frameworkId) {
      if (operationMutation.isPending) return { pending: true, targetKey: frameworkId };
      if (operationMutation.isError) {
        return {
          error: { kind: "error", message: errorMessage(operationMutation.error), retryable: false },
          pending: false,
          targetKey: frameworkId,
        };
      }
    }
    if (activeOperation?.relatedEntityId === frameworkId) {
      if (activeOperation.status === "queued" || activeOperation.status === "running") {
        return { operationId: activeOperation.id, pending: true, targetKey: frameworkId };
      }
      if (activeOperation.status === "failed") {
        return {
          error: {
            kind: "error",
            message: activeOperation.error ?? t("extensions.error.operationFailed"),
            retryable: false,
          },
          operationId: activeOperation.id,
          pending: false,
          targetKey: frameworkId,
        };
      }
    }
    return undefined;
  }

  const emptyStateSlot = { title: t("extensions.empty") };

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("extensions.description")}
        icon={Cpu}
        primaryAction={
          <Button disabled={overviewQuery.isFetching} onClick={() => void overviewQuery.refetch()} variant="outline">
            <RefreshCw className={overviewQuery.isFetching ? "animate-spin" : ""} />
            {t("extensions.refresh")}
          </Button>
        }
        title={t("extensions.title")}
      />

      {overview && !overview.environment.nativeOperationsAvailable ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning">{t("extensions.environment.desktopOnly")}</div>
      ) : null}
      {previewError ? <div className="rounded-md border p-3 text-sm ucd-status-danger">{previewError}</div> : null}

      <div className="grid gap-3 md:grid-cols-3">
        <StatCard hint={t("extensions.stats.installedHint")} icon={Box} label={t("extensions.stats.installed")} value={String(installed)} />
        <StatCard hint={t("extensions.stats.runningHint")} icon={Activity} label={t("extensions.stats.running")} value={String(running)} />
        <StatCard hint={t("extensions.stats.errorsHint")} icon={Cpu} label={t("extensions.stats.errors")} value={String(errors)} />
      </div>

      <SectionPanel description={t("extensions.list.description")} title={t("extensions.list.title")} variant="plain">
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
              {visibleDefinitions.map((definition) => {
                const status = overview?.statuses.find((item) => item.frameworkId === definition.id);
                if (!status) return null;
                return (
                  <ExtensionFrameworkCard
                    activeOperation={activeOperation?.relatedEntityId === definition.id ? activeOperation : undefined}
                    definition={definition}
                    key={definition.id}
                    mutationState={stateFor(definition.id)}
                    nativeAvailable={nativeAvailable}
                    onOpenPreview={(frameworkId) => void openPreview(frameworkId)}
                    onRunAction={(action, frameworkId) => void runAction(action, frameworkId)}
                    status={status}
                  />
                );
              })}
            </div>
          )}
        </AsyncBoundary>
      </SectionPanel>

      {preview ? (
        <ExtensionInstallPreviewDialog
          nativeAvailable={nativeAvailable}
          onClose={() => setPreview(null)}
          onInstall={() => {
            setPreview(null);
            void runAction("install", preview.frameworkId);
          }}
          preview={preview}
        />
      ) : null}
    </div>
  );
}
