import { useEffect, useRef, useState } from "react";
import { useDebouncedValue } from "../hooks/use-debounced-value";
import { executionTargetSearchProviders, type ExecutionTargetKind, type ExecutionTargetOption, type ExecutionTargetProviders } from "./execution-target-providers";

const DEBOUNCE_MS = 250;

export interface ExecutionTargetSearchState {
  options: ExecutionTargetOption[];
  loading: boolean;
  error: string | null;
}

/**
 * One kind at a time -- mirrors use-command-center-search.ts's own debounce and stale-result
 * discard shape (design.md Decision 4's "旧查询不得覆盖新结果"), scoped down from a merged
 * multi-provider search to a single provider: a target picker searches whichever one kind its own
 * `<select>` currently has chosen, never a cross-kind merge.
 *
 * `providers` defaults to the real registry but is an explicit parameter, same reason as that same
 * hook's own doc comment: tests can inject small, fully-controllable fakes. Read through a ref and
 * excluded from the effect's own dependency array for the same reason too -- a caller passing an
 * inline object literal would otherwise get a fresh identity every render and retrigger the effect
 * regardless of whether `kind`/`query` changed.
 */
export function useExecutionTargetSearch(
  kind: ExecutionTargetKind,
  query: string,
  providers: ExecutionTargetProviders = executionTargetSearchProviders,
): ExecutionTargetSearchState {
  const debouncedQuery = useDebouncedValue(query, DEBOUNCE_MS);
  const [state, setState] = useState<ExecutionTargetSearchState>({ options: [], loading: false, error: null });
  const providersRef = useRef(providers);
  providersRef.current = providers;

  useEffect(() => {
    const controller = new AbortController();
    setState((current) => ({ ...current, loading: true, error: null }));
    providersRef.current[kind](debouncedQuery)
      .then((options) => {
        if (controller.signal.aborted) return;
        setState({ options, loading: false, error: null });
      })
      .catch((reason: unknown) => {
        if (controller.signal.aborted) return;
        setState({ options: [], loading: false, error: reason instanceof Error ? reason.message : String(reason) });
      });
    return () => controller.abort();
    // `providers` deliberately excluded -- see the doc comment above.
  }, [kind, debouncedQuery]);

  return state;
}
