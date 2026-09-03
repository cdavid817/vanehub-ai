import type {
  SessionLogNotice,
  SessionLogUnsubscribe,
} from "../types/session-log-notice";

export interface SessionLogNoticeDispatcherInput {
  /** Last sequence the caller already has. Anything at or below it is a replay. */
  fromSequence?: number;
  listener: (notice: SessionLogNotice) => void;
}

export interface SessionLogNoticeDispatcher {
  /** Feed one already-parsed notice. Returns whether it reached the listener. */
  accept: (notice: SessionLogNotice) => boolean;
  lastSequence: () => number;
}

/**
 * Sequence handling shared by every runtime, so the desktop and the Web mock cannot drift on the
 * one behaviour a subscriber actually depends on.
 *
 * De-duplication. A bootstrap watermark and a live subscription overlap by design — the caller
 * subscribes first and reads the watermark second, so a notice can be in both — and a notice at or
 * below the last seen sequence is dropped rather than applied twice.
 *
 * Gap detection. The bridge between a log append and the index is bounded and lossy on purpose:
 * the record is already durable on disk, and the index is rebuildable from the file it is in. What
 * must not happen is a subscriber treating its locally accumulated rows as the whole log. A jump in
 * the sequence is evidence that something was lost, so a synthetic gap is emitted before the notice
 * that revealed the jump.
 *
 * Native gap notices arrive here too, and they are *not* the same thing: one says "the bridge
 * dropped receipts", the other says "notices did not reach this subscriber". Both leave the view
 * short, so both are reported — but a native gap carries its own count and is passed through
 * without a second one being synthesised around it.
 */
export function createSessionLogNoticeDispatcher({
  fromSequence = 0,
  listener,
}: SessionLogNoticeDispatcherInput): SessionLogNoticeDispatcher {
  let lastSequence = fromSequence;

  return {
    accept(notice) {
      if (notice.sequence <= lastSequence) return false;

      // A native gap shares the sequence space but does not advance it by one per record, so the
      // arithmetic below would invent a delivery gap on top of the loss it already describes.
      if (notice.noticeKind === "gap") {
        lastSequence = notice.sequence;
        listener(notice);
        return true;
      }

      const missing = notice.sequence - lastSequence - 1;
      if (missing > 0) {
        listener({
          noticeKind: "gap",
          recordId: "",
          sequence: notice.sequence,
          occurredAt: notice.occurredAt,
          level: "warn",
          coverageState: notice.coverageState,
          droppedCount: missing,
          reasonCode: "log_notice_not_delivered",
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
export function onceSessionLogUnsubscribe(
  unsubscribe: SessionLogUnsubscribe,
): SessionLogUnsubscribe {
  let released = false;
  return () => {
    if (released) return;
    released = true;
    unsubscribe();
  };
}
