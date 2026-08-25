// @vitest-environment jsdom

import { act, renderHook, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { SessionLogNotice } from "../types/session-log-notice";
import type { SessionLogEntry, SessionLogPage } from "../types/session-workspace";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionLogs: vi.fn(),
    getSessionLogRecord: vi.fn(),
    exportSessionLogs: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));

import { useSessionLogs } from "./use-session-logs";

function entry(id: string, message = `message ${id}`): SessionLogEntry {
  return {
    id,
    timestamp: "2026-08-25T10:00:00.000Z",
    level: "info",
    category: "session.runtime",
    message,
    context: {},
  };
}

function page(items: SessionLogEntry[]): SessionLogPage {
  return {
    items,
    truncated: false,
    nextCursor: null,
    coverage: { state: "complete", droppedCount: 0, truncated: false, reasonCodes: [] },
  };
}

function notice(overrides: Partial<SessionLogNotice> = {}): SessionLogNotice {
  return {
    noticeKind: "appended",
    recordId: "log-live",
    sequence: 10,
    occurredAt: "2026-08-25T10:00:05.000Z",
    level: "info",
    coverageState: "complete",
    sessionId: "session-1",
    ...overrides,
  };
}

function mount(overrides: { search?: string; levels?: SessionLogEntry["level"][] } = {}) {
  return renderHook(() => useSessionLogs({
    levels: overrides.levels ?? [],
    scope: {},
    search: overrides.search ?? "",
    sessionId: "session-1",
  }));
}

/**
 * A live notice announces that a row exists. It does not carry the row, and it cannot answer every
 * filter — so the three things a view may do with one are the whole subject here.
 */
describe("live log insertion", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.listSessionLogs.mockResolvedValue(page([entry("log-1")]));
  });

  it("fetches the announced row and puts it at the newest edge", async () => {
    mockAgentService.getSessionLogRecord.mockResolvedValue(entry("log-live", "the new line"));
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    await act(async () => {
      await result.current.applyLiveNotice(notice());
    });

    // Fetched by id rather than carried on the event: one authoritative shape for a row, instead
    // of two that can disagree about what it says.
    expect(mockAgentService.getSessionLogRecord).toHaveBeenCalledWith("log-live");
    expect(result.current.entries[0].message).toBe("the new line");
    expect(result.current.entries).toHaveLength(2);
  });

  it("does not fetch a row the current filters already exclude", async () => {
    const { result } = mount({ levels: ["error"] });
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    const decision = await act(async () => result.current.applyLiveNotice(notice({ level: "debug" })));

    expect(decision).toBe("ignore");
    // The level was decidable from the notice, so the fetch is work that never needed doing.
    expect(mockAgentService.getSessionLogRecord).not.toHaveBeenCalled();
    expect(result.current.entries).toHaveLength(1);
  });

  it("invalidates the first page instead of guessing while a search is active", async () => {
    const { result } = mount({ search: "timeout" });
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    const decision = await act(async () => result.current.applyLiveNotice(notice()));

    expect(decision).toBe("invalidate");
    expect(result.current.firstPageInvalidated).toBe(true);
    // Nothing was inserted and nothing was dropped: the rows on screen are still correct, and the
    // view has said it cannot place this one among them.
    expect(mockAgentService.getSessionLogRecord).not.toHaveBeenCalled();
    expect(result.current.entries).toHaveLength(1);
  });

  it("clears the invalidation once the first page is reloaded", async () => {
    const { result } = mount({ search: "timeout" });
    await waitFor(() => expect(result.current.entries).toHaveLength(1));
    await act(async () => {
      await result.current.applyLiveNotice(notice());
    });
    expect(result.current.firstPageInvalidated).toBe(true);

    await act(async () => {
      await result.current.refresh();
    });

    expect(result.current.firstPageInvalidated).toBe(false);
  });

  it("does not insert the same row twice when a notice is delivered again", async () => {
    mockAgentService.getSessionLogRecord.mockResolvedValue(entry("log-live"));
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    await act(async () => {
      await result.current.applyLiveNotice(notice());
      await result.current.applyLiveNotice(notice());
    });

    // A bootstrap watermark and a live subscription overlap by design, so a repeated notice is the
    // ordinary case rather than a fault.
    expect(result.current.entries.filter((item) => item.id === "log-live")).toHaveLength(1);
  });

  it("invalidates when the announced row cannot be fetched", async () => {
    mockAgentService.getSessionLogRecord.mockRejectedValue(new Error("index unavailable"));
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    await act(async () => {
      await result.current.applyLiveNotice(notice());
    });

    // Saying so is the honest outcome. Dropping it silently would leave the list one row short with
    // nothing to explain the gap.
    expect(result.current.firstPageInvalidated).toBe(true);
    expect(result.current.entries).toHaveLength(1);
  });

  it("adds nothing when the announced row no longer exists", async () => {
    mockAgentService.getSessionLogRecord.mockResolvedValue(null);
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    await act(async () => {
      await result.current.applyLiveNotice(notice());
    });

    expect(result.current.entries).toHaveLength(1);
  });

  it("invalidates on a gap notice, because a hole changes what the page is missing", async () => {
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    const decision = await act(async () => result.current.applyLiveNotice(
      notice({ noticeKind: "gap", recordId: "", droppedCount: 2, sessionId: undefined }),
    ));

    expect(decision).toBe("invalidate");
    expect(result.current.firstPageInvalidated).toBe(true);
    expect(mockAgentService.getSessionLogRecord).not.toHaveBeenCalled();
  });

  it("keeps the rows on screen while a live insertion is being resolved", async () => {
    let release: ((value: SessionLogEntry) => void) | null = null;
    mockAgentService.getSessionLogRecord.mockReturnValue(
      new Promise<SessionLogEntry>((resolve) => { release = resolve; }),
    );
    const { result } = mount();
    await waitFor(() => expect(result.current.entries).toHaveLength(1));

    const pending = result.current.applyLiveNotice(notice());
    // A live update in flight must not blank the list. The rows already loaded are still correct,
    // whatever the fetch turns out to say.
    expect(result.current.entries).toHaveLength(1);

    await act(async () => {
      release?.(entry("log-live"));
      await pending;
    });
    expect(result.current.entries).toHaveLength(2);
  });
});
