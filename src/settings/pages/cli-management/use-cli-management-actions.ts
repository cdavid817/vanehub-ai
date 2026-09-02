import { useState } from "react";
import { useMutation, useQuery, type QueryClient } from "@tanstack/react-query";
import { agentService } from "../../../services/runtime-agent-client";
import { settingsService } from "../../../services/runtime-settings-client";
import { readCliRejection, type CliBulkItemResult, type CliRejection } from "../../../types/cli-environment";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import { recommendedSourceId } from "./cli-management-presenters";
import { useCliOperationTracking } from "./use-cli-operation-tracking";

/**
 * Every mutation this page can start, the two plan queries they feed, and the operation tracking
 * both drive.
 *
 * Extracted out of the page component (task 12.18's own extraction pass): five mutations plus two
 * dependent queries is a materially bigger orchestration surface than SSH/Extensions/Plugins ever
 * had (each had one or two dominant mutations), and inlining all of it is what pushed
 * `cli-management-page.tsx` to this repo's 300-line ceiling with no room left for the primitive
 * migration itself. The page keeps exactly one piece of state this hook does not:
 * `selectedVersions`, which is per-card controlled `<select>` state, not mutation orchestration.
 */
export function useCliManagementActions(
  snapshots: readonly CliEnvironmentSnapshot[],
  queryClient: QueryClient,
  snapshotsQueryKey: readonly unknown[],
) {
  const tracking = useCliOperationTracking(snapshots, queryClient, snapshotsQueryKey);
  const [planId, setPlanId] = useState<string | null>(null);
  const [bulkPlanId, setBulkPlanId] = useState<string | null>(null);
  const [bulkResults, setBulkResults] = useState<readonly CliBulkItemResult[] | null>(null);
  const [planRejection, setPlanRejection] = useState<CliRejection | null>(null);

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

  /** The details drawer's own "rerun" action -- same tracking and failure reporting as a mutation,
   *  kept a plain async call because it has no plan/review step of its own to gate on. */
  function rerunDiagnostics(agentId: string) {
    void agentService.runCliDoctor(agentId)
      .then((operation) => tracking.trackMutation(operation, agentId))
      .catch((error: unknown) => reportFailure("CliManagementPage.runCliDoctor", error));
  }

  return {
    bulkExecuteMutation,
    bulkPlanQuery,
    bulkPrepareMutation,
    bulkResults,
    executeMutation,
    planQuery,
    planRejection,
    prepareMutation,
    refreshMutation,
    tracking,
    closeBulkPlan() {
      setBulkPlanId(null);
      setBulkResults(null);
    },
    closePlan() {
      setPlanRejection(null);
      setPlanId(null);
    },
    rerunDiagnostics,
  };
}
