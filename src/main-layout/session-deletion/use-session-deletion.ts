import { useCallback, useEffect, useRef, useState } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { agentService } from "../../services/runtime-agent-client";
import type { Session } from "../../types/agent";
import type { DeletionPreviewWorktree, SessionDeletionOperation } from "../../types/session-deletion";
import { isDeletionOutcomeTerminal } from "../../types/session-deletion";
import {
  buildChoices,
  canSubmit,
  emptyChoices,
  newRequestId,
  remainingSessions,
  retryAllowed,
  retryNeedsPreview,
  setAcknowledgement,
  toggleRemove,
  type DeletionDialogState,
  type RetryContext,
} from "./session-deletion-model";

const POLL_INTERVAL_MS = 500;
/** Consecutive status reads that may fail before the dialog stops claiming to follow the operation. */
const MAX_POLL_FAILURES = 10;

function errorText(reason: unknown): string {
  return reason instanceof Error ? reason.message : String(reason);
}

function deletedCount(operation: SessionDeletionOperation): number {
  return operation.groups.filter((group) => group.dbEffect === "deleted").length;
}

/**
 * Every visible delete entry point calls `request`. The hook previews, holds the user's explicit
 * choices for exactly one preview, executes, and follows the operation to its recorded end. It
 * never calls the legacy single-session delete.
 */
