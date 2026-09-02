import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import { CliEnvironmentCard } from "./cli-environment-card";

/**
 * The card grid.
 *
 * A separate component so the page owns state and this owns layout. Emptiness (genuinely no tools,
 * or a filter matching none) is the page's own shared `AsyncBoundary`'s job now (task 12.18) --
 * this component only ever receives a non-empty list.
 */
export function CliEnvironmentList({
  snapshots,
  selectedVersions,
  operations,
  refreshingAgentIds,
  mutatingAgentIds,
  detailsAgentId,
  detailsPanelId,
  onSelectedVersionChange,
  onRefresh,
  onRequestChange,
  onOpenDetails,
  onCancelOperation,
}: {
  snapshots: readonly CliEnvironmentSnapshot[];
  selectedVersions: Record<string, string>;
  operations: Record<string, OperationTask | undefined>;
  refreshingAgentIds: ReadonlySet<string>;
  mutatingAgentIds: ReadonlySet<string>;
  detailsAgentId: string | null;
  detailsPanelId: string;
  onSelectedVersionChange: (agentId: string, version: string) => void;
  onRefresh: (agentId: string) => void;
  onRequestChange: (
    snapshot: CliEnvironmentSnapshot,
    targetVersion: string,
    trigger: HTMLElement,
  ) => void;
  onOpenDetails: (agentId: string, trigger: HTMLElement) => void;
  onCancelOperation: (agentId: string) => void;
}) {
  return (
    <div className="grid gap-4 xl:grid-cols-2">
      {snapshots.map((snapshot) => (
        <CliEnvironmentCard
          detailsOpen={detailsAgentId === snapshot.agentId}
          detailsPanelId={detailsPanelId}
          key={snapshot.agentId}
          mutating={mutatingAgentIds.has(snapshot.agentId)}
          operation={operations[snapshot.agentId]}
          refreshing={refreshingAgentIds.has(snapshot.agentId)}
          selectedVersion={selectedVersions[snapshot.agentId] ?? ""}
          snapshot={snapshot}
          onCancelOperation={() => onCancelOperation(snapshot.agentId)}
          onOpenDetails={(trigger) => onOpenDetails(snapshot.agentId, trigger)}
          onRefresh={() => onRefresh(snapshot.agentId)}
          onRequestChange={(version, trigger) => onRequestChange(snapshot, version, trigger)}
          onSelectedVersionChange={(version) => onSelectedVersionChange(snapshot.agentId, version)}
        />
      ))}
    </div>
  );
}
