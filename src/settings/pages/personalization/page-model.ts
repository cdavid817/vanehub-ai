import type {
  PersonalizationHealth,
  PersonalizationHealthState,
  PersonalizationPolicyRef,
} from "../../../types/personalization";
import type { MemoryQuery } from "../../../types/personalization-memory";
import type { InstructionDraftMap } from "./instruction-drafts";

// The page's own state, kept apart from the server's.
//
// Six things that change for six different reasons: what the queries are doing, which scope the
// user is looking at, what they have typed, what is in flight, what the store refused, and whether
// the store is safe to write to at all. Collapsing any pair means one of them resets the other --
// a refetch clearing a conflict, a scope change cancelling a save.

/** What a query is doing, projected from the cache rather than copied out of it. */
export interface QuerySlice {
  status: "loading" | "ready" | "error";
  error: string | null;
}

export interface PersonalizationQueryState {
  policies: QuerySlice;
  memories: QuerySlice;
  candidates: QuerySlice;
  health: QuerySlice;
}

export interface MaintenanceState {
  health: PersonalizationHealthState;
  memoryAvailable: boolean;
  pendingCandidates: number;
  /** A reconciliation the user started, which is not something a query result can report. */
  reconciling: boolean;
}

export interface PersonalizationPageState {
  query: PersonalizationQueryState;
  scope: PersonalizationPolicyRef;
  drafts: InstructionDraftMap;
  memoryQuery: MemoryQuery;
  maintenance: MaintenanceState;
}

const LOADING: QuerySlice = { status: "loading", error: null };

export const initialPersonalizationPageState: PersonalizationPageState = {
  query: { policies: LOADING, memories: LOADING, candidates: LOADING, health: LOADING },
  scope: { scopeKind: "global" },
  drafts: {},
  memoryQuery: {},
  maintenance: {
    // Not `ready` until something says so: assuming health would let the page offer writes during
    // a migration, which is the one state the store must not take them in.
    health: "not_started",
    memoryAvailable: false,
    pendingCandidates: 0,
    reconciling: false,
  },
};

/**
 * Projects one query's cache entry into the slice the page renders.
 *
 * Deliberately a projection and not a copy: react-query already owns whether data is stale, in
 * flight, or failed, and a second copy of that would need its own invalidation to stay true.
 */
export function describeQuery(input: { isPending: boolean; error: unknown }): QuerySlice {
  if (input.error) {
    return { status: "error", error: input.error instanceof Error ? input.error.message : String(input.error) };
  }
  return input.isPending ? LOADING : { status: "ready", error: null };
}

export function selectScope(
  state: PersonalizationPageState,
  scope: PersonalizationPolicyRef,
): PersonalizationPageState {
  // Drafts are untouched: they are keyed by scope, so switching away and back returns the user to
  // what they were typing rather than to what the store last said.
  return { ...state, scope };
}

/**
 * A filter change starts the result set over.
 *
 * A cursor names a position in one ordering of one filtered set; carrying it into a different set
 * would resume from a row that is no longer in it, which reads as a page of missing results.
 */
export function setMemoryFilter(
  state: PersonalizationPageState,
  patch: Omit<MemoryQuery, "cursor">,
): PersonalizationPageState {
  return { ...state, memoryQuery: { ...state.memoryQuery, ...patch, cursor: undefined } };
}

export function advanceMemoryCursor(
  state: PersonalizationPageState,
  cursor: string | null,
): PersonalizationPageState {
  return { ...state, memoryQuery: { ...state.memoryQuery, cursor: cursor ?? undefined } };
}

export function applyHealth(
  state: PersonalizationPageState,
  health: PersonalizationHealth,
): PersonalizationPageState {
  return {
    ...state,
    maintenance: {
      ...state.maintenance,
      health: health.state,
      memoryAvailable: health.memoryAvailable,
      pendingCandidates: health.pendingCandidates,
    },
  };
}

export function setReconciling(
  state: PersonalizationPageState,
  reconciling: boolean,
): PersonalizationPageState {
  return { ...state, maintenance: { ...state.maintenance, reconciling } };
}

/**
 * Whether the store will accept a policy or memory write right now.
 *
 * `busy` and the two rebuild states are transient rather than broken, but a write during any of
 * them races the process that is rewriting the same rows.
 */
export function acceptsWrites(maintenance: MaintenanceState): boolean {
  return maintenance.health === "ready" && !maintenance.reconciling;
}

export function withQuery(
  state: PersonalizationPageState,
  patch: Partial<PersonalizationQueryState>,
): PersonalizationPageState {
  return { ...state, query: { ...state.query, ...patch } };
}
