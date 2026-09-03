// @vitest-environment jsdom

import { Fragment } from "react";
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { SessionLogNotice } from "../types/session-log-notice";
import type {
  SessionLogCoverage,
  SessionLogCoverageState,
  SessionLogEntry,
  SessionLogPage,
} from "../types/session-workspace";

const { mockAgentService, mockStream } = vi.hoisted(() => {
  const listeners = new Set<(notice: unknown) => void>();
  return {
    mockAgentService: {
      listSessionLogs: vi.fn(),
      getSessionLogRecord: vi.fn(),
      exportSessionLogs: vi.fn(),
    },
    mockStream: {
      listeners,
      subscribe: vi.fn((_input: unknown, listener: (notice: unknown) => void) => {
        listeners.add(listener);
        return () => listeners.delete(listener);
      }),
      publish(notice: unknown) {
        for (const listener of [...listeners]) listener(notice);
      },
    },
  };
});

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));
vi.mock("../services/runtime-session-log-client", () => ({ sessionLogNoticeStream: mockStream }));
vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>
      {items.map((item, index) => <Fragment key={index}>{renderItem(item, index) as never}</Fragment>)}
    </div>
  ),
}));

import { LogsTab } from "./logs-tab";

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

function coverage(state: SessionLogCoverageState, overrides: Partial<SessionLogCoverage> = {}): SessionLogCoverage {
  return { state, droppedCount: 0, truncated: false, reasonCodes: [], ...overrides };
}

function page(
  items: SessionLogEntry[],
  options: { truncated?: boolean; nextCursor?: string | null; coverage?: SessionLogCoverage } = {},
): SessionLogPage {
  return {
    items,
    truncated: options.truncated ?? false,
    nextCursor: options.nextCursor ?? null,
    coverage: options.coverage ?? coverage("complete"),
  };
}

function notice(overrides: Partial<SessionLogNotice> = {}): SessionLogNotice {
  return {
    noticeKind: "appended",
    recordId: "log-live",
    sequence: 99,
    occurredAt: "2026-08-25T10:00:09.000Z",
    level: "info",
    coverageState: "complete",
    sessionId: "session-1",
    ...overrides,
  };
}

async function publish(value: SessionLogNotice) {
  await act(async () => {
    mockStream.publish(value);
    // Let the fetch-by-id promise settle before the assertions look at the list.
    await Promise.resolve();
    await Promise.resolve();
  });
}

/**
 * The situations the Logs tab has to survive that are not "one page loaded successfully".
 *
 * Each one is a way the corpus, the index or the connection can change underneath a list somebody
 * is reading, and each has a different honest answer. They are together in one file because the
 * point is that they are *distinguishable* — a view that responded to all eight the same way would
 * be right about at most one.
 */
