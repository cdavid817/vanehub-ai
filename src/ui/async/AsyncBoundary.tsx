import type { ReactNode } from "react";
import { AlertTriangle, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { useReducedMotion } from "../../hooks/use-reduced-motion";
import { cn } from "../../lib/utils";
import { EmptyState, type EmptyStateProps } from "../empty-state/EmptyState";
import { isAsyncViewLoading, type AsyncViewState } from "./async-view-state";
import { RefreshIndicator } from "./RefreshIndicator";

type EmptyStateSlot = Pick<EmptyStateProps, "title" | "description" | "action">;

export interface AsyncBoundaryProps<T> {
  state: AsyncViewState<T>;
  children: (data: T) => ReactNode;
  onRetry?: () => void;
  isEmpty?: (data: T) => boolean;
  filtered?: boolean;
  emptyState?: EmptyStateSlot;
  filteredEmptyState?: EmptyStateSlot;
  unavailableState?: EmptyStateSlot;
  restrictedState?: EmptyStateSlot;
  loadingFallback?: ReactNode;
  className?: string;
}

/**
 * Renders the query lifecycle described by `AsyncViewState<T>` (design.md Decision 11):
 * initial loading, error/unavailable/restricted, empty/filtered-empty, and content-with-refresh.
 * `emptyState`/`filteredEmptyState` are optional — without them, an empty result just renders
 * `children(data)` rather than guessing untranslated placeholder copy.
 */
export function AsyncBoundary<T>({
  state,
  children,
  onRetry,
  isEmpty,
  filtered = false,
  emptyState,
  filteredEmptyState,
  unavailableState,
  restrictedState,
  loadingFallback,
  className,
}: AsyncBoundaryProps<T>) {
  const { t } = useTranslation();
  const reducedMotion = useReducedMotion();

  if (isAsyncViewLoading(state)) {
    return loadingFallback !== undefined ? (
      <>{loadingFallback}</>
    ) : (
      <div className={cn("flex min-h-40 items-center justify-center gap-2 p-6 text-sm text-muted-foreground", className)} role="status">
        <LoaderCircle aria-hidden="true" className={cn("h-5 w-5", !reducedMotion && "animate-spin")} />
        {t("workbenchUi.async.loading")}
      </div>
    );
  }

  if (state.error) {
    if (state.error.kind === "unavailable") {
      return <EmptyState className={className} title={t("workbenchUi.evidence.unavailable")} variant="unavailable" {...unavailableState} />;
    }
    if (state.error.kind === "restricted") {
      return <EmptyState className={className} title={t("workbenchUi.evidence.restricted")} variant="restricted" {...restrictedState} />;
    }
    const canRetry = state.error.retryable && Boolean(onRetry);
    return (
      <div className={cn("flex min-h-40 flex-col items-center justify-center gap-3 p-6 text-center", className)} role="alert">
        <AlertTriangle aria-hidden="true" className="h-6 w-6 text-destructive" />
        <p className="text-sm text-muted-foreground">{state.error.message}</p>
        {canRetry ? (
          <button className="ucd-focus-ring rounded-sm text-sm font-medium text-primary underline-offset-4 hover:underline" onClick={onRetry} type="button">
            {t("featureLoad.retry")}
          </button>
        ) : null}
      </div>
    );
  }

  if (state.data === undefined) return null;

  if (isEmpty?.(state.data)) {
    const slot = filtered ? filteredEmptyState : emptyState;
    if (slot) return <EmptyState className={className} variant={filtered ? "no-filter-match" : "no-data"} {...slot} />;
  }

  return (
    <div className={className}>
      <RefreshIndicator refreshing={state.refreshing} stale={state.stale} />
      {children(state.data)}
    </div>
  );
}
