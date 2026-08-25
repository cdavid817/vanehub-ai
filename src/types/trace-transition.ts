/**
 * The live trace-transition contract.
 *
 * Identifiers and a status, never a span. A view that received the span itself would hold a second
 * shape for something it can already fetch, and the two disagree the moment anything writes to it
 * again — so a notice says *what* changed and the view refetches the timeline it already knows how
 * to ask for.
 */

export type TraceTransitionKind =
  | "run-started"
  | "run-finished"
  | "span-started"
  | "span-finished";

export type TraceTransitionStatus =
  | "accepted"
  | "running"
  | "succeeded"
  | "failed"
  | "cancelled"
  | "incomplete";

export interface TraceTransitionNotice {
  kind: TraceTransitionKind;
  runId: string;
  traceId: string;
  /** The span that changed, for a span transition. Absent for a run transition. */
  spanId?: string;
  status: TraceTransitionStatus;
  /** When the transition happened, for a finish. Absent for a start, which happens now. */
  occurredAt?: string;
  /**
   * Whether this changes the run list rather than one open timeline.
   *
   * Decided natively so two views cannot answer it differently, and so a busy run's spans cannot
   * be made to re-read the run list once per span.
   */
  affectsRunList: boolean;
}

export type TraceTransitionUnsubscribe = () => void;

export interface TraceTransitionStream {
  subscribe: (
    listener: (notice: TraceTransitionNotice) => void,
  ) => TraceTransitionUnsubscribe;
}
