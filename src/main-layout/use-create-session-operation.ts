import { useEffect, useRef } from "react";
import { operationService } from "../services/runtime-operation-client";
import { conciseError, resolveCreatedSession } from "./create-session-dialog-utils";
import type { Session } from "../types/agent";

/** How long between polls of a creation operation that is still queued or running. */
const POLL_INTERVAL_MS = 600;

/**
 * How many times the canonical session read is retried after the operation already succeeded.
 *
 * Bounded because a read that keeps failing is a real problem the user needs to be told about, and
 * an unbounded retry would spin quietly forever instead. Small because the operation has already
 * succeeded: the session exists, and this is only fetching its canonical shape.
 */
const MAX_RESOLVE_ATTEMPTS = 5;

/**
 * How long the whole watch may run before it reports that it gave up.
 *
 * Separate from the retry count, because the two bound different failures. The count bounds a read
 * that keeps throwing; this bounds an operation that never leaves `queued` or `running` and so
 * never throws at all. That state is reachable: when the native side persists a session but cannot
 * record the completion, the operation record stays non-terminal, and a client polling it would
 * otherwise wait forever with a spinner and no explanation.
 */
const WATCH_BUDGET_MS = 120_000;

/**
 * Watches one session-creation operation through to a delivered session.
 *
 * Creation has more states than the two booleans it used to be tracked with. In particular
 * "the operation succeeded but reading the created session failed" is its own state, and it was
 * previously indistinguishable from failure: the operation was marked handled *before* the read,
 * so a transient read error ended the flow with the session already created. The user's only
 * remaining move was to create it again, which makes a second one.
 *
 * Three rules hold this together, and each exists because breaking it produces a session the user
 * did not ask for:
 *
 * - An operation is handled only once a session has been delivered.
 * - A failed read retries the read. Creation is never resubmitted as recovery: retrying a read is
 *   idempotent, retrying a creation is not.
 * - The dialog stays busy for the entire watch. Clearing it between a failed read and its retry
 *   would re-enable the submit button with no spinner and no error on screen, which is an
 *   invitation to press it again.
 */
export function useCreateSessionOperation({
  active,
  handledOperationId,
  onCreated,
  operationId,
  setError,
  setHandledOperationId,
  setLoading,
  t,
}: {
  /**
   * Whether the creation surface is still open.
   *
   * The dialog returns null when closed but keeps every hook mounted, so without this the watch
   * outlives the surface: a user who cancels still gets navigated into the session when the
   * operation lands. The session itself is not cancelled — it stays reachable from the session
   * list — but they asked not to be taken there.
   */
  active: boolean;
  handledOperationId: string | null;
  onCreated: (session: Session) => void;
  operationId: string | null;
  setError: (value: string | null) => void;
  setHandledOperationId: (value: string | null) => void;
  setLoading: (value: boolean) => void;
  t: (key: string) => string;
}) {
  // Counted per creation, not per effect run, and in a ref rather than state: it drives nothing on
  // screen, and a re-render per attempt would restart the very effect doing the retrying.
  //
  // The distinction matters because this effect re-runs far more often than the creation changes.
  // Its callers pass `onCreated` as an inline closure, so every parent render produces a new
  // identity and re-runs the effect; resetting the counter there would put the ceiling permanently
  // out of reach and turn a bounded retry into an unbounded one.
  const watch = useRef<{ operationId: string | null; attempts: number; startedAt: number }>({
    operationId: null,
    attempts: 0,
    startedAt: 0,
  });

  // Held in refs and kept out of the dependency list. Callers pass `onCreated` as an inline arrow
  // and `t` comes from `useTranslation`, so both change identity on every parent render. Depending
  // on them tore down the effect and re-entered `poll()` immediately each time -- the retry
  // interval was never actually waited, and a few renders during a transient failure could burn
  // the whole attempt budget in milliseconds and report a failure that had not happened.
  const callbacks = useRef({ onCreated, setError, setHandledOperationId, setLoading, t });
  callbacks.current = { onCreated, setError, setHandledOperationId, setLoading, t };

  useEffect(() => {
    if (!active || !operationId || handledOperationId === operationId) return;
    let cancelled = false;
    let timer: number | undefined;
    if (watch.current.operationId !== operationId) {
      watch.current = { operationId, attempts: 0, startedAt: Date.now() };
    }

    const later = (run: () => void) => {
      timer = window.setTimeout(run, POLL_INTERVAL_MS);
    };

    /** Ends the watch: the dialog stops being busy and the operation stops being polled. */
    const settle = (message: string) => {
      callbacks.current.setLoading(false);
      callbacks.current.setHandledOperationId(operationId);
      callbacks.current.setError(message);
    };

    const outOfBudget = () => Date.now() - watch.current.startedAt > WATCH_BUDGET_MS;

    async function deliver(result: Parameters<typeof resolveCreatedSession>[0]) {
      const session = await resolveCreatedSession(result);
      if (cancelled) return;
      if (!session) {
        // A result that does not describe a session is not a transient read failure; retrying
        // would ask the same question and get the same answer.
        settle(callbacks.current.t("createSession.error.command"));
        return;
      }
      // Handled only now. Marking it earlier is what turned a failed read into a dead end.
      callbacks.current.setLoading(false);
      callbacks.current.setHandledOperationId(operationId);
      callbacks.current.onCreated(session);
    }

    async function poll() {
      try {
        const operation = await operationService.getOperationStatus(operationId as string);
        if (cancelled) return;
        if (operation.status === "queued" || operation.status === "running") {
          if (outOfBudget()) {
            settle(callbacks.current.t("createSession.error.command"));
            return;
          }
          later(() => void poll());
          return;
        }
        if (operation.status === "failed") {
          settle(operation.error ?? callbacks.current.t("createSession.error.command"));
          return;
        }
        await deliver(operation.result);
      } catch (operationError) {
        if (cancelled) return;
        watch.current.attempts += 1;
        if (watch.current.attempts < MAX_RESOLVE_ATTEMPTS && !outOfBudget()) {
          // The operation is identified and may well have succeeded. Reading again is safe;
          // creating again is not, so this retries the read and nothing else. The dialog stays
          // busy meanwhile — a live submit button between attempts is how a second session gets
          // created while the first is still being recovered.
          later(() => void poll());
          return;
        }
        settle(conciseError(operationError, callbacks.current.t));
      }
    }

    void poll();
    return () => {
      cancelled = true;
      if (timer !== undefined) window.clearTimeout(timer);
    };
    // Only what actually identifies this watch. Anything else re-enters the poll loop.
  }, [active, handledOperationId, operationId]);
}
