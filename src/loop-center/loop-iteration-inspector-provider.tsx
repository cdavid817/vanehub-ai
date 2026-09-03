import { useTranslation } from "react-i18next";
import { useLoopRunQuery } from "../hooks/use-loop-queries";
import type { LoopRun } from "../types/loop";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { InspectorProviderProps } from "../ui/inspector/inspector-provider-registry";
import { LoopInspectorBody } from "./loop-inspector";

/**
 * The `loop-iteration` Inspector provider (17.3): re-homes `LoopInspector`'s own run/limits/
 * workspace content behind the shared Inspector mechanism instead of Loop Center's former bespoke
 * column-3 render, reusing `LoopInspectorBody` (extracted from `LoopInspector` unchanged) rather
 * than rewriting its data-derivation or markup.
 *
 * `selection.iterationId` is intentionally never read here: `LoopInspectorBody` already defaults
 * to the run's own latest iteration internally (`run.iterations.at(-1)`), exactly like
 * `LoopInspector` always has. `WorkbenchSelection`'s loop-shaped variant only models the run as a
 * whole plus one iteration id today, not a reader-chosen iteration among several -- letting a
 * reader pick a *specific* one is task 17.11's own still-open design question, out of scope here.
 */
export function LoopIterationInspectorProvider({ context, selection }: InspectorProviderProps<"loop-iteration">) {
  const { t } = useTranslation();
  const runQuery = useLoopRunQuery(selection.loopRunId);

  // Unlike SessionOverview's list+find (where "not present in the loaded list" is a real,
  // reachable outcome), `getLoopRun` fetches this one run directly by id -- a run that no longer
  // exists surfaces as a rejected query (the `error` branch below), not a resolved-but-empty one,
  // so there is no separate "unavailable" case to model here.
  const state: AsyncViewState<LoopRun> = {
    data: runQuery.data,
    initialLoading: runQuery.isLoading,
    refreshing: runQuery.isFetching && !runQuery.isLoading,
    stale: false,
    error: runQuery.isError ? { kind: "error", message: t("loops.states.error"), retryable: true } : undefined,
  };

  return (
    <AsyncBoundary onRetry={() => runQuery.refetch()} state={state}>
      {(resolvedRun) => <LoopInspectorBody onInspect={context.onInspectLoop} run={resolvedRun} />}
    </AsyncBoundary>
  );
}
