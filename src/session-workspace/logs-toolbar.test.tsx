// @vitest-environment jsdom

import { Fragment } from "react";
import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type {
  SessionLogCoverageState,
  SessionLogEntry,
  SessionLogPage,
} from "../types/session-workspace";

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    listSessionLogs: vi.fn(),
    exportSessionLogs: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: mockAgentService }));
vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => unknown; testId?: string }) => (
    <div data-testid={testId}>
      {items.map((item, index) => <Fragment key={index}>{renderItem(item, index) as never}</Fragment>)}
    </div>
  ),
}));

import { LogsTab } from "./logs-tab";

function entry(id: string): SessionLogEntry {
  return {
    id,
    timestamp: "2026-08-25T10:00:00.000Z",
    level: "info",
    category: "session.runtime",
    message: `message ${id}`,
    context: {},
  };
}

function page(state?: SessionLogCoverageState, droppedCount = 0): SessionLogPage {
  return {
    items: [entry("log-1")],
    truncated: false,
    nextCursor: null,
    coverage: state
      ? { state, droppedCount, truncated: false, reasonCodes: [] }
      : undefined,
  };
}

describe("Logs toolbar", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.listSessionLogs.mockResolvedValue(page("complete"));
  });

  it("offers Pause while following and Follow once paused", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    const pause = screen.getByRole("button", { name: "Pause" });
    expect(pause.getAttribute("aria-pressed")).toBe("false");

    fireEvent.click(pause);

    const follow = screen.getByRole("button", { name: "Follow" });
    expect(follow.getAttribute("aria-pressed")).toBe("true");
  });

  it("offers Jump to latest only once the view is no longer following", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    // Following: the button would scroll to where the reader already is.
    expect(screen.queryByRole("button", { name: /Jump to latest/ })).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    expect(screen.getByRole("button", { name: "Jump to latest" })).toBeDefined();
  });

  it("returns to following when Jump to latest is pressed", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");
    fireEvent.click(screen.getByRole("button", { name: "Pause" }));

    fireEvent.click(screen.getByRole("button", { name: "Jump to latest" }));

    expect(screen.getByRole("button", { name: "Pause" })).toBeDefined();
    expect(screen.queryByRole("button", { name: /Jump to latest/ })).toBeNull();
  });

  it("says nothing about coverage when the index reports complete", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    // A banner on every page would train the reader to ignore it, and `complete` is precisely the
    // case with nothing to warn about.
    expect(screen.queryByText(/log index/i)).toBeNull();
  });

  it("says the list is not final while the index is still catching up", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page("indexing"));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    await screen.findByText("The log index is still catching up, so this list is not final yet.");
  });

  it("says an empty result is not a conclusion when coverage is partial", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page("partial", 3));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    const notice = await screen.findByRole("status");
    expect(notice.textContent).toContain("Some records are known to be missing");
    // The count turns "something is missing" into "this much is missing", which is the difference
    // between a caveat a reader can act on and one they can only worry about.
    expect(notice.textContent).toContain("3");
  });

  it("treats a page that reported no coverage as unavailable rather than complete", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page(undefined));
    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    // A runtime that did not report is not a runtime that reported everything. Reading absence as
    // `complete` is the one default that lets a reader conclude something from an empty list.
    await screen.findByText("The log index cannot answer right now, so this list may be incomplete.");
  });
});

describe("Logs scope chips", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.listSessionLogs.mockResolvedValue(page("complete"));
  });

  it("shows the correlations narrowing the list, which were chosen on another panel", async () => {
    renderWithAppProviders(
      <LogsTab correlation={{ runId: "run-42", traceId: "trace-7" }} sessionId="session-1" />,
    );
    await screen.findByText("message log-1");

    const chips = screen.getByRole("group", { name: "Active log filters" });
    expect(chips.textContent).toContain("Run");
    expect(chips.textContent).toContain("run-42");
    expect(chips.textContent).toContain("Trace");
    expect(chips.textContent).toContain("trace-7");
  });

  it("shows no chips when nothing narrows the list", async () => {
    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("message log-1");

    expect(screen.queryByRole("group", { name: "Active log filters" })).toBeNull();
  });

  it("drops one correlation from the query when its chip is cleared", async () => {
    renderWithAppProviders(
      <LogsTab correlation={{ runId: "run-42", traceId: "trace-7" }} sessionId="session-1" />,
    );
    await screen.findByText("message log-1");
    const chips = screen.getByRole("group", { name: "Active log filters" });

    fireEvent.click(chips.querySelectorAll("button")[0]);

    const last = mockAgentService.listSessionLogs.mock.calls.at(-1)?.[0];
    expect(last.runId).toBeNull();
    // Clearing one leaves the others: a chip is a single filter, not the whole scope.
    expect(last.traceId).toBe("trace-7");
  });

  it("carries every correlation into the query the index narrows by", async () => {
    renderWithAppProviders(
      <LogsTab
        correlation={{
          agentId: "agent-1",
          operationId: "operation-1",
          runId: "run-1",
          spanId: "span-1",
          traceId: "trace-1",
        }}
        seatId="seat-1"
        sessionId="session-1"
      />,
    );
    await screen.findByText("message log-1");

    const first = mockAgentService.listSessionLogs.mock.calls[0][0];
    expect(first.seatId).toBe("seat-1");
    expect(first.runId).toBe("run-1");
    expect(first.traceId).toBe("trace-1");
    expect(first.spanId).toBe("span-1");
    expect(first.operationId).toBe("operation-1");
    expect(first.agentId).toBe("agent-1");
  });

  it("exports through the same scope the list is showing", async () => {
    mockAgentService.exportSessionLogs.mockResolvedValue({ status: "cancelled", path: null });
    renderWithAppProviders(<LogsTab correlation={{ runId: "run-42" }} sessionId="session-1" />);
    await screen.findByText("message log-1");

    fireEvent.click(screen.getByRole("button", { name: "Export" }));

    // An export wider than the list would hand the user a file that does not match what they were
    // looking at when they asked for it.
    expect(mockAgentService.exportSessionLogs).toHaveBeenCalledWith(
      expect.objectContaining({ runId: "run-42", sessionId: "session-1" }),
    );
  });
});
