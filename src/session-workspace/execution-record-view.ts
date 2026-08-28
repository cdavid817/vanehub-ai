import type {
  EvidenceFidelity,
  EvidenceStatus,
  ExecutionRecordFilters,
  ExecutionRecordKind,
} from "../types/session-workspace-evidence";

/**
 * What the reader is looking at.
 *
 * Legacy is a view rather than a kind filter, because it is not the same corpus: native records
 * come from the journal and legacy activity is projected from loaded messages. Offering "Legacy"
 * alongside "Commands" in one filter would imply they can be combined, and a combined list would
 * mix observed work with an assistant's account of it.
 */
export type ExecutionRecordView = "all" | "commands" | "tools" | "delegations" | "verification" | "legacy";

export const EXECUTION_RECORD_VIEWS: readonly ExecutionRecordView[] = [
  "all",
  "commands",
  "tools",
  "delegations",
  "verification",
  "legacy",
];

/**
 * The kinds a native view asks the query for. Null is the legacy view, which asks the query for
 * nothing at all.
 */
const VIEW_KINDS = {
  all: ["command", "tool", "delegation", "verification"],
  commands: ["command"],
  tools: ["tool"],
  delegations: ["delegation"],
  verification: ["verification"],
  legacy: null,
} satisfies Record<ExecutionRecordView, readonly ExecutionRecordKind[] | null>;

export function isLegacyView(view: ExecutionRecordView): boolean {
  return VIEW_KINDS[view] === null;
}

/** Filters the reader can set independently of the view. */
export interface ExecutionRecordFilterState {
  statuses: readonly EvidenceStatus[];
  fidelities: readonly EvidenceFidelity[];
  search: string;
}

export const EMPTY_FILTERS: ExecutionRecordFilterState = {
  statuses: [],
  fidelities: [],
  search: "",
};

/**
 * The query filters a view and a filter state produce together.
 *
 * The view owns `kinds` outright rather than merging with a kind the reader might also have
 * chosen: a Commands view showing tools, or a Commands view showing nothing because the two
 * disagreed, are both states the reader has no way to explain from what is on screen. One owner
 * per field is what makes that unrepresentable.
 */
export function queryFilters(
  view: ExecutionRecordView,
  filters: ExecutionRecordFilterState,
): ExecutionRecordFilters {
  const kinds = VIEW_KINDS[view];
  const search = filters.search.trim();
  return {
    kinds: kinds === null ? [] : [...kinds],
    statuses: [...filters.statuses],
    fidelities: [...filters.fidelities],
    ...(search.length > 0 ? { search } : {}),
  };
}

/** Whether anything is narrowing the list, which is what an empty result has to explain. */
export function hasActiveFilters(filters: ExecutionRecordFilterState): boolean {
  return (
    filters.statuses.length > 0 ||
    filters.fidelities.length > 0 ||
    filters.search.trim().length > 0
  );
}

export const SELECTABLE_STATUSES: readonly EvidenceStatus[] = [
  "running",
  "succeeded",
  "failed",
  "cancelled",
  "incomplete",
  "queued",
];

export const SELECTABLE_FIDELITIES: readonly EvidenceFidelity[] = [
  "native",
  "proxied",
  "inferred",
  "opaque",
];

/** Toggles one value in a selection, keeping the declared order so the query key is stable. */
export function toggleSelection<T>(current: readonly T[], value: T, order: readonly T[]): T[] {
  const next = current.includes(value)
    ? current.filter((entry) => entry !== value)
    : [...current, value];
  return order.filter((entry) => next.includes(entry));
}
