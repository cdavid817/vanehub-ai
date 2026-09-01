import { useEffect, useId, useMemo, useRef, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { TerminalSquare } from "lucide-react";
import { useTranslation } from "react-i18next";
import { orderByAgentPriority } from "../../../lib/agent-display-order";
import { agentService } from "../../../services/runtime-agent-client";
import { settingsService } from "../../../services/runtime-settings-client";
import { readCliRejection, type CliBulkItemResult, type CliRejection } from "../../../types/cli-environment";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import type { SettingsPageStatus } from "../../settings-page-types";
import { PageHeader } from "../page-parts";
import { CliActionPlanDialog } from "./cli-action-plan-dialog";
import { CliBulkPlanDialog } from "./cli-bulk-plan-dialog";
import { CliDetailsDrawer } from "./cli-details-drawer";
import { CliEnvironmentList } from "./cli-environment-list";
import { CliSummaryBar } from "./cli-summary-bar";
import { CliToolbar } from "./cli-toolbar";
import { isOperationRunning } from "./cli-operation-status";
import {
  availableSourceIds,
  bulkUpgradeEligible,
  filterSnapshots,
  recommendedSourceId,
  summaryCounts,
} from "./cli-management-presenters";
import { useCliManagementPageStatus } from "./use-cli-management-page-status";
import { useCliOperationTracking } from "./use-cli-operation-tracking";

const cliEnvironmentsQueryKey = ["cli-environments"] as const;

export function refreshButtonState(isPending: boolean, operation?: OperationTask) {
  const running = isPending || isOperationRunning(operation);
  return {
    disabled: running,
    labelKey: running ? "cli.refreshing" : "cli.refresh",
    iconClassName: `h-4 w-4 ${running ? "animate-spin" : ""}`,
  };
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
  const [planId, setPlanId] = useState<string | null>(null);
  const [bulkPlanId, setBulkPlanId] = useState<string | null>(null);
  const [bulkResults, setBulkResults] = useState<readonly CliBulkItemResult[] | null>(null);
  const [planRejection, setPlanRejection] = useState<CliRejection | null>(null);
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

  const tracking = useCliOperationTracking(snapshots, queryClient, cliEnvironmentsQueryKey);

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

  function reportFailure(source: string, error: unknown, details?: Record<string, string>) {
    void settingsService.reportClientLogEvent({
      level: "error",
      kind: "critical-operation-failure",
      message: String(error),
      source,
      details,
    });
  }

  const refreshMutation = useMutation({
    mutationFn: (agentId: string | null) =>
      agentService.refreshCliEnvironments(agentId ? [agentId] : [], false),
    onSuccess: (operation, agentId) => tracking.trackRefresh(operation, agentId, snapshots),
    onError: (error, agentId) =>
      reportFailure("CliManagementPage.refresh", error, agentId ? { agentId } : undefined),
  });

  const prepareMutation = useMutation({
    mutationFn: ({ snapshot, targetVersion }: {
      snapshot: CliEnvironmentSnapshot;
      targetVersion: string;
    }) =>
      agentService.prepareCliAction({
        agentId: snapshot.agentId,
        // No action: the backend derives install/upgrade/downgrade from the target and what is
        // installed, so this page never compares two versions.
        action: null,
        sourceId: recommendedSourceId(snapshot) ?? "",
        targetVersion,
        channel: null,
      }),
    onSuccess: async (operation, variables) => {
      tracking.trackMutation(operation, variables.snapshot.agentId);
      const prepared = await tracking.awaitPlanId(operation.id);
      if (prepared) setPlanId(prepared);
    },
    onError: (error, variables) =>
      reportFailure("CliManagementPage.prepareCliAction", error, {
        agentId: variables.snapshot.agentId,
        targetVersion: variables.targetVersion,
      }),
  });

  const executeMutation = useMutation({
    mutationFn: (input: { planId: string; expectedRevision: number }) =>
      agentService.executeCliAction(input),
    onSuccess: (operation) => {
      tracking.trackMutation(operation, operation.relatedEntityId ?? null);
      setPlanRejection(null);
      setPlanId(null);
    },
    onError: (error) => {
      // A refusal stays on screen. Closing the dialog silently would leave the user believing the
      // change ran, which is the one thing a refusal guarantees did not happen.
      setPlanRejection(readCliRejection(error));
      reportFailure("CliManagementPage.executeCliAction", error);
    },
  });

  const bulkPrepareMutation = useMutation({
    mutationFn: (agentIds: string[]) => agentService.prepareCliBulkUpgrade(agentIds),
    onSuccess: async (operation) => {
      const prepared = await tracking.awaitPlanId(operation.id);
      if (prepared) {
        setBulkResults(null);
        setBulkPlanId(prepared);
      }
    },
    onError: (error) => reportFailure("CliManagementPage.prepareCliBulkUpgrade", error),
  });

  const bulkExecuteMutation = useMutation({
    mutationFn: (input: { planId: string; expectedRevision: number }) =>
      agentService.executeCliBulkAction(input),
    onSuccess: async (operation) => {
      const items = await tracking.awaitBulkItems(operation.id);
      setBulkResults(items);
    },
    onError: (error) => reportFailure("CliManagementPage.executeCliBulkAction", error),
  });

  const planQuery = useQuery({
    queryKey: ["cli-action-plan", planId],
    queryFn: () => agentService.getCliActionPlan(planId ?? ""),
    enabled: planId !== null,
  });
  const bulkPlanQuery = useQuery({
    queryKey: ["cli-bulk-plan", bulkPlanId],
    queryFn: () => agentService.getCliBulkActionPlan(bulkPlanId ?? ""),
    enabled: bulkPlanId !== null,
  });

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
        bulkPending={bulkPrepareMutation.isPending}
        refreshing={refreshMutation.isPending && refreshMutation.variables === null}
        search={search}
        sourceFilter={sourceFilter}
        sourceIds={availableSourceIds(snapshots)}
        onAttentionOnlyChange={setAttentionOnly}
        onBulkUpgrade={() => bulkPrepareMutation.mutate(bulkEligible.map((s) => s.agentId))}
        onRefreshAll={() => refreshMutation.mutate(null)}
        onSearchChange={setSearch}
        onSourceFilterChange={setSourceFilter}
      />
      {snapshotsQuery.error ? (
        <div className="rounded-md border p-3 text-sm ucd-status-warning" role="alert">
          {String(snapshotsQuery.error)}
        </div>
      ) : null}
      <CliEnvironmentList
        detailsAgentId={detailsAgentId}
        detailsPanelId={detailsPanelId}
        mutatingAgentIds={tracking.mutatingAgentIds}
        operations={tracking.operationsByAgentId}
        refreshingAgentIds={tracking.refreshingAgentIds}
        selectedVersions={selectedVersions}
        snapshots={visible}
        onCancelOperation={(agentId) => tracking.cancel(agentId)}
        onOpenDetails={(agentId, trigger) => {
          triggerRef.current = trigger;
          setDetailsAgentId(agentId);
        }}
        onRefresh={(agentId) => refreshMutation.mutate(agentId)}
        onRequestChange={(snapshot, targetVersion, trigger) => {
          triggerRef.current = trigger;
          prepareMutation.mutate({ snapshot, targetVersion });
        }}
        onSelectedVersionChange={(agentId, version) =>
          setSelectedVersions((current) => ({ ...current, [agentId]: version }))}
      />

      {detailsSnapshot ? (
        <CliDetailsDrawer
          diagnosticsRunning={tracking.mutatingAgentIds.has(detailsSnapshot.agentId)}
          operation={tracking.operationsByAgentId[detailsSnapshot.agentId]}
          panelId={detailsPanelId}
          returnFocus={triggerRef.current}
          snapshot={detailsSnapshot}
          onCancelOperation={() => tracking.cancel(detailsSnapshot.agentId)}
          onClose={() => setDetailsAgentId(null)}
          onRerunDiagnostics={() => {
            void agentService.runCliDoctor(detailsSnapshot.agentId)
              .then((operation) => tracking.trackMutation(operation, detailsSnapshot.agentId))
              .catch((error: unknown) => reportFailure("CliManagementPage.runCliDoctor", error));
          }}
        />
      ) : null}

      {planQuery.data ? (
        <CliActionPlanDialog
          displayName={
            snapshots.find((snapshot) => snapshot.agentId === planQuery.data?.agentId)?.displayName
            ?? planQuery.data.agentId
          }
          plan={planQuery.data}
          rejection={planRejection}
          returnFocus={triggerRef.current}
          submitting={executeMutation.isPending}
          onCancel={() => {
            setPlanRejection(null);
            setPlanId(null);
          }}
          onConfirm={(input) => executeMutation.mutate(input)}
          onPrepareAgain={() => {
            setPlanRejection(null);
            setPlanId(null);
          }}
        />
      ) : null}

      {bulkPlanQuery.data ? (
        <CliBulkPlanDialog
          displayNames={Object.fromEntries(
            snapshots.map((snapshot) => [snapshot.agentId, snapshot.displayName]),
          )}
          plan={bulkPlanQuery.data}
          results={bulkResults}
          returnFocus={triggerRef.current}
          submitting={bulkExecuteMutation.isPending}
          onClose={() => {
            setBulkPlanId(null);
            setBulkResults(null);
          }}
          onConfirm={(input) => bulkExecuteMutation.mutate(input)}
        />
      ) : null}
    </div>
  );
}
