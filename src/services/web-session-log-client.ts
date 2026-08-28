import type {
  SessionLogNotice,
  SessionLogNoticeStream,
  SessionLogSubscription,
  SessionLogUnsubscribe,
} from "../types/session-log-notice";
import {
  createSessionLogNoticeDispatcher,
  onceSessionLogUnsubscribe,
} from "./session-log-notice-stream";

/**
 * The clock, the ids and the sequence are fixed rather than derived from the wall clock or a
 * counter seeded by one.
 *
 * A mock that stamped `Date.now()` would make every assertion about ordering depend on when the
 * test ran, and the failure would surface as a flake on a loaded machine rather than as the
 * ordering bug it was supposed to catch. Everything here is a pure function of how many notices
 * have been emitted.
 */
const MOCK_EPOCH_MS = Date.parse("2026-08-24T10:00:00.000Z");
const MOCK_STEP_MS = 1000;

function mockOccurredAt(sequence: number): string {
  return new Date(MOCK_EPOCH_MS + sequence * MOCK_STEP_MS).toISOString();
}

function mockRecordId(sequence: number): string {
  return `mock-log-record-${sequence}`;
}

export interface SimulatedSessionLogNotice {
  noticeKind?: SessionLogNotice["noticeKind"];
  level?: SessionLogNotice["level"];
  coverageState?: SessionLogNotice["coverageState"];
  sessionId?: string;
  /**
   * Skips ahead, which is how a test stages a delivery gap: the dispatcher reads the jump and
   * emits the synthetic gap that a real dropped notice would have produced.
   */
  advanceBy?: number;
  droppedCount?: number;
  reasonCode?: string;
}

export interface WebSessionLogClient extends SessionLogNoticeStream {
  /** Publishes one notice to every live subscriber, exactly as the native event channel would. */
  emitSimulatedNotice: (notice?: SimulatedSessionLogNotice) => SessionLogNotice;
  /** The watermark a subscriber resumes from, as the native bootstrap command reports it. */
  watermarkSequence: () => number;
}

/**
 * The Web/mock log-notice stream.
 *
 * Shares the dispatcher with the desktop client rather than reimplementing de-duplication and gap
 * detection, because the point of a mock is that a view behaves the same against it. A second
 * implementation of the one behaviour a subscriber depends on would make the browser build the
 * place where that behaviour is *not* checked.
 */
export function createWebSessionLogClient(): WebSessionLogClient {
  let sequence = 0;
  const listeners = new Set<(notice: SessionLogNotice) => void>();

  return {
    subscribe(input: SessionLogSubscription, listener): SessionLogUnsubscribe {
      // Listener first, watermark second — the same order the desktop client uses, so a test that
      // stages a race sees the same outcome in both runtimes.
      const dispatcher = createSessionLogNoticeDispatcher({
        fromSequence: input.fromSequence ?? sequence,
        listener,
      });
      const accept = (notice: SessionLogNotice) => {
        dispatcher.accept(notice);
      };
      listeners.add(accept);
      return onceSessionLogUnsubscribe(() => {
        listeners.delete(accept);
      });
    },

    emitSimulatedNotice(notice = {}) {
      sequence += notice.advanceBy ?? 1;
      const kind = notice.noticeKind ?? "appended";
      const published: SessionLogNotice = {
        noticeKind: kind,
        // A gap has no row to fetch, in the mock exactly as on the desktop: a view that reached
        // for one here would pass against the mock and fail against the real runtime.
        recordId: kind === "gap" ? "" : mockRecordId(sequence),
        sequence,
        occurredAt: mockOccurredAt(sequence),
        level: notice.level ?? (kind === "gap" ? "warn" : "info"),
        coverageState: notice.coverageState ?? (kind === "gap" ? "partial" : "complete"),
        sessionId: kind === "gap" ? undefined : (notice.sessionId ?? "session-1"),
        droppedCount: kind === "gap" ? (notice.droppedCount ?? 1) : undefined,
        reasonCode: kind === "gap" ? (notice.reasonCode ?? "log_receipt_dropped") : undefined,
      };
      for (const listener of [...listeners]) listener(published);
      return published;
    },

    watermarkSequence: () => sequence,
  };
}
