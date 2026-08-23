import { useCallback, useEffect, useRef, useState } from "react";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  EvidenceCursor,
  ExecutionRecord,
  ExecutionRecordFilters,
  QueryCoverage,
  WorkspaceEvidenceScope,
} from "../types/session-workspace-evidence";
import { EVIDENCE_PAGE_LIMITS } from "../types/session-workspace-evidence";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

export interface ExecutionRecordPagesInput {
  scope: WorkspaceEvidenceScope | null;
  filters: ExecutionRecordFilters;
  /** False while the panel stays mounted behind another tab. Defers reads, keeps rows. */
  isVisible?: boolean;
  /** Bumped by a live notice to re-read the first page without discarding what is on screen. */
  refreshToken?: number;
  service?: SessionWorkspaceEvidenceService;
}

export interface ExecutionRecordPagesState {
  records: ExecutionRecord[];
  coverage: QueryCoverage | null;
  hasMore: boolean;
  loading: boolean;
  /** Blocks: there is nothing to look at yet. */
  initialError: WorkspaceErrorKey | null;
  /** Does not block: rows are on screen and a later request failed. */
  pageError: WorkspaceErrorKey | null;
  loadMore: () => Promise<void>;
  retry: () => Promise<void>;
}

/**
 * Merges a page into what is already loaded, keyed by record id.
 *
 * Two things arrive through here that a plain append would get wrong. A record that was running
 * when it was first read and has since finished comes back with the same id and a terminal status:
 * it replaces the row rather than adding a second one under the same identity. And a keyset page
 * fetched across a concurrent append can legitimately repeat a boundary row, which an append would
 * show twice.
 */
export function mergeRecordPage(
  loaded: readonly ExecutionRecord[],
  page: readonly ExecutionRecord[],
): ExecutionRecord[] {
  const merged = loaded.slice();
  const positions = new Map(merged.map((record, index) => [record.id, index]));
  for (const record of page) {
    const at = positions.get(record.id);
    if (at === undefined) {
      positions.set(record.id, merged.length);
      merged.push(record);
      continue;
    }
    merged[at] = record;
  }
  return merged;
}

/**
 * One bounded page at a time, newest first, with the rows already on screen never thrown away by a
 * failure.
 *
 * The continuation token is held rather than recomputed: a retry has to resume from the boundary
 * the failed request used, and re-deriving one from the loaded rows is how a keyset pager turns
 * back into offset arithmetic and starts skipping across a concurrent append.
 */
export function useExecutionRecordPages({
  filters,
  isVisible = true,
  refreshToken = 0,
  scope,
  service = defaultAgentService,
}: ExecutionRecordPagesInput): ExecutionRecordPagesState {
  const [records, setRecords] = useState<ExecutionRecord[]>([]);
  const [coverage, setCoverage] = useState<QueryCoverage | null>(null);
  const [cursor, setCursor] = useState<EvidenceCursor | null>(null);
  const [loading, setLoading] = useState(false);
  const [initialError, setInitialError] = useState<WorkspaceErrorKey | null>(null);
  const [pageError, setPageError] = useState<WorkspaceErrorKey | null>(null);
  const [pendingRead, setPendingRead] = useState(false);
  // Read inside async callbacks so a late response cannot resurrect a previous scope's rows.
  const generation = useRef(0);
  const filterKey = JSON.stringify(filters);
  const scopeKey = JSON.stringify(scope);

  const read = useCallback(
    async (from: EvidenceCursor | null, replace: boolean) => {
      if (scope === null) return;
      const attempt = generation.current;
      setLoading(true);
      setPageError(null);
      try {
        const page = await service.listExecutionRecords({
          scope,
          filters,
          limit: EVIDENCE_PAGE_LIMITS.default,
          ...(from === null ? {} : { cursor: from }),
        });
        if (attempt !== generation.current) return;
        setRecords((current) => (replace ? page.items : mergeRecordPage(current, page.items)));
        setCoverage(page.coverage);
        setCursor(page.nextCursor ?? null);
        setInitialError(null);
      } catch (reason: unknown) {
        if (attempt !== generation.current) return;
        // Rows already on screen stay: a failed continuation says nothing about the page the
        // reader is looking at, and blanking it would lose work they were in the middle of.
        if (replace && records.length === 0) setInitialError(workspaceErrorKey(reason));
        else setPageError(workspaceErrorKey(reason));
      } finally {
        if (attempt === generation.current) setLoading(false);
      }
    },
    // `records.length` is read only to decide which error slot a failure belongs in, and adding it
    // here would re-create the callback on every append.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [filterKey, scopeKey, service],
  );

  useEffect(() => {
    generation.current += 1;
    setRecords([]);
    setCoverage(null);
    setCursor(null);
    setInitialError(null);
    setPageError(null);
    setPendingRead(true);
  }, [filterKey, scopeKey]);

  // A live notice re-reads the newest page without clearing what is loaded: the merge replaces the
  // rows that moved and leaves the rest, so a running record becoming terminal does not cost the
  // reader their scroll position.
  useEffect(() => {
    if (refreshToken === 0) return;
    setPendingRead(true);
  }, [refreshToken]);

  useEffect(() => {
    if (!pendingRead || !isVisible || scope === null) return;
    setPendingRead(false);
    void read(null, records.length === 0);
    // `records.length` decides replace-versus-merge for this one read and must not re-trigger it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [isVisible, pendingRead, read, scope]);

  const loadMore = useCallback(async () => {
    if (cursor === null || loading) return;
    await read(cursor, false);
  }, [cursor, loading, read]);

  const retry = useCallback(async () => {
    // The same cursor the failed attempt used. A retry that moved the boundary would skip rows
    // between the two, and nothing downstream could tell that it had.
    await read(cursor, records.length === 0);
  }, [cursor, read, records.length]);

  return {
    coverage,
    hasMore: cursor !== null,
    initialError,
    loadMore,
    loading,
    pageError,
    records,
    retry,
  };
}
