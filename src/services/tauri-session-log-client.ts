import type {
  SessionLogNotice,
  SessionLogNoticeStream,
  SessionLogSubscription,
  SessionLogUnsubscribe,
} from "../types/session-log-notice";
import type { NativeSessionLogTransport } from "./native-session-log-transport";
import {
  createSessionLogNoticeDispatcher,
  onceSessionLogUnsubscribe,
  type SessionLogNoticeDispatcher,
} from "./session-log-notice-stream";

const NOTICE_KINDS = new Set(["appended", "gap"]);
const LEVELS = new Set(["error", "warn", "info", "debug"]);
const COVERAGE_STATES = new Set(["complete", "indexing", "partial", "unavailable"]);

function optionalText(value: unknown): string | undefined {
  return typeof value === "string" && value.length > 0 ? value : undefined;
}

/**
 * Validates a notice off the event channel.
 *
 * Strict about the discriminant and lenient about the optional correlation, because those are two
 * different risks: a payload whose kind or level is unrecognised would be routed by a `switch` that
 * has no branch for it, while a missing `runId` is the ordinary case of a record whose producer did
 * not attach one.
 */
export function parseSessionLogNotice(payload: unknown): SessionLogNotice {
  if (typeof payload !== "object" || payload === null) {
    throw new Error("session log notice is not an object");
  }
  const raw = payload as Record<string, unknown>;
  const noticeKind = raw.noticeKind;
  const level = raw.level;
  const coverageState = raw.coverageState;
  if (typeof noticeKind !== "string" || !NOTICE_KINDS.has(noticeKind)) {
    throw new Error("session log notice has no recognised kind");
  }
  if (typeof level !== "string" || !LEVELS.has(level)) {
    throw new Error("session log notice has no recognised level");
  }
  if (typeof coverageState !== "string" || !COVERAGE_STATES.has(coverageState)) {
    throw new Error("session log notice has no recognised coverage state");
  }
  if (typeof raw.sequence !== "number" || !Number.isFinite(raw.sequence)) {
    throw new Error("session log notice has no sequence");
  }
  return {
    noticeKind: noticeKind as SessionLogNotice["noticeKind"],
    recordId: typeof raw.recordId === "string" ? raw.recordId : "",
    sequence: raw.sequence,
    occurredAt: typeof raw.occurredAt === "string" ? raw.occurredAt : "",
    level: level as SessionLogNotice["level"],
    coverageState: coverageState as SessionLogNotice["coverageState"],
    sessionId: optionalText(raw.sessionId),
    runId: optionalText(raw.runId),
    traceId: optionalText(raw.traceId),
    spanId: optionalText(raw.spanId),
    operationId: optionalText(raw.operationId),
    agentId: optionalText(raw.agentId),
    seatId: optionalText(raw.seatId),
    droppedCount: typeof raw.droppedCount === "number" ? raw.droppedCount : undefined,
    reasonCode: optionalText(raw.reasonCode),
  };
}

function safeParse(payload: unknown): SessionLogNotice | null {
  try {
    return parseSessionLogNotice(payload);
  } catch {
    return null;
  }
}

async function resumeSequence(
  transport: NativeSessionLogTransport,
  input: SessionLogSubscription,
): Promise<number> {
  if (typeof input.fromSequence === "number") return input.fromSequence;
  try {
    const bootstrap = await transport.invokeSessionLog(
      "get_session_log_subscription_bootstrap",
      {},
    );
    const watermark = (bootstrap as Record<string, unknown> | null)?.watermarkSequence;
    return typeof watermark === "number" ? watermark : 0;
  } catch {
    // A bootstrap that cannot be read is not a reason to drop the subscription. Resuming from zero
    // replays notices the caller may already have, and the dispatcher de-duplicates those; refusing
    // to subscribe would instead leave the view with no live updates at all.
    return 0;
  }
}

/**
 * The desktop log-notice stream, built around an injected transport.
 *
 * Listener first, watermark second, buffer in between. Reading the watermark first and subscribing
 * after would lose every notice published in that window, and nothing downstream could detect the
 * loss: from the subscriber's point of view the sequences would be contiguous. So the listener goes
 * up first, everything arriving before the watermark is known is held, and the buffer is replayed
 * through the same dispatcher that de-duplicates and detects gaps for live notices.
 */
export function createTauriSessionLogClient(
  transport: NativeSessionLogTransport,
): SessionLogNoticeStream & {
  subscribeAsync: (
    subscription: SessionLogSubscription,
    listener: (notice: SessionLogNotice) => void,
  ) => Promise<SessionLogUnsubscribe>;
} {
  async function subscribeAsync(
    input: SessionLogSubscription,
    listener: (notice: SessionLogNotice) => void,
  ): Promise<SessionLogUnsubscribe> {
    let dispatcher: SessionLogNoticeDispatcher | null = null;
    const buffered: SessionLogNotice[] = [];
    const unsubscribe = await transport.subscribeSessionLogNotices((payload) => {
      // A malformed notice is dropped rather than thrown: an event handler has no caller to reject
      // to, and one bad frame must not tear down a live subscription.
      const parsed = safeParse(payload);
      if (!parsed) return;
      if (dispatcher) dispatcher.accept(parsed);
      else buffered.push(parsed);
    });

    const watermark = await resumeSequence(transport, input);
    dispatcher = createSessionLogNoticeDispatcher({ fromSequence: watermark, listener });
    // Ordered by sequence rather than by arrival: gap detection reads a jump in the sequence, and
    // replaying two frames out of order would report a gap that never happened.
    for (const notice of [...buffered].sort((left, right) => left.sequence - right.sequence)) {
      dispatcher.accept(notice);
    }
    return onceSessionLogUnsubscribe(unsubscribe);
  }

  return {
    subscribe(input, listener) {
      // The synchronous shape a React effect can call and clean up. The release is captured once
      // the async subscription settles; calling it before then still cancels, because the pending
      // promise chain checks the flag.
      let released = false;
      let release: SessionLogUnsubscribe | null = null;
      void subscribeAsync(input, listener).then((unsubscribe) => {
        if (released) unsubscribe();
        else release = unsubscribe;
      });
      return onceSessionLogUnsubscribe(() => {
        released = true;
        release?.();
      });
    },
    subscribeAsync,
  };
}
