/**
 * The live session-log notice contract.
 *
 * A notice announces that the index changed; it never carries what changed. No message, no
 * category, no safe context — a subscriber that wants the row fetches it by id. Two reasons, and
 * both matter: the event channel would otherwise carry the whole corpus, and a row would have two
 * shapes that can disagree about what it says.
 */

/** What a notice announces. A gap has no row behind it, which is why it is not an appended one. */
export type SessionLogNoticeKind = "appended" | "gap";

export type SessionLogLevel = "error" | "warn" | "info" | "debug";

export type SessionLogCoverageState = "complete" | "indexing" | "partial" | "unavailable";

/**
 * Identifiers, ordering, correlation and coverage.
 *
 * `recordId` is empty on a gap because there is nothing to fetch. Every correlation field is
 * optional because a record carries the ones its producer attached and no others — absent rather
 * than null, so a reader tests one thing instead of two.
 */
export interface SessionLogNotice {
  noticeKind: SessionLogNoticeKind;
  recordId: string;
  sequence: number;
  occurredAt: string;
  level: SessionLogLevel;
  coverageState: SessionLogCoverageState;
  sessionId?: string;
  runId?: string;
  traceId?: string;
  spanId?: string;
  operationId?: string;
  agentId?: string;
  seatId?: string;
  /** Gap only: how many records were lost, and the stable code saying why. Never what they said. */
  droppedCount?: number;
  reasonCode?: string;
}

export interface SessionLogSubscription {
  /**
   * Resume point. Notices at or below this are replays of what the caller already has, and are
   * dropped rather than applied twice.
   */
  fromSequence?: number;
}

export type SessionLogUnsubscribe = () => void;

export interface SessionLogNoticeStream {
  /**
   * Registers a listener and returns its release.
   *
   * Listener-first by contract: the caller subscribes, then reads the watermark. Reading first
   * would lose every notice published in between, and the sequences the subscriber went on to see
   * would be contiguous — so the loss would be invisible, which is the one outcome this whole
   * surface exists to prevent.
   */
  subscribe: (
    subscription: SessionLogSubscription,
    listener: (notice: SessionLogNotice) => void,
  ) => SessionLogUnsubscribe;
}
