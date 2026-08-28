import { detectRuntimeKind } from "./runtime-adapter";
import type {
  TraceTransitionNotice,
  TraceTransitionStream,
  TraceTransitionUnsubscribe,
} from "../types/trace-transition";

/**
 * The native event channel. Has to match `TRACE_TRANSITION_EVENT` in the Rust publisher verbatim;
 * a mismatch produces a subscription that never fires and never errors, which is the one failure
 * a live view cannot detect from the inside.
 */
export const TRACE_TRANSITION_CHANNEL = "execution-trace:transition";

const KINDS = new Set([
  "run-started",
  "run-finished",
  "span-started",
  "span-finished",
]);

/**
 * Validates a notice off the event channel.
 *
 * Strict about the discriminant and the run id, lenient about the rest. A payload whose kind is
 * unrecognised would be routed by a `switch` that has no branch for it; a missing `occurredAt` is
 * the ordinary case of a start, which has no time to report other than now.
 */
export function parseTraceTransition(payload: unknown): TraceTransitionNotice | null {
  if (typeof payload !== "object" || payload === null) return null;
  const raw = payload as Record<string, unknown>;
  if (typeof raw.kind !== "string" || !KINDS.has(raw.kind)) return null;
  if (typeof raw.runId !== "string" || raw.runId.length === 0) return null;
  if (typeof raw.status !== "string") return null;
  return {
    kind: raw.kind as TraceTransitionNotice["kind"],
    runId: raw.runId,
    traceId: typeof raw.traceId === "string" ? raw.traceId : "",
    spanId: typeof raw.spanId === "string" ? raw.spanId : undefined,
    status: raw.status as TraceTransitionNotice["status"],
    occurredAt: typeof raw.occurredAt === "string" ? raw.occurredAt : undefined,
    // Absent means false rather than "assume it does". Assuming would make every span transition
    // re-read the run list, which is the exact cost this flag exists to avoid.
    affectsRunList: raw.affectsRunList === true,
  };
}

/**
 * The live trace stream for whichever runtime is hosting the UI.
 *
 * The browser build has no native transitions to deliver, and says so by never calling the
 * listener rather than by inventing fixture activity: a mock that emitted transitions on a timer
 * would make the coalescing look like it worked without anything having transitioned.
 */
export function createTraceTransitionStream(): TraceTransitionStream {
  if (detectRuntimeKind() !== "tauri") {
    return { subscribe: () => () => {} };
  }
  return {
    subscribe(listener) {
      let released = false;
      let stop: TraceTransitionUnsubscribe | null = null;
      void import("@tauri-apps/api/event").then(async ({ listen }) => {
        const unlisten = await listen<unknown>(TRACE_TRANSITION_CHANNEL, (event) => {
          const parsed = parseTraceTransition(event.payload);
          // A malformed frame is dropped rather than thrown: an event handler has no caller to
          // reject to, and one bad frame must not tear down a live subscription.
          if (parsed) listener(parsed);
        });
        if (released) unlisten();
        else stop = () => unlisten();
      });
      return () => {
        released = true;
        stop?.();
      };
    },
  };
}

export const traceTransitionStream = createTraceTransitionStream();
