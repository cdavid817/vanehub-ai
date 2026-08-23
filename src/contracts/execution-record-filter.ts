import type {
  ExecutionRecord,
  ExecutionRecordQuery,
} from "../types/session-workspace-evidence";

/**
 * The fields a bounded record search reads.
 *
 * Only what the producer already redacted and the projection already holds. Searching a raw
 * payload would put producer content back into a query path the whole design exists to keep it out
 * of, and searching an identifier would make a search box a way to probe for ids.
 */
function searchableText(record: ExecutionRecord): string {
  switch (record.kind) {
    case "command":
      return record.redactedDisplay ?? "";
    case "tool":
      return record.toolName;
    case "verification":
      return record.verificationName;
    case "legacy":
      return record.label;
    case "delegation":
      return "";
  }
}

/**
 * Whether one record belongs in the answer to one query.
 *
 * Stated here rather than inside an adapter because both runtimes have to agree on it: the desktop
 * expresses the same rule in SQL, and a rule implemented once per adapter diverges the first time
 * a filter is added to only one of them — which is exactly how the Web adapter came to accept a
 * search term and then ignore it, returning every row as though the term had matched them all.
 */
export function matchesExecutionRecordQuery(
  record: ExecutionRecord,
  query: ExecutionRecordQuery,
): boolean {
  const { filters, scope } = query;
  if (record.sessionId !== scope.sessionId) return false;
  // An absent correlation is not a match for a concrete filter value. Attributing an uncorrelated
  // record to the current selection is the behaviour the seat work removed elsewhere.
  if (scope.seatId !== undefined && record.seatId !== scope.seatId) return false;
  if (scope.runId !== undefined && record.runId !== scope.runId) return false;
  if (scope.traceId !== undefined && record.traceId !== scope.traceId) return false;
  if (scope.spanId !== undefined && record.spanId !== scope.spanId) return false;
  if (!filters) return true;
  if (filters.kinds?.length && !filters.kinds.includes(record.kind)) return false;
  if (filters.statuses?.length && !filters.statuses.includes(record.status)) return false;
  if (filters.fidelities?.length && !filters.fidelities.includes(record.fidelity)) return false;

  const search = filters.search?.trim().toLowerCase() ?? "";
  // A blank term is no filter at all, not a filter that matches nothing.
  if (search.length === 0) return true;
  return searchableText(record).toLowerCase().includes(search);
}
