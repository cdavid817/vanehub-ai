import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { TerminalSquare } from "lucide-react";
import { useTranslation } from "react-i18next";
import { orderByAgentPriority } from "../../../lib/agent-display-order";
import { agentService } from "../../../services/runtime-agent-client";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { SettingsPageStatus } from "../../settings-page-types";
import { AsyncBoundary } from "../../../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../../../ui/async/async-view-state";
import { PageHeader } from "../../../ui/page-header/PageHeader";
import { CliActionPlanDialog } from "./cli-action-plan-dialog";
import { CliBulkPlanDialog } from "./cli-bulk-plan-dialog";
import { CliDetailsDrawer } from "./cli-details-drawer";
import { CliEnvironmentList } from "./cli-environment-list";
import { CliSummaryBar } from "./cli-summary-bar";
import { CliToolbar } from "./cli-toolbar";
import {
  availableSourceIds,
  bulkUpgradeEligible,
  filterSnapshots,
  summaryCounts,
} from "./cli-management-presenters";
import { useCliManagementActions } from "./use-cli-management-actions";
import { useCliManagementPageStatus } from "./use-cli-management-page-status";

const cliEnvironmentsQueryKey = ["cli-environments"] as const;

function errorMessage(reason: unknown) {
  return reason instanceof Error ? reason.message : String(reason);
}