export function useSessionDeletion() {
  const queryClient = useQueryClient();
  const [state, setState] = useState<DeletionDialogState>({ status: "closed" });
  const generation = useRef(0);
  const executingOperationId = state.status === "executing" ? state.operationId : null;

  const invalidateSessions = useCallback(() => {
    void queryClient.invalidateQueries({ queryKey: ["sessions"] });
    void queryClient.invalidateQueries({ queryKey: ["session-categories"] });
    void queryClient.invalidateQueries({ queryKey: ["workflow"] });
  }, [queryClient]);

  const loadPreview = useCallback(async (sessions: Session[], retryOf: RetryContext | null) => {
    const token = generation.current + 1;
    generation.current = token;
    setState({ status: "loading", sessions, retryOf });
    try {
      const preview = await agentService.previewSessionDeletion({ sessionIds: sessions.map((session) => session.id) });
      if (generation.current !== token) return;
      setState({
        status: "ready",
        sessions,
        preview,
        choices: emptyChoices(preview),
        requestId: newRequestId(preview.previewId),
        error: null,
        retryOf,
      });
    } catch (reason: unknown) {
      if (generation.current !== token) return;
      setState({ status: "preview-failed", sessions, error: errorText(reason), retryOf });
    }
  }, []);

  const request = useCallback((sessions: Session[]) => {
    if (sessions.length === 0) return;
    void loadPreview(sessions, null);
  }, [loadPreview]);

  const close = useCallback(() => {
    // Closing mid-execution is refused: the dialog is the only surface that shows this operation.
    setState((current) => current.status === "executing" ? current : { status: "closed" });
  }, []);

  const toggleWorktree = useCallback((worktree: DeletionPreviewWorktree) => {
    setState((current) => current.status === "ready"
      ? { ...current, choices: toggleRemove(current.choices, worktree), error: null }
      : current);
  }, []);

  const acknowledgeIgnored = useCallback((worktree: DeletionPreviewWorktree, acknowledged: boolean) => {
    setState((current) => current.status === "ready"
      ? { ...current, choices: setAcknowledgement(current.choices, worktree, acknowledged) }
      : current);
  }, []);

  const refresh = useCallback(() => {
    if (state.status === "ready" || state.status === "preview-failed" || state.status === "loading") {
      void loadPreview(state.sessions, state.retryOf);
    }
  }, [state, loadPreview]);

  // Follows an accepted operation. Polling rather than events: the result must be observable
  // even if the event that announced it was missed, and the journal is the truth either way.
  // Keyed on the operation id alone so a progress update does not restart the timer.
  useEffect(() => {
    if (!executingOperationId) return;
    const operationId = executingOperationId;
    let cancelled = false;
    let inFlight = false;
    let failures = 0;
    let knownDeleted = 0;
    const timer = setInterval(() => {
      if (inFlight) return;
      inFlight = true;
      void agentService.getSessionDeletionOperation(operationId).then((operation: SessionDeletionOperation) => {
        if (cancelled) return;
        failures = 0;
        const deleted = deletedCount(operation);
        if (deleted > knownDeleted) {
          knownDeleted = deleted;
          invalidateSessions();
        }
        if (isDeletionOutcomeTerminal(operation.outcome)) {
          clearInterval(timer);
          setState((current) => current.status === "executing" && current.operationId === operationId
            ? { status: "settled", sessions: current.sessions, preview: current.preview, operation }
            : current);
        } else {
          setState((current) => current.status === "executing" && current.operationId === operationId
            ? { ...current, operation }
            : current);
        }
      }).catch((reason: unknown) => {
        if (cancelled) return;
        failures += 1;
        if (failures < MAX_POLL_FAILURES) return;
        // The operation can no longer be observed from here. The journal still holds its
        // real state; the dialog must not sit locked on a status it cannot read.
        clearInterval(timer);
        invalidateSessions();
        setState((current) => current.status === "executing" && current.operationId === operationId
          ? { status: "preview-failed", sessions: current.sessions, error: errorText(reason), retryOf: null }
          : current);
      }).finally(() => {
        inFlight = false;
      });
    }, POLL_INTERVAL_MS);
    return () => {
      cancelled = true;
      clearInterval(timer);
    };
  }, [executingOperationId, invalidateSessions]);

  const confirm = useCallback(async () => {
    if (state.status !== "ready" || !canSubmit(state.preview, state.choices)) return;
    const { preview, choices, requestId, sessions, retryOf } = state;
    const worktreeChoices = buildChoices(preview, choices);
    try {
      const handle = retryOf
        ? await agentService.retrySessionDeletion({
          operationId: retryOf.operationId,
          expectedRevision: retryOf.expectedRevision,
          retryRequestId: requestId,
          previewId: preview.previewId,
          worktreeChoices,
        })
        : await agentService.executeSessionDeletion({ requestId, previewId: preview.previewId, worktreeChoices });
      setState({ status: "executing", sessions, preview, requestId, operationId: handle.operationId, operation: null });
    } catch (reason: unknown) {
      setState((current) => current.status === "ready" ? { ...current, error: errorText(reason) } : current);
    }
  }, [state]);

  /** A new, explicitly authorized attempt for what did not finish. */
  const retry = useCallback(async () => {
    if (state.status !== "settled" || !retryAllowed(state.operation)) return;
    const { operation, sessions } = state;
    const retryOf = { operationId: operation.operationId, expectedRevision: operation.revision };
    if (!retryNeedsPreview(operation)) {
      // Database-only: the directory is confirmed gone, so no new preview is needed.
      try {
        const handle = await agentService.retrySessionDeletion({
          operationId: operation.operationId,
          expectedRevision: operation.revision,
          retryRequestId: newRequestId(operation.operationId),
          worktreeChoices: [],
        });
        setState({ status: "executing", sessions, preview: state.preview, requestId: handle.operationId, operationId: handle.operationId, operation: null });
      } catch (reason: unknown) {
        setState({ status: "preview-failed", sessions, error: errorText(reason), retryOf });
      }
      return;
    }
    void loadPreview(remainingSessions(sessions, operation), retryOf);
  }, [state, loadPreview]);

  return {
    state,
    busy: state.status === "loading" || state.status === "executing",
    request,
    close,
    toggleWorktree,
    acknowledgeIgnored,
    refresh,
    confirm,
    retry,
  };
}

export type SessionDeletionController = ReturnType<typeof useSessionDeletion>;
