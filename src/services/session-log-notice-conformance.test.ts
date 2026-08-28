import { describe, expect, it } from "vitest";
import { createFixtureSessionLogTransport } from "../contracts/fixtures/session-log-notice-transport";
import type { SessionLogNotice } from "../types/session-log-notice";
import { createTauriSessionLogClient } from "./tauri-session-log-client";
import { createWebSessionLogClient } from "./web-session-log-client";

/**
 * One suite, two runtimes.
 *
 * The behaviour a subscriber depends on — de-duplication, gap detection, an unsubscribe that can be
 * called twice — has to be identical on the desktop and in the browser, or the browser build
 * becomes the place where it is not checked. Running the same cases against both is the only way
 * that stays true as either side changes.
 *
 * The desktop client runs against a fixture transport rather than a live command, which is the
 * point of the seam: what is under test is the client's behaviour, and it can be settled without a
 * running Tauri runtime.
 */
interface ConformanceRuntime {
  subscribe: (
    fromSequence: number | undefined,
    listener: (notice: SessionLogNotice) => void,
  ) => Promise<() => void>;
  /** Publishes one appended notice at the given sequence. */
  publish: (sequence: number) => Promise<void>;
  /** Publishes a native gap notice at the given sequence. */
  publishGap: (sequence: number, droppedCount: number) => Promise<void>;
}

const runtimes: { name: string; create: () => ConformanceRuntime }[] = [
  {
    name: "Tauri fixture transport",
    create: () => {
      const transport = createFixtureSessionLogTransport();
      const client = createTauriSessionLogClient(transport);
      const raw = (sequence: number, extra: Record<string, unknown>) => ({
        noticeKind: "appended",
        recordId: `record-${sequence}`,
        sequence,
        occurredAt: "2026-08-24T10:00:00.000Z",
        level: "info",
        coverageState: "complete",
        sessionId: "session-1",
        ...extra,
      });
      return {
        subscribe: (fromSequence, listener) =>
          client.subscribeAsync({ fromSequence }, listener),
        publish: async (sequence) => {
          transport.publish(raw(sequence, {}));
        },
        publishGap: async (sequence, droppedCount) => {
          transport.publish(
            raw(sequence, {
              noticeKind: "gap",
              recordId: "",
              level: "warn",
              coverageState: "partial",
              sessionId: undefined,
              droppedCount,
              reasonCode: "log_receipt_dropped",
            }),
          );
        },
      };
    },
  },
  {
    name: "Web/mock",
    create: () => {
      const client = createWebSessionLogClient();
      const advanceTo = (sequence: number) => sequence - client.watermarkSequence();
      return {
        subscribe: async (fromSequence, listener) =>
          client.subscribe({ fromSequence }, listener),
        publish: async (sequence) => {
          client.emitSimulatedNotice({ advanceBy: advanceTo(sequence) });
        },
        publishGap: async (sequence, droppedCount) => {
          client.emitSimulatedNotice({
            noticeKind: "gap",
            advanceBy: advanceTo(sequence),
            droppedCount,
          });
        },
      };
    },
  },
];