export function CliManagementPage({ onStatusChange, searchTerm }: { onStatusChange?: (status: SettingsPageStatus | null) => void; searchTerm: string }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [search, setSearch] = useState("");
  const [stateFilter, setStateFilter] = useState("all");
  const [sourceFilter, setSourceFilter] = useState("all");
  const [attentionOnly, setAttentionOnly] = useState(false);
  const [selectedVersions, setSelectedVersions] = useState<Record<string, string>>({});
  const [detailsAgentId, setDetailsAgentId] = useState<string | null>(null);
  const triggerRef = useRef<HTMLElement | null>(null);
  const detailsPanelId = `${useId()}-cli-details`;

  const snapshotsQuery = useQuery({
    queryKey: cliEnvironmentsQueryKey,
    queryFn: () => agentService.listCliEnvironments(),
    // Cached data stays on screen while a background refresh runs. A blank list would read as
    // "nothing is installed" for as long as the probes take.
    placeholderData: (previous) => previous,
  });
  const snapshots = useMemo(
    () => orderByAgentPriority(snapshotsQuery.data ?? [], (snapshot) => snapshot.agentId),
    [snapshotsQuery.data],
  );

  const actions = useCliManagementActions(snapshots, queryClient, cliEnvironmentsQueryKey);

  useEffect(() => {
    setSelectedVersions((current) => {
      const next = { ...current };
      for (const snapshot of snapshots) {
        // The backend's own default for the offered action. Never a version this page picked.
        if (!next[snapshot.agentId]) {
          next[snapshot.agentId] = snapshot.allowedActions[0]?.defaultTarget ?? "";
        }
      }
      return next;
    });
  }, [snapshots]);

  const effectiveSearch = search || searchTerm;
  const visible = useMemo(
    () => filterSnapshots(snapshots, {
      search: effectiveSearch,
      state: stateFilter,
      source: sourceFilter,
      attentionOnly,
    }),
    [snapshots, effectiveSearch, stateFilter, sourceFilter, attentionOnly],
  );
  const counts = useMemo(() => summaryCounts(snapshots), [snapshots]);
  useCliManagementPageStatus({ error: snapshotsQuery.error, onStatusChange, updateCount: counts.updates });
  const bulkEligible = useMemo(() => snapshots.filter(bulkUpgradeEligible), [snapshots]);
  const detailsSnapshot = snapshots.find((snapshot) => snapshot.agentId === detailsAgentId);
  const filtersActive = Boolean(effectiveSearch.trim()) || stateFilter !== "all" || sourceFilter !== "all" || attentionOnly;

  // task 12.18: this page's own snapshotsQuery projected into the shared AsyncBoundary's
  // AsyncViewState shape -- src/ui/ primitives cannot import this service's own error type
  // (ARCH-FE-005), so the projection lives here rather than in the primitive. Only routed into
  // AsyncBoundary's own full-screen error state when there is no data at all to fall back on --
  // with `placeholderData` above keeping the last-known list on screen across a refresh, replacing
  // it with a full-screen error on every transient refetch failure would regress this page's own
  // deliberate "never blank the list" intent, so a refetch-time failure keeps using the narrower
  // inline warning below instead, exactly as it did before this migration.
  const asyncState: AsyncViewState<CliEnvironmentSnapshot[]> = {
    data: snapshotsQuery.data,
    error: snapshotsQuery.isError && snapshotsQuery.data === undefined
      ? { kind: "error", message: errorMessage(snapshotsQuery.error), retryable: true }
      : undefined,
    initialLoading: snapshotsQuery.isLoading,
    refreshing: snapshotsQuery.isFetching && !snapshotsQuery.isLoading,
    stale: snapshotsQuery.isStale,
  };
  // Same message and action whether the catalog is genuinely empty or a filter just matched
  // nothing, matching this page's own pre-existing behavior -- AsyncBoundary still picks the
  // right icon/variant (Inbox vs. SearchX) for each case on its own.
  const emptyStateSlot = { title: t("cli.list.empty") };

  return (
    <div className="space-y-4">
      <PageHeader
        description={t("cli.description")}
        icon={TerminalSquare}
        title={t("cli.title")}
      />
      <CliSummaryBar activeState={stateFilter} counts={counts} onSelectState={setStateFilter} />
      <CliToolbar
        attentionOnly={attentionOnly}
        bulkEligibleCount={bulkEligible.length}
        bulkPending={actions.bulkPrepareMutation.isPending}
        refreshing={actions.refreshMutation.isPending && actions.refreshMutation.variables === null}
        search={search}
        sourceFilter={sourceFilter}
        sourceIds={availableSourceIds(snapshots)}
        onAttentionOnlyChange={setAttentionOnly}
        onBulkUpgrade={() => actions.bulkPrepareMutation.mutate(bulkEligible.map((s) => s.agentId))}
        onRefreshAll={() => actions.refreshMutation.mutate(null)}
        onSearchChange={setSearch}
        onSourceFilterChange={setSourceFilter}
      />
      {snapshotsQuery.isError && snapshotsQuery.data !== undefined ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">
          {errorMessage(snapshotsQuery.error)}
        </div>
      ) : null}
      <AsyncBoundary
        emptyState={emptyStateSlot}
        filtered={filtersActive}
        filteredEmptyState={emptyStateSlot}
        isEmpty={() => visible.length === 0}
        onRetry={() => void snapshotsQuery.refetch()}
        state={asyncState}
      >
        {() => (
          <CliEnvironmentList
            detailsAgentId={detailsAgentId}
            detailsPanelId={detailsPanelId}
            mutatingAgentIds={actions.tracking.mutatingAgentIds}
            operations={actions.tracking.operationsByAgentId}
            refreshingAgentIds={actions.tracking.refreshingAgentIds}
            selectedVersions={selectedVersions}
            snapshots={visible}
            onCancelOperation={(agentId) => actions.tracking.cancel(agentId)}
            onOpenDetails={(agentId, trigger) => {
              triggerRef.current = trigger;
              setDetailsAgentId(agentId);
            }}
            onRefresh={(agentId) => actions.refreshMutation.mutate(agentId)}
            onRequestChange={(snapshot, targetVersion, trigger) => {
              triggerRef.current = trigger;
              actions.prepareMutation.mutate({ snapshot, targetVersion });
            }}
            onSelectedVersionChange={(agentId, version) =>
              setSelectedVersions((current) => ({ ...current, [agentId]: version }))}
          />
        )}
      </AsyncBoundary>

      {detailsSnapshot ? (
        <CliDetailsDrawer
          diagnosticsRunning={actions.tracking.mutatingAgentIds.has(detailsSnapshot.agentId)}
          operation={actions.tracking.operationsByAgentId[detailsSnapshot.agentId]}
          panelId={detailsPanelId}
          returnFocus={triggerRef.current}
          snapshot={detailsSnapshot}
          onCancelOperation={() => actions.tracking.cancel(detailsSnapshot.agentId)}
          onClose={() => setDetailsAgentId(null)}
          onRerunDiagnostics={() => actions.rerunDiagnostics(detailsSnapshot.agentId)}
        />
      ) : null}

      {actions.planQuery.data ? (
        <CliActionPlanDialog
          displayName={
            snapshots.find((snapshot) => snapshot.agentId === actions.planQuery.data?.agentId)?.displayName
            ?? actions.planQuery.data.agentId
          }
          plan={actions.planQuery.data}
          rejection={actions.planRejection}
          returnFocus={triggerRef.current}
          submitting={actions.executeMutation.isPending}
          onCancel={actions.closePlan}
          onConfirm={(input) => actions.executeMutation.mutate(input)}
          onPrepareAgain={actions.closePlan}
        />
      ) : null}

      {actions.bulkPlanQuery.data ? (
        <CliBulkPlanDialog
          displayNames={Object.fromEntries(
            snapshots.map((snapshot) => [snapshot.agentId, snapshot.displayName]),
          )}
          plan={actions.bulkPlanQuery.data}
          results={actions.bulkResults}
          returnFocus={triggerRef.current}
          submitting={actions.bulkExecuteMutation.isPending}
          onClose={actions.closeBulkPlan}
          onConfirm={(input) => actions.bulkExecuteMutation.mutate(input)}
        />
      ) : null}
    </div>
  );
}