describe("Logs tab scenarios", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockStream.listeners.clear();
    mockAgentService.listSessionLogs.mockResolvedValue(page([entry("log-1")]));
  });

  it("inserts a live row that arrived while a later page was still loading", async () => {
    let releasePage: ((value: SessionLogPage) => void) | null = null;
    mockAgentService.listSessionLogs
      .mockResolvedValueOnce(page([entry("log-1")], { truncated: true, nextCursor: "cursor-1" }))
      .mockReturnValueOnce(new Promise<SessionLogPage>((resolve) => { releasePage = resolve; }));
    mockAgentService.getSessionLogRecord.mockResolvedValue(entry("log-live", "arrived mid-page"));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    fireEvent.click(screen.getByRole("button", { name: "Load more" }));
    await publish(notice());

    // The live row lands at the newest edge while an older page is still in flight. The two touch
    // different ends of the list, so neither has to wait for the other.
    await screen.findByText("arrived mid-page");
    await act(async () => {
      releasePage?.(page([entry("log-2")]));
      await Promise.resolve();
    });
    await screen.findByText("message log-2");
    expect(screen.getByText("arrived mid-page")).toBeDefined();
  });

  it("keeps the rows and offers a retry when a cursor is refused", async () => {
    mockAgentService.listSessionLogs
      .mockResolvedValueOnce(page([entry("log-1")], { truncated: true, nextCursor: "cursor-1" }))
      .mockRejectedValueOnce(new Error("validation error: log_cursor_filter_mismatch"));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    fireEvent.click(screen.getByRole("button", { name: "Load more" }));

    // A refused cursor is not a reason to discard what is already on screen, and not a reason to
    // silently restart from the newest page either — that would look like an ordinary refresh.
    await screen.findByText("The next page of logs could not be loaded.");
    expect(screen.getByText("message log-1")).toBeDefined();
    expect(screen.getByRole("button", { name: "Retry" })).toBeDefined();
  });

  it("says the list is not final while a repair is running", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(
      page([entry("log-1")], { coverage: coverage("indexing") }),
    );
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    await screen.findByText("The log index is still catching up, so this list is not final yet.");
    // The rows are real, so they stay: `indexing` is a caveat about the set, not about the rows.
    expect(screen.getByText("message log-1")).toBeDefined();
  });

  it("says nothing new after a rotation, because a rotation changes nothing a reader can see", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page([entry("log-1")]));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    // A rotated file holds the same records under a new name. The index keeps its identity and its
    // checkpoint, so coverage stays complete and the list is unchanged — and a banner here would be
    // noise that trains the reader to ignore the ones that matter.
    expect(screen.queryByText(/log index/i)).toBeNull();
  });

  it("reports partial coverage after a directory change rather than an empty session", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(
      page([], { coverage: coverage("partial", { reasonCodes: ["log_retention_expired"] }) }),
    );
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    // The corpus was replaced, so there is nothing to show — and the difference between "this
    // session logged nothing" and "the logs this session wrote are no longer here" is the entire
    // reason coverage exists.
    await screen.findByText(/Some records are known to be missing/);
    expect(screen.getByText("No matching session logs were found.")).toBeDefined();
  });

  it("reports retention as a reason the list starts where it does", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(
      page([entry("log-1")], {
        coverage: coverage("partial", {
          droppedCount: 12,
          oldestAvailableAt: "2026-08-20T00:00:00.000Z",
          reasonCodes: ["log_retention_expired"],
        }),
      }),
    );
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    const notice = await screen.findByRole("status");
    expect(notice.textContent).toContain("Some records are known to be missing");
    expect(notice.textContent).toContain("12");
  });

  it("invalidates the first page when a gap notice arrives", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    await publish(notice({ noticeKind: "gap", recordId: "", droppedCount: 4, sessionId: undefined }));

    await screen.findByText("New log entries arrived that the current filters cannot be applied to.");
    // Nothing was fetched and nothing was invented: a gap names records that were never read.
    expect(mockAgentService.getSessionLogRecord).not.toHaveBeenCalled();
    expect(screen.getByText("message log-1")).toBeDefined();
  });

  it("clears the invalidation when the reader refreshes", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");
    await publish(notice({ noticeKind: "gap", recordId: "", sessionId: undefined }));
    await screen.findByRole("button", { name: "Refresh" });

    fireEvent.click(screen.getByRole("button", { name: "Refresh" }));

    await waitFor(() => {
      expect(screen.queryByText("New log entries arrived that the current filters cannot be applied to.")).toBeNull();
    });
  });

  it("exports through the same filters the list is showing", async () => {
    mockAgentService.exportSessionLogs.mockResolvedValue({ status: "cancelled", path: null });
    renderWithAppProviders(<LogsTab correlation={{ runId: "run-9" }} sessionId="session-1" />);
    await screen.findByText("message log-1");
    fireEvent.click(screen.getByRole("button", { name: "Error" }));

    fireEvent.click(screen.getByRole("button", { name: "Export" }));

    // The file has to match the list the reader was looking at when they asked for it. An export
    // wider or narrower than the visible filters is one they cannot check without reading it all.
    const exported = mockAgentService.exportSessionLogs.mock.calls.at(-1)?.[0];
    expect(exported.runId).toBe("run-9");
    expect(exported.levels).not.toContain("error");
    expect(exported.sessionId).toBe("session-1");
  });

  it("offers a jump only for rows it actually withheld", async () => {
    mockAgentService.getSessionLogRecord.mockResolvedValue(entry("log-live"));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    await publish(notice({ level: "debug", recordId: "log-live" }));
    await publish(notice({ level: "debug", recordId: "log-live-2" }));

    // Both were inserted while paused, so the offer names two. A notice that had been ignored or
    // that invalidated the page would not be a row waiting above the viewport.
    await screen.findByRole("button", { name: /Jump to latest \(2 new\)/ });
  });

  it("does not count a notice it ignored as a row waiting above", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");
    fireEvent.click(screen.getByRole("button", { name: "Error" }));
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    await publish(notice({ level: "error" }));

    // `error` was just switched off, so the row is out of scope. Offering a jump to it would send
    // the reader to a place with nothing new in it.
    expect(screen.getByRole("button", { name: "Jump to latest" })).toBeDefined();
    expect(mockAgentService.getSessionLogRecord).not.toHaveBeenCalled();
  });

  it("stops listening when the panel is hidden behind another tab", async () => {
    const { rerender } = renderWithAppProviders(<LogsTab isVisible sessionId="session-1" />);
    await screen.findByText("message log-1");
    expect(mockStream.listeners.size).toBe(1);

    rerender(<LogsTab isVisible={false} sessionId="session-1" />);

    // A hidden panel does not read logs, and a subscription it cannot render into is one that only
    // costs work.
    expect(mockStream.listeners.size).toBe(0);
  });
});
