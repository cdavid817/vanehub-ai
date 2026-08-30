export type DisplayableErrorKind = "error" | "unavailable" | "restricted";

/**
 * A presentation-ready projection of a service/query failure. `src/ui/` primitives cannot
 * import the service-layer `ServiceError` (see ARCH-FE-005), so callers map their own error
 * shape into this one — `message` is already localized by the caller, not by the primitive.
 */
export interface DisplayableError {
  kind: DisplayableErrorKind;
  message: string;
  retryable: boolean;
}

export interface AsyncViewState<T> {
  data?: T;
  initialLoading: boolean;
  refreshing: boolean;
  error?: DisplayableError;
  stale: boolean;
}

export function isAsyncViewLoading<T>(state: AsyncViewState<T>): boolean {
  return state.initialLoading && state.data === undefined;
}
