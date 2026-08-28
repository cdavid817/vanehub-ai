// @vitest-environment jsdom

import { Fragment } from "react";
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { SessionLogEntry, SessionLogPage } from "../types/session-workspace";

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

function entry(id: string, message: string): SessionLogEntry {
  return {
    id,
    timestamp: "2026-08-22T10:00:00.000Z",
    level: "info",
    category: "session.runtime",
    message,
    context: {},
  };
}

function page(items: SessionLogEntry[], truncated: boolean, nextCursor: string | null): SessionLogPage {
  return { items, truncated, nextCursor };
}

describe("LogsTab", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  // The whole list used to be replaced by an error panel when a later page failed, discarding the
  // rows the user was reading.
  it("keeps loaded entries visible when loading the next page fails", async () => {
    mockAgentService.listSessionLogs
      .mockResolvedValueOnce(page([entry("log-1", "first page entry")], true, "cursor-1"))
      .mockRejectedValueOnce(new Error("Storage unavailable"));

    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("first page entry");

    fireEvent.click(screen.getByRole("button", { name: "Load more" }));

    await screen.findByText("The next page of logs could not be loaded.");
    expect(screen.getByText("first page entry")).toBeDefined();
    // The continuation boundary is intact, so the failed page can be retried from the same cursor.
    expect(screen.getByRole("button", { name: "Retry" })).toBeDefined();
  });

  it("retries the failed page from the original cursor", async () => {
    mockAgentService.listSessionLogs
      .mockResolvedValueOnce(page([entry("log-1", "first page entry")], true, "cursor-1"))
      .mockRejectedValueOnce(new Error("Storage unavailable"))
      .mockResolvedValueOnce(page([entry("log-2", "second page entry")], false, null));

    renderWithAppProviders(<LogsTab sessionId="session-1" />);
    await screen.findByText("first page entry");
    fireEvent.click(screen.getByRole("button", { name: "Load more" }));
    await screen.findByRole("button", { name: "Retry" });

    fireEvent.click(screen.getByRole("button", { name: "Retry" }));

    await screen.findByText("second page entry");
    expect(screen.getByText("first page entry")).toBeDefined();
    expect(mockAgentService.listSessionLogs).toHaveBeenLastCalledWith(
      expect.objectContaining({ cursor: "cursor-1" }),
    );
  });

  it("shows a blocking error with Retry when nothing has loaded yet", async () => {
    mockAgentService.listSessionLogs.mockRejectedValue(new Error("Storage unavailable"));

    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    await screen.findByRole("button", { name: "Retry" });
    expect(screen.queryByRole("button", { name: "Load more" })).toBeNull();
  });

  // The seat switcher used to be rendered beside Logs without reaching this query at all.
  it("carries the selected seat into the log query", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page([], false, null));

    renderWithAppProviders(<LogsTab seatId="seat-builder" sessionId="session-1" />);

    await waitFor(() => {
      expect(mockAgentService.listSessionLogs).toHaveBeenCalledWith(
        expect.objectContaining({ seatId: "seat-builder", sessionId: "session-1" }),
      );
    });
  });

  it("requeries when the selected seat changes and leaves the previous seat's rows behind", async () => {
    mockAgentService.listSessionLogs
      .mockResolvedValueOnce(page([entry("log-1", "planner entry")], false, null))
      .mockResolvedValueOnce(page([entry("log-2", "builder entry")], false, null));

    const { rerender } = renderWithAppProviders(<LogsTab seatId="seat-planner" sessionId="session-1" />);
    await screen.findByText("planner entry");

    rerender(<LogsTab seatId="seat-builder" sessionId="session-1" />);

    await screen.findByText("builder entry");
    expect(screen.queryByText("planner entry")).toBeNull();
  });

  it("sends no seat when every seat is selected", async () => {
    mockAgentService.listSessionLogs.mockResolvedValue(page([], false, null));

    renderWithAppProviders(<LogsTab sessionId="session-1" />);

    await waitFor(() => {
      expect(mockAgentService.listSessionLogs).toHaveBeenCalledWith(
        expect.objectContaining({ seatId: null }),
      );
    });
  });
});
