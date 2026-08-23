import { useTranslation } from "react-i18next";
import type { CliEnvironmentSnapshot } from "../../../types/cli-environment-snapshot";
import type { OperationTask } from "../../../types/operation";
import { CliEnvironmentCard } from "./cli-environment-card";

/**
 * The card grid.
 *
 * A separate component so the page owns state and this owns layout, and so the empty case has one
 * place to live rather than being a conditional inside a map.
 */
export function CliEnvironmentList({
  snapshots,
  selectedVersions,
  operations,
  refreshingAgentIds,
  mutatingAgentIds,
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
  onSelectedVersionChange: (agentId: string, version: string) => void;
  onRefresh: (agentId: string) => void;
  onRequestChange: (snapshot: CliEnvironmentSnapshot, targetVersion: string) => void;
  onOpenDetails: (agentId: string) => void;
  onCancelOperation: (agentId: string) => void;
}) {
  const { t } = useTranslation();

  if (snapshots.length === 0) {
    return (
      <p className="ucd-panel rounded-lg p-6 text-center text-sm text-muted-foreground">
        {t("cli.list.empty")}
      </p>
    );
  }

  return (
    <div className="grid gap-4 xl:grid-cols-2">
      {snapshots.map((snapshot) => (
        <CliEnvironmentCard
          key={snapshot.agentId}
          mutating={mutatingAgentIds.has(snapshot.agentId)}
          operation={operations[snapshot.agentId]}
          refreshing={refreshingAgentIds.has(snapshot.agentId)}
          selectedVersion={selectedVersions[snapshot.agentId] ?? ""}
          snapshot={snapshot}
          onCancelOperation={() => onCancelOperation(snapshot.agentId)}
          onOpenDetails={() => onOpenDetails(snapshot.agentId)}
          onRefresh={() => onRefresh(snapshot.agentId)}
          onRequestChange={(version) => onRequestChange(snapshot, version)}
          onSelectedVersionChange={(version) => onSelectedVersionChange(snapshot.agentId, version)}
        />
      ))}
    </div>
  );
}
