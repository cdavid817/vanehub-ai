import { useEffect, useRef, useState } from "react";
import { useDebouncedValue } from "../hooks/use-debounced-value";
import { SEARCH_PROVIDERS } from "./command-center-registry";
import { rankSearchResults } from "./rank-search-results";
import type { WorkbenchSearchProvider, WorkbenchSearchResult, WorkbenchSearchScope } from "./command-center-types";

const ALL_SCOPES: WorkbenchSearchScope[] = ["session", "project", "run", "goal", "work-item", "evaluation"];
/** Matches the session sidebar's own search debounce (`use-main-layout-model.ts`) for consistency. */
const DEBOUNCE_MS = 250;
const RESULT_LIMIT = 8;

export interface CommandCenterSearchState {
  results: WorkbenchSearchResult[];
  loading: boolean;
  /** 6.13's "failed provider" state: ids of providers whose `search()` rejected this round — the
   *  other providers' results still show, per `Promise.allSettled` below, not a blanket failure. */
  failedProviderIds: string[];
}

/**
 * 6.10. `AbortController` is created fresh per debounced query; its `signal.aborted` check after
 * `Promise.allSettled` resolves is what "旧查询不得覆盖新结果" (design.md Decision 4) actually means
 * in practice — none of the registry's three underlying service calls are truly abortable
 * (confirmed while building each), so the guarantee is "a stale response is discarded on arrival,"
 * not "a stale request is torn down mid-flight." Both satisfy 6.10's own wording ("ignore stale
 * results deterministically"), which only requires the former.
 *
 * `providers` defaults to the real registry but is an explicit parameter so tests can inject a
 * small set of fully-controllable fakes — cleaner than mocking `command-center-registry.ts`'s
 * module exports, and it keeps this hook's own tests about its orchestration (debounce, cancel,
 * merge, rank), not re-proving what each real provider already proves about itself.
 *
 * Read through a ref, not a `useEffect` dependency: a caller passing an inline array literal
 * (`useCommandCenterSearch(query, [a, b])`) — exactly what an early version of this hook's own
 * tests did — creates a new array identity every render. Depending on it directly reruns the
 * effect every render regardless of whether the query changed, and since the effect itself calls
 * `setState`, that is an infinite render loop, not just wasted work. The default value is a stable
 * module-level `const`, so real callers omitting the argument were never at risk — this hardens the
 * one path a caller passing its own array could still hit.
 */
export function useCommandCenterSearch(query: string, providers: WorkbenchSearchProvider[] = SEARCH_PROVIDERS): CommandCenterSearchState {
  const debouncedQuery = useDebouncedValue(query, DEBOUNCE_MS);
  const [state, setState] = useState<CommandCenterSearchState>({ results: [], loading: false, failedProviderIds: [] });
  const providersRef = useRef(providers);
  providersRef.current = providers;

  useEffect(() => {
    if (!debouncedQuery.trim()) {
      setState({ results: [], loading: false, failedProviderIds: [] });
      return;
    }
    const controller = new AbortController();
    setState((current) => ({ ...current, loading: true }));

    const activeProviders = providersRef.current.filter((provider) => ALL_SCOPES.some((scope) => provider.supports(scope)));
    void Promise.allSettled(
      activeProviders.map((provider) => provider.search({
        query: debouncedQuery, scopes: ALL_SCOPES, limit: RESULT_LIMIT, signal: controller.signal,
      })),
    ).then((outcomes) => {
      if (controller.signal.aborted) return;
      const merged: WorkbenchSearchResult[] = [];
      const failedProviderIds: string[] = [];
      outcomes.forEach((outcome, index) => {
        if (outcome.status === "fulfilled") merged.push(...outcome.value.items);
        else failedProviderIds.push(activeProviders[index].id);
      });
      setState({ results: rankSearchResults(merged, debouncedQuery), loading: false, failedProviderIds });
    });

    return () => controller.abort();
    // `providers` deliberately excluded: see the doc comment above this hook.
  }, [debouncedQuery]);

  return state;
}
