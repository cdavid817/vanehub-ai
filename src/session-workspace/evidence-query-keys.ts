import type {
  EvidenceCursor,
  EvidenceRecordId,
  EvidenceSessionId,
  ExecutionRecordFilters,
  WorkspaceEvidenceScope,
} from "../types/session-workspace-evidence";

/**
 * Query keys for every evidence read, built in one place.
 *
 * The failure this prevents is silent: a panel that forgets to put the selected seat, run, or
 * cursor in its key gets a cache hit from a different scope and renders another seat's rows
 * without erroring. Building keys by hand at each call site makes that omission invisible, so the
 * scope is destructured exhaustively here — adding a field to `WorkspaceEvidenceScope` breaks this
 * file until the field is placed deliberately.
 */
function scopeKey(scope: WorkspaceEvidenceScope): readonly unknown[] {
  const {
    sessionId,
    seatId,
    runId,
    traceId,
    spanId,
    operationId,
    commandId,
    relativePath,
    hunkFingerprint,
    occurredAt,
  } = scope;
  return [
    sessionId,
    seatId ?? null,
    runId ?? null,
    traceId ?? null,
    spanId ?? null,
    operationId ?? null,
    commandId ?? null,
    relativePath ?? null,
    hunkFingerprint ?? null,
    occurredAt ?? null,
  ];
}

/**
 * Filters are normalized before they enter a key so that two equivalent filter sets cannot produce
 * two cache entries. Arrays are sorted and empty values collapse to null.
 */
function filtersKey(filters: ExecutionRecordFilters | undefined): readonly unknown[] {
  if (!filters) return [null, null, null, null];
  const { kinds, statuses, fidelities, search } = filters;
  return [
    kinds && kinds.length > 0 ? [...kinds].sort() : null,
    statuses && statuses.length > 0 ? [...statuses].sort() : null,
    fidelities && fidelities.length > 0 ? [...fidelities].sort() : null,
    search && search.trim().length > 0 ? search.trim() : null,
  ];
}

const root = "session-workspace-evidence" as const;

export const evidenceQueryKeys = {
  all: () => [root] as const,

  /**
   * Null is a key, not an error. The workspace mounts its summary query before a session is
   * selected and leaves it disabled; giving that state its own key keeps the "no session" entry
   * from colliding with a real one.
   */
  summary: (sessionId: EvidenceSessionId | null, seatId?: string) =>
    [root, "summary", sessionId, seatId ?? null] as const,

  /**
   * The cursor is part of the key. A page fetched with one continuation token is not the same
   * result as a page fetched with another, and sharing an entry between them is how a keyset list
   * ends up showing a duplicated or skipped row.
   */
  records: (
    scope: WorkspaceEvidenceScope,
    filters?: ExecutionRecordFilters,
    cursor?: EvidenceCursor,
  ) => [root, "records", scopeKey(scope), filtersKey(filters), cursor ?? null] as const,

  recordDetail: (sessionId: EvidenceSessionId, recordId: EvidenceRecordId) =>
    [root, "record-detail", sessionId, recordId] as const,

  report: (
    sessionId: EvidenceSessionId,
    runIds: readonly string[],
    seatIds: readonly string[],
    from: string | undefined,
    to: string | undefined,
    groupBy: string | undefined,
  ) => [
    root,
    "report",
    sessionId,
    [...runIds].sort(),
    [...seatIds].sort(),
    from ?? null,
    to ?? null,
    groupBy ?? null,
  ] as const,
} as const;
