import { describe, expect, it } from "vitest";
import type { SessionLogNotice } from "../types/session-log-notice";
import { decideLiveNotice, type LiveNoticeScope } from "./log-live-policy";

/**
 * What a live notice is allowed to do to a filtered list.
 *
 * The notice carries identifiers and never the log line, which is deliberate — the event channel
 * would otherwise carry the corpus, and a row would have two shapes that can disagree. The cost is
 * that some filters can be decided from a notice and some cannot, and the whole of this policy is
 * about not pretending otherwise. Guessing produces one of two failures: a row appears that the
 * active search excludes, or a row is withheld that it admits. Both read as a broken filter, and
 * neither can be diagnosed by looking harder at the list.
 */
function notice(overrides: Partial<SessionLogNotice> = {}): SessionLogNotice {
  return {
    noticeKind: "appended",
    recordId: "record-1",
    sequence: 1,
    occurredAt: "2026-08-25T10:00:00.000Z",
    level: "info",
    coverageState: "complete",
    sessionId: "session-1",
    ...overrides,
  };
}

function scope(overrides: Partial<LiveNoticeScope> = {}): LiveNoticeScope {
  return {
    levels: [],
    search: "",
    correlation: {},
    sessionId: "session-1",
    ...overrides,
  };
}

describe("live notice policy", () => {
  it("inserts when every active filter can be answered from the notice", () => {
    expect(decideLiveNotice(notice(), scope())).toBe("insert");
  });

  it("ignores a notice from another session", () => {
    expect(decideLiveNotice(notice({ sessionId: "session-2" }), scope())).toBe("ignore");
  });

  it("ignores a notice that names no session while a session is selected", () => {
    // Unattributed is not the same as matching. Admitting it would put a record in this session's
    // list on the strength of the notice failing to say otherwise.
    expect(decideLiveNotice(notice({ sessionId: undefined }), scope())).toBe("ignore");
  });

  it("ignores a level the list is not showing", () => {
    const decision = decideLiveNotice(notice({ level: "debug" }), scope({ levels: ["error"] }));

    expect(decision).toBe("ignore");
  });

  it("inserts a level the list is showing", () => {
    const decision = decideLiveNotice(
      notice({ level: "error" }),
      scope({ levels: ["error", "warn"] }),
    );

    expect(decision).toBe("insert");
  });

  it("treats an empty level filter as every level, matching what the query does", () => {
    expect(decideLiveNotice(notice({ level: "debug" }), scope({ levels: [] }))).toBe("insert");
  });

  it("ignores a record that does not carry the correlation being filtered on", () => {
    const decision = decideLiveNotice(
      notice({ runId: undefined }),
      scope({ correlation: { runId: "run-1" } }),
    );

    // The query follows the same rule: a record emitted without a run is not attributed to
    // whichever run happens to be selected, because that would make the filter look like evidence.
    expect(decision).toBe("ignore");
  });

  it("ignores a record carrying a different value for the filtered correlation", () => {
    const decision = decideLiveNotice(
      notice({ runId: "run-2" }),
      scope({ correlation: { runId: "run-1" } }),
    );

    expect(decision).toBe("ignore");
  });

  it("inserts a record carrying every filtered correlation", () => {
    const decision = decideLiveNotice(
      notice({
        agentId: "agent-1",
        operationId: "operation-1",
        runId: "run-1",
        seatId: "seat-1",
        spanId: "span-1",
        traceId: "trace-1",
      }),
      scope({
        correlation: {
          agentId: "agent-1",
          operationId: "operation-1",
          runId: "run-1",
          seatId: "seat-1",
          spanId: "span-1",
          traceId: "trace-1",
        },
      }),
    );

    expect(decision).toBe("insert");
  });

  it("ignores when one of several correlations disagrees", () => {
    const decision = decideLiveNotice(
      notice({ runId: "run-1", traceId: "trace-2" }),
      scope({ correlation: { runId: "run-1", traceId: "trace-1" } }),
    );

    expect(decision).toBe("ignore");
  });

  it("treats a blank correlation filter as no filter at all", () => {
    const decision = decideLiveNotice(
      notice({ runId: undefined }),
      scope({ correlation: { runId: "  " } }),
    );

    expect(decision).toBe("insert");
  });

  it("invalidates rather than guessing when a text search is active", () => {
    const decision = decideLiveNotice(notice(), scope({ search: "timeout" }));

    // The notice has no message, category or context to match. A search is exactly the filter
    // under which a reader is most sure that what they see is everything that matched.
    expect(decision).toBe("invalidate");
  });

  it("still ignores an out-of-scope record even while a search is active", () => {
    const decision = decideLiveNotice(
      notice({ level: "debug" }),
      scope({ levels: ["error"], search: "timeout" }),
    );

    // Cheaper and more accurate than invalidating: the level already excluded it, so the search
    // never needs to be considered and the page does not have to be thrown away.
    expect(decision).toBe("ignore");
  });

  it("invalidates on a gap, because a hole changes what the page is missing", () => {
    const decision = decideLiveNotice(
      notice({ noticeKind: "gap", recordId: "", droppedCount: 3, sessionId: undefined }),
      scope(),
    );

    // There is no row to insert and no way to ignore it honestly: the page it describes is now
    // claiming a completeness it does not have.
    expect(decision).toBe("invalidate");
  });

  it("invalidates on a gap even when a filter would have excluded a row", () => {
    const decision = decideLiveNotice(
      notice({ noticeKind: "gap", level: "warn", recordId: "", sessionId: undefined }),
      scope({ levels: ["error"] }),
    );

    // The records behind a gap were never read, so nothing is known about their levels. Filtering
    // it out would be excluding records on the strength of a field that describes the gap notice
    // rather than the records it stands for.
    expect(decision).toBe("invalidate");
  });
});
