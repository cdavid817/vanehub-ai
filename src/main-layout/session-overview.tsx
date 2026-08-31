import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState } from "../ui/async/async-view-state";
import type { InspectorProviderProps } from "../ui/inspector/inspector-provider-registry";
import { SessionOverviewSections } from "./session-overview-sections";

/**
 * The `session`-kind Inspector provider (design.md Decision 8: "Session Overview 使用
 * Section/Accordion"). Registered against `INSPECTOR_PROVIDERS.session` separately from this file
 * (the coordinator's own, later task) — this component's exported props therefore match
 * `InspectorProviderProps<"session">` exactly, so registering it needs no changes here.
 */
export function SessionOverview({ selection, context }: InspectorProviderProps<"session">) {
  const { t } = useTranslation();
  // Same query key ["sessions"] the rest of the app already reads sessions through
  // (session-sidebar.tsx and friends), so this shares React Query's cache instead of issuing a
  // second, redundant list request for a session the app has almost certainly already loaded.
  const sessionsQuery = useQuery({ queryKey: ["sessions"], queryFn: () => agentService.listSessions() });
  // Known, accepted limitation: this only finds sessions `listSessions()` returns. An
  // archived-only session resolves to the same `unavailable` state below as a genuinely deleted
  // one — correct, honest behavior for this pass ("nothing to show"), not a bug to fix here.
  const session = sessionsQuery.data?.find((candidate) => candidate.id === selection.sessionId) ?? null;

  const state: AsyncViewState<Session> = {
    data: session ?? undefined,
    initialLoading: sessionsQuery.isLoading,
    refreshing: sessionsQuery.isFetching && !sessionsQuery.isLoading,
    stale: false,
    error: sessionsQuery.isError
      ? { kind: "error", message: t("sessionOverview.loadError"), retryable: true }
      : !sessionsQuery.isLoading && !session
        ? { kind: "unavailable", message: t("workbenchUi.evidence.unavailable"), retryable: false }
        : undefined,
  };

  return (
    <AsyncBoundary onRetry={() => sessionsQuery.refetch()} state={state}>
      {(resolvedSession) => <SessionOverviewSections context={context} session={resolvedSession} />}
    </AsyncBoundary>
  );
}
