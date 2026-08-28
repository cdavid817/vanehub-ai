import type {
  EvidenceSessionId,
  ExecutionEvidenceNotice,
  Unsubscribe,
} from "../types/session-workspace-evidence";

export interface EvidenceNoticeDispatcherInput {
  sessionId: EvidenceSessionId;
  /** Last sequence the caller already has. Anything at or below it is a replay. */
  fromSequence?: number;
  listener: (notice: ExecutionEvidenceNotice) => void;
}

export interface EvidenceNoticeDispatcher {
  /** Feed one already-parsed notice. Returns whether it reached the listener. */
  accept: (notice: ExecutionEvidenceNotice) => boolean;
  lastSequence: () => number;
}

/**
 * Sequence handling shared by every runtime, so the desktop and the Web mock cannot drift on the
 * one behaviour a subscriber actually depends on.
 *
 * Two things happen here that a raw event listener does not do:
 *
 * De-duplication. An attach snapshot and a live subscription overlap by design — a frame can be
 * in both — so a notice at or below the last seen sequence is dropped rather than applied twice.
 *
 * Gap detection. A bounded queue drops notices under load. A jump in the sequence is the only
 * evidence that happened, so one synthetic `coverage-gap` notice is emitted before the notice
 * that revealed the jump. Without it the subscriber would treat its locally accumulated rows as
 * complete, which is precisely the false claim this change exists to remove.
 */
export function createEvidenceNoticeDispatcher({
  sessionId,
  fromSequence = 0,
  listener,
}: EvidenceNoticeDispatcherInput): EvidenceNoticeDispatcher {
  let lastSequence = fromSequence;

  return {
    accept(notice) {
      if (notice.sessionId !== sessionId) return false;
      if (notice.sequence <= lastSequence) return false;

      const missing = notice.sequence - lastSequence - 1;
      if (missing > 0) {
        listener({
          kind: "coverage-gap",
          sequence: notice.sequence,
          sessionId,
          occurredAt: notice.occurredAt,
          droppedCount: missing,
        });
      }

      lastSequence = notice.sequence;
      listener(notice);
      return true;
    },
    lastSequence: () => lastSequence,
  };
}

/**
 * Wraps an unsubscribe so calling it twice is a no-op. React cleanup can run more than once in
 * development and after a fast re-render; a second call must not tear down a later subscription.
 */
export function onceUnsubscribe(unsubscribe: Unsubscribe): Unsubscribe {
  let released = false;
  return () => {
    if (released) return;
    released = true;
    unsubscribe();
  };
}
