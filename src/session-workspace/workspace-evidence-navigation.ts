import type {
  WorkspaceEvidenceScope,
  WorkspaceEvidenceTabId,
} from "../types/session-workspace-evidence";

/**
 * A correlation field: every scope field except the one the scope belongs to.
 *
 * `sessionId` is deliberately not a filter. It says whose evidence this is, and a control that
 * offered to clear it would be offering to point one session's panels at another session's rows.
 */
export type EvidenceScopeField = Exclude<keyof WorkspaceEvidenceScope, "sessionId">;

/** The correlation half of a scope, separated so it can never claim a session of its own. */
export type WorkspaceEvidenceCorrelation = Omit<WorkspaceEvidenceScope, "sessionId">;

/** Declaration order is chip order, so two panels cannot disagree about how a scope reads. */
export const EVIDENCE_SCOPE_FIELDS: readonly EvidenceScopeField[] = [
  "seatId",
  "runId",
  "traceId",
  "spanId",
  "operationId",
  "commandId",
  "relativePath",
  "hunkFingerprint",
  "occurredAt",
];

/**
 * What each destination actually reads.
 *
 * A panel that receives a field it does not filter by must say so rather than look filtered. The
 * Files tree handed a `commandId` renders the whole tree; if the chips still claimed one command,
 * nothing on screen would tell the reader which of the two statements is true.
 *
 * `satisfies` rather than a type annotation so a new tab id is a compile error here — a tab left
 * out would silently consume nothing and report every field as unsupported.
 */
export const TAB_SCOPE_FIELDS = {
  "terminal-history": ["seatId", "runId", "operationId", "commandId", "occurredAt"],
  shell: ["seatId"],
  logs: ["seatId", "runId", "traceId", "spanId", "operationId", "occurredAt"],
  traces: ["runId", "traceId", "spanId", "occurredAt"],
  changes: ["runId", "relativePath", "hunkFingerprint"],
  // Files' merged Documents view reads the same path filter the Explorer view always did.
  files: ["relativePath"],
  report: ["seatId", "runId"],
} satisfies Record<WorkspaceEvidenceTabId, readonly EvidenceScopeField[]>;

/**
 * Fields whose meaning is owned by another field.
 *
 * Clearing the owner clears what it owns: a span id whose trace is gone, or a hunk whose file is
 * gone, is a filter no query can resolve — it would either return nothing or, worse, match the
 * same identifier under a different parent.
 */
export const DEPENDENT_SCOPE_FIELDS = {
  runId: ["traceId", "spanId", "operationId", "commandId"],
  traceId: ["spanId"],
  relativePath: ["hunkFingerprint"],
} satisfies Partial<Record<EvidenceScopeField, readonly EvidenceScopeField[]>>;

function dependentsOf(field: EvidenceScopeField): readonly EvidenceScopeField[] {
  const table: Partial<Record<EvidenceScopeField, readonly EvidenceScopeField[]>> =
    DEPENDENT_SCOPE_FIELDS;
  return table[field] ?? [];
}

/** The transitive closure of `fields` under ownership, so clearing a run also drops its spans. */
export function withDependentScopeFields(
  fields: readonly EvidenceScopeField[],
): Set<EvidenceScopeField> {
  const closed = new Set<EvidenceScopeField>();
  const pending = [...fields];
  while (pending.length > 0) {
    const field = pending.pop();
    if (field === undefined || closed.has(field)) continue;
    closed.add(field);
    pending.push(...dependentsOf(field));
  }
  return closed;
}

function presentFields(correlation: WorkspaceEvidenceCorrelation): EvidenceScopeField[] {
  return EVIDENCE_SCOPE_FIELDS.filter((field) => correlation[field] !== undefined);
}

/** The fields the destination will actually apply, in chip order. */
export function consumedScopeFields(
  tab: WorkspaceEvidenceTabId,
  correlation: WorkspaceEvidenceCorrelation,
): EvidenceScopeField[] {
  const supported = new Set<EvidenceScopeField>(TAB_SCOPE_FIELDS[tab]);
  return presentFields(correlation).filter((field) => supported.has(field));
}

/** The fields the destination was handed and will ignore. Rendered as an explicit notice. */
export function unsupportedScopeFields(
  tab: WorkspaceEvidenceTabId,
  correlation: WorkspaceEvidenceCorrelation,
): EvidenceScopeField[] {
  const supported = new Set<EvidenceScopeField>(TAB_SCOPE_FIELDS[tab]);
  return presentFields(correlation).filter((field) => !supported.has(field));
}

/** Whether a tab reads any cross-panel correlation at all. Chips render only where it does. */
export function tabConsumesScope(tab: WorkspaceEvidenceTabId): boolean {
  return TAB_SCOPE_FIELDS[tab].length > 0;
}