describe.each(runtimes)("session log notice conformance: $name", ({ create }) => {
  it("delivers notices in sequence to a subscriber that resumed from a watermark", async () => {
    const runtime = create();
    const seen: SessionLogNotice[] = [];
    await runtime.subscribe(10, (notice) => seen.push(notice));

    await runtime.publish(11);
    await runtime.publish(12);

    expect(seen.map((notice) => notice.sequence)).toEqual([11, 12]);
    expect(seen.every((notice) => notice.noticeKind === "appended")).toBe(true);
  });

  it("drops a replay at or below the watermark instead of applying it twice", async () => {
    const runtime = create();
    const seen: SessionLogNotice[] = [];
    await runtime.subscribe(10, (notice) => seen.push(notice));

    // The overlap a listener-first subscription creates by design: the watermark is read after the
    // listener is registered, so a notice can be in both.
    await runtime.publish(9);
    await runtime.publish(10);
    await runtime.publish(11);

    expect(seen.map((notice) => notice.sequence)).toEqual([11]);
  });

  it("reports a jump in the sequence as a gap before the notice that revealed it", async () => {
    const runtime = create();
    const seen: SessionLogNotice[] = [];
    await runtime.subscribe(0, (notice) => seen.push(notice));

    await runtime.publish(1);
    // Two notices never arrived. Nothing else says so: the subscriber's own view would simply be
    // short, and would look complete.
    await runtime.publish(4);

    expect(seen.map((notice) => notice.noticeKind)).toEqual(["appended", "gap", "appended"]);
    expect(seen[1].droppedCount).toBe(2);
    expect(seen[1].recordId).toBe("");
    expect(seen[2].sequence).toBe(4);
  });

  it("passes a native gap through without inventing a second one around it", async () => {
    const runtime = create();
    const seen: SessionLogNotice[] = [];
    await runtime.subscribe(0, (notice) => seen.push(notice));

    await runtime.publish(1);
    // The bridge dropped receipts and said so. Its sequence does not advance one-per-record, so
    // treating the jump as a delivery gap would report the same loss twice under two reasons.
    await runtime.publishGap(5, 3);

    expect(seen.map((notice) => notice.noticeKind)).toEqual(["appended", "gap"]);
    expect(seen.filter((notice) => notice.noticeKind === "gap")).toHaveLength(1);
    expect(seen[1].droppedCount).toBe(3);
  });

  it("never puts log content on a notice", async () => {
    const runtime = create();
    const seen: SessionLogNotice[] = [];
    await runtime.subscribe(0, (notice) => seen.push(notice));

    await runtime.publish(1);
    await runtime.publishGap(2, 1);

    for (const notice of seen) {
      const keys = Object.keys(notice);
      // The line itself, its category, and its safe context all stay behind the fetch-by-id path:
      // the event channel is where redaction cannot be re-applied.
      expect(keys).not.toContain("message");
      expect(keys).not.toContain("category");
      expect(keys).not.toContain("context");
      expect(keys).not.toContain("path");
    }
  });

  it("releases a subscription once, however many times it is called", async () => {
    const runtime = create();
    const first: SessionLogNotice[] = [];
    const second: SessionLogNotice[] = [];
    const releaseFirst = await runtime.subscribe(0, (notice) => first.push(notice));
    await runtime.subscribe(0, (notice) => second.push(notice));

    releaseFirst();
    // React cleanup runs more than once in development and after a fast re-render. A second call
    // must not reach past its own subscription.
    releaseFirst();
    releaseFirst();
    await runtime.publish(1);

    expect(first).toHaveLength(0);
    expect(second.map((notice) => notice.sequence)).toEqual([1]);
  });
});

describe("the desktop client's own wire handling", () => {
  it("holds notices that arrive before the watermark is known, then replays them in order", async () => {
    const transport = createFixtureSessionLogTransport();
    transport.setWatermark(5);
    const client = createTauriSessionLogClient(transport);
    const seen: SessionLogNotice[] = [];

    const raw = (sequence: number) => ({
      noticeKind: "appended",
      recordId: `record-${sequence}`,
      sequence,
      occurredAt: "2026-08-24T10:00:00.000Z",
      level: "info",
      coverageState: "complete",
    });

    // The listener is registered synchronously inside subscribeAsync, and the bootstrap it awaits
    // is a microtask — so publishing before the await settles is exactly the window this buffers.
    const pending = client.subscribeAsync({}, (notice) => seen.push(notice));
    transport.publish(raw(7));
    transport.publish(raw(6));
    await pending;

    // Replayed by sequence rather than by arrival: out-of-order replay would read as a jump and
    // report a gap that never happened.
    expect(seen.map((notice) => notice.sequence)).toEqual([6, 7]);
    expect(seen.every((notice) => notice.noticeKind === "appended")).toBe(true);
  });

  it("drops a malformed frame without tearing down the subscription", async () => {
    const transport = createFixtureSessionLogTransport();
    const client = createTauriSessionLogClient(transport);
    const seen: SessionLogNotice[] = [];
    await client.subscribeAsync({ fromSequence: 0 }, (notice) => seen.push(notice));

    // An event handler has no caller to reject to, so a bad frame is dropped rather than thrown.
    transport.publish(null);
    transport.publish({ noticeKind: "invented", sequence: 1 });
    transport.publish({ noticeKind: "appended", sequence: "not a number" });
    transport.publish({
      noticeKind: "appended",
      recordId: "record-1",
      sequence: 1,
      occurredAt: "2026-08-24T10:00:00.000Z",
      level: "info",
      coverageState: "complete",
    });

    expect(seen.map((notice) => notice.sequence)).toEqual([1]);
    expect(transport.handlerCount()).toBe(1);
  });

  it("subscribes anyway when the bootstrap cannot be read", async () => {
    const transport = createFixtureSessionLogTransport();
    const failing = {
      ...transport,
      invokeSessionLog: () => Promise.reject(new Error("index unavailable")),
    };
    const client = createTauriSessionLogClient(failing);
    const seen: SessionLogNotice[] = [];

    // Refusing to subscribe would leave the view with no live updates at all; resuming from zero
    // replays what the caller may already have, and the dispatcher de-duplicates that.
    await client.subscribeAsync({}, (notice) => seen.push(notice));
    transport.publish({
      noticeKind: "appended",
      recordId: "record-1",
      sequence: 1,
      occurredAt: "2026-08-24T10:00:00.000Z",
      level: "info",
      coverageState: "unavailable",
    });

    expect(seen.map((notice) => notice.sequence)).toEqual([1]);
  });
});
