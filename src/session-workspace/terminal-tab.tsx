import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { ChatMessage } from "../types/chat";
import type { ExecutionRecord } from "../types/session-workspace-evidence";
import { ArtifactPanel } from "./artifact-panel";
import { BuiltinToolActivity } from "./builtin-tool-activity";
import { DelegationPanel } from "./delegation-panel";
import { ExecutionRecordDetailDrawer } from "./execution-record-detail-drawer";
import { ExecutionRecordList } from "./execution-record-list";
import { ExecutionRecordToolbar } from "./execution-record-toolbar";
import {
  EMPTY_FILTERS,
  hasActiveFilters,
  isLegacyView,
  queryFilters,
  type ExecutionRecordFilterState,
  type ExecutionRecordView,
} from "./execution-record-view";
import { LegacyActivityList } from "./legacy-activity-list";
import {
  CoverageNotice,
  emptyStateFor,
  PageErrorNotice,
  TerminalHistoryEmpty,
} from "./terminal-history-states";
import { useExecutionRecordPages } from "./use-execution-record-pages";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";
import { WorkspaceState } from "./workspace-state";

export { toolUseCount } from "./terminal-utils";

/**
 * Terminal History, composed rather than implemented.
 *
 * Everything this file used to do — reading `message.toolUse`, deciding what a row says, deciding
 * which fields exist — now lives in a module that can be tested without a DOM. What is left is the
 * arrangement: which view is selected, which record is open, and where the two lists go.
 */
export function TerminalTab({
  builtinToolsAvailable = false,
  isVisible = true,
  messages,
  partial,
  seatId = null,
  sessionId = null,
  targetRoot = "",
}: {
  builtinToolsAvailable?: boolean;
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  messages: ChatMessage[];
  partial: boolean;
  seatId?: string | null;
  sessionId?: string | null;
  targetRoot?: string;
}) {
  const { t } = useTranslation();
  const { scope } = useWorkspaceEvidenceScope();
  const [view, setView] = useState<ExecutionRecordView>("all");
  const [filters, setFilters] = useState<ExecutionRecordFilterState>(EMPTY_FILTERS);
  const [selected, setSelected] = useState<ExecutionRecord | null>(null);

  const evidenceSessionId = useMemo(() => {
    const parsed = evidenceSessionIdSchema.safeParse(sessionId);
    return parsed.success ? parsed.data : null;
  }, [sessionId]);

  // The seat the tab is showing narrows the query, but only where the panel's own switcher says
  // so: the cross-panel scope owns everything else.
  const recordScope = useMemo(() => {
    if (scope === null || isLegacyView(view)) return null;
    const parsed = seatId === null ? null : scope.seatId;
    return parsed === null ? scope : { ...scope, seatId: parsed };
  }, [scope, seatId, view]);

  const query = useMemo(() => queryFilters(view, filters), [filters, view]);
  const pages = useExecutionRecordPages({
    filters: query,
    isVisible,
    scope: recordScope,
  });

  if (!sessionId || evidenceSessionId === null) return <WorkspaceState kind="unavailable" />;

  const emptyState = emptyStateFor({
    coverage: pages.coverage,
    filtered: hasActiveFilters(filters),
    hasError: pages.initialError !== null,
    loading: pages.loading,
  });

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      {builtinToolsAvailable ? (
        <div className="grid shrink-0 gap-3 overflow-y-auto">
          <BuiltinToolActivity isVisible={isVisible} sessionId={sessionId} />
          <ArtifactPanel sessionId={sessionId} />
          <DelegationPanel defaultTargetRoot={targetRoot} sessionId={sessionId} />
        </div>
      ) : null}
      <ExecutionRecordToolbar
        filters={filters}
        onFiltersChange={setFilters}
        onViewChange={(next) => {
          setView(next);
          setSelected(null);
        }}
        view={view}
      />
      <div className="grid min-h-0 flex-1 gap-3 lg:grid-cols-[minmax(0,1fr)_minmax(260px,0.42fr)]">
        <div className="flex min-h-0 flex-col gap-2">
          {isLegacyView(view) ? (
            <LegacyActivityList
              messages={messages}
              messagesPartial={partial}
              onSelect={setSelected}
              seatId={seatId}
              selectedId={selected?.id ?? null}
              sessionId={evidenceSessionId}
            />
          ) : (
            <>
              <CoverageNotice coverage={pages.coverage} />
              {pages.pageError === null ? null : (
                <PageErrorNotice message={t(pages.pageError)} onRetry={() => void pages.retry()} />
              )}
              {pages.records.length === 0 ? (
                <TerminalHistoryEmpty state={emptyState} />
              ) : (
                <div className="min-h-0 flex-1">
                  <ExecutionRecordList
                    ariaLabel={t("executionRecords.listLabel")}
                    hasMore={pages.hasMore}
                    loading={pages.loading}
                    onLoadMore={() => void pages.loadMore()}
                    onSelect={setSelected}
                    records={pages.records}
                    selectedId={selected?.id ?? null}
                  />
                </div>
              )}
            </>
          )}
        </div>
        {selected === null ? null : (
          <ExecutionRecordDetailDrawer onClose={() => setSelected(null)} record={selected} />
        )}
      </div>
    </div>
  );
}
