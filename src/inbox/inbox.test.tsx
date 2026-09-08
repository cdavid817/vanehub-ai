// @vitest-environment jsdom

import { cleanup, fireEvent, screen, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import { resetWebMissionControlRunsForTest } from "../services/web-agent-client";
import { resetWebSystemActivityForTest, seedWebSystemActivityEventForTest } from "../services/web-system-activity-state";
import { Inbox } from "./inbox";
import { useInboxUnread } from "./use-inbox-badge";

vi.mock("../components/measured-virtual-list", () => ({
  MeasuredVirtualList: <T,>({ items, renderItem, testId }: { items: readonly T[]; renderItem: (item: T, index: number) => ReactNode; testId?: string }) => (
    <div data-testid={testId} role="list">{items.map(renderItem)}</div>
  ),
}));

function BadgeProbe() {
  const unread = useInboxUnread("sessions");
  return <output data-testid="badge-probe">{unread}</output>;
}

beforeEach(async () => {
  resetWebMissionControlRunsForTest();
  resetWebSystemActivityForTest();
  localStorage.clear();
  await activateAppLanguage("en");
});
afterEach(() => { cleanup(); vi.restoreAllMocks(); });

describe("Inbox", () => {
  it("composes Mission Control runs and unread warning activity into three sections", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);

    await waitFor(() => expect(screen.getByTestId("inbox-attention-count").textContent).not.toBe("0"));
    const attention = screen.getByTestId("inbox-sections").querySelector("#inbox-attention")!;
    // The web fixture carries waiting_approval, waiting_user, stuck, failed, and review runs.
    expect(attention.querySelectorAll("[data-testid^='mission-run-']").length).toBeGreaterThan(0);
    // Only the critical event is an attention row; the info event is counted but not listed.
    expect(attention.querySelectorAll("[data-testid='system-activity-item']")).toHaveLength(1);
    expect(attention.textContent).toContain("Breaker opened");
    expect(Number(screen.getByTestId("inbox-running-count").textContent)).toBeGreaterThan(0);
    expect(Number(screen.getByTestId("inbox-recent-count").textContent)).toBeGreaterThan(0);
    expect(document.querySelector("#inbox-attention")?.getAttribute("data-inbox-focused")).toBe("true");
    expect(screen.queryByTestId("inbox-empty")).toBeNull();
  });

  it("forwards row actions through the Mission Control navigation contract", async () => {
    const onNavigate = vi.fn();
    renderWithAppProviders(<Inbox onNavigate={onNavigate} onViewChange={vi.fn()} view="attention" />);
    // The failed run is both an attention run and a recently finished one, so it renders twice.
    const [review] = await screen.findAllByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294");
    fireEvent.click(review.querySelector("[data-action='review']")!);
    expect(onNavigate).toHaveBeenCalledWith(expect.objectContaining({ kind: "review" }));
  });

  it("switches to Board, renders the work board, persists the choice, and reports the view", { timeout: 20_000 }, async () => {
    const onViewChange = vi.fn();
    renderWithAppProviders(<Inbox onViewChange={onViewChange} view="attention" />);
    await screen.findByTestId("inbox-sections");
    expect(screen.getByTestId("inbox").getAttribute("data-inbox-mode")).toBe("list");

    fireEvent.click(screen.getByTestId("inbox-view-board"));
    expect(onViewChange).toHaveBeenCalledWith("board");
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("board");
    expect(screen.getByTestId("inbox").getAttribute("data-inbox-mode")).toBe("board");
    expect(screen.queryByTestId("inbox-sections")).toBeNull();
    await waitFor(() => expect(document.querySelector("#todo-board")).toBeTruthy(), { timeout: 15_000 });

    fireEvent.click(screen.getByTestId("inbox-view-list"));
    expect(onViewChange).toHaveBeenLastCalledWith("attention");
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("list");
    expect(screen.getByTestId("inbox-view-list").getAttribute("aria-pressed")).toBe("true");
  });

  it("honours a board route view and a persisted board preference", async () => {
    const { unmount } = renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="board" />);
    expect(screen.getByTestId("inbox").getAttribute("data-inbox-mode")).toBe("board");
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("board");
    unmount();
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    expect(screen.getByTestId("inbox").getAttribute("data-inbox-mode")).toBe("board");
  });

  it("opens the Activity log with the timeline's search and severity filter but no maintenance controls", { timeout: 20_000 }, async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    await screen.findByTestId("inbox-sections");
    const toggle = screen.getByRole("button", { name: "Activity log" });
    expect(toggle.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(toggle);
    expect(toggle.getAttribute("aria-expanded")).toBe("true");

    await waitFor(() => expect(screen.getByTestId("system-activity-view")).toBeTruthy(), { timeout: 15_000 });
    expect(screen.getByRole("textbox", { name: "Search events or safe identities" })).toBeTruthy();
    expect(screen.getByRole("combobox", { name: "Filter by severity" })).toBeTruthy();
    expect(screen.queryByTestId("system-activity-rebuild")).toBeNull();
    expect(screen.queryByTestId("system-activity-export")).toBeNull();
    expect(screen.queryByTestId("system-activity-health")).toBeNull();
  });

  it("opens the Mission Control console on the run that was inspected, and moves with the next one", { timeout: 20_000 }, async () => {
    const detail = vi.spyOn(agentService, "getMissionControlRun");
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="running" />);
    const [failed] = await screen.findAllByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a294");
    expect(screen.getByRole("button", { name: "Mission Control console" }).getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(failed.querySelector("button")!);
    expect(screen.getByRole("button", { name: "Mission Control console" }).getAttribute("aria-expanded")).toBe("true");
    await waitFor(() => expect(screen.getByTestId("mission-control")).toBeTruthy(), { timeout: 15_000 });
    await waitFor(() => expect(detail).toHaveBeenLastCalledWith("018f0f17-4d6a-7e20-b41d-66c5271a294"));
    await waitFor(() => expect(screen.getByTestId("mission-control").querySelector("[role='tablist']")).toBeTruthy());

    const [waitingUser] = await screen.findAllByTestId("mission-run-018f0f17-4d6a-7e20-b41d-66c5271a291");
    fireEvent.click(waitingUser.querySelector("button")!);
    await waitFor(() => expect(detail).toHaveBeenLastCalledWith("018f0f17-4d6a-7e20-b41d-66c5271a291"));
    expect(document.querySelector("#inbox-running")?.getAttribute("data-inbox-focused")).toBe("true");
  });

  it("shows a safe error when the overview fails and keeps activity", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockRejectedValue(new Error("token=secret"));
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "warning");
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    await waitFor(() => expect(document.querySelector("[aria-live='polite']")).toBeTruthy());
    expect(document.body.textContent).not.toContain("secret");
    await waitFor(() => expect(screen.getByTestId("inbox-attention-count").textContent).toBe("1"));
  });

  it("loads once in Board mode for the badge but does not poll until the list returns", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    const onBadgeChange = vi.fn();
    const { rerender } = renderWithAppProviders(<Inbox onBadgeChange={onBadgeChange} onViewChange={vi.fn()} view="board" />);
    await waitFor(() => expect(document.querySelector("#todo-board")).toBeTruthy(), { timeout: 15_000 });
    await waitFor(() => expect(onBadgeChange).toHaveBeenCalled());
    expect(onBadgeChange.mock.lastCall?.[0]).toBeGreaterThan(0);
    const afterFirstLoad = overview.mock.calls.length;
    fireEvent(window, new Event("focus"));
    await new Promise((resolve) => setTimeout(resolve, 50));
    expect(overview).toHaveBeenCalledTimes(afterFirstLoad);

    // The toggle asks the shell for the attention route; the shell answers with the new view.
    fireEvent.click(screen.getByTestId("inbox-view-list"));
    rerender(<Inbox onBadgeChange={onBadgeChange} onViewChange={vi.fn()} view="attention" />);
    await waitFor(() => expect(overview.mock.calls.length).toBeGreaterThan(afterFirstLoad));
    const afterListLoad = overview.mock.calls.length;
    fireEvent(window, new Event("focus"));
    await waitFor(() => expect(overview.mock.calls.length).toBeGreaterThan(afterListLoad));
  }, 20_000);

  it("keeps a slow load's result instead of discarding it for the next poll", async () => {
    const empty = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    const pending: Array<() => void> = [];
    const overview = vi.spyOn(agentService, "getMissionControlOverview")
      .mockImplementation(() => new Promise((resolve) => { pending.push(() => resolve(empty)); }));
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    await waitFor(() => expect(overview).toHaveBeenCalledTimes(1));
    // Polls that fire while the first load is still running do not start more requests.
    fireEvent(window, new Event("focus"));
    fireEvent(window, new Event("focus"));
    await new Promise((resolve) => setTimeout(resolve, 20));
    expect(overview).toHaveBeenCalledTimes(1);
    // When the slow response lands it is applied, and the one remembered poll runs afterwards.
    pending[0]();
    await screen.findByTestId("inbox-empty");
    await waitFor(() => expect(overview).toHaveBeenCalledTimes(2));
    pending[1]();
  });

  it("keeps the Board preference that a route asked for across a hidden render with a list view", async () => {
    const { rerender } = renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="board" />);
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("board");
    // The shell hands a hidden Inbox its default list view; that must not flip the preference.
    rerender(<Inbox active={false} onViewChange={vi.fn()} view="attention" />);
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("board");
    expect(screen.getByTestId("inbox").getAttribute("data-inbox-mode")).toBe("board");
  });

  it("keeps the empty state and a still refresh icon during background polls", async () => {
    const empty = { counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, attention: { items: [], nextCursor: null }, active: { items: [], nextCursor: null }, recent: { items: [], nextCursor: null } };
    const pending: { release: (() => void) | null } = { release: null };
    const overview = vi.spyOn(agentService, "getMissionControlOverview")
      .mockResolvedValueOnce(empty)
      .mockImplementation(() => new Promise((resolve) => { pending.release = () => resolve(empty); }));
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    await screen.findByTestId("inbox-empty");

    // A background poll is now pending; the empty state and the idle icon must not flicker.
    fireEvent(window, new Event("focus"));
    await waitFor(() => expect(overview).toHaveBeenCalledTimes(2));
    expect(screen.getByTestId("inbox-empty")).toBeTruthy();
    expect(screen.getByTestId("inbox-refresh").querySelector("svg")?.getAttribute("class")).not.toContain("animate-spin");

    // A refresh the user asked for does spin until it settles.
    fireEvent.click(screen.getByTestId("inbox-refresh"));
    await waitFor(() => expect(screen.getByTestId("inbox-refresh").querySelector("svg")?.getAttribute("class")).toContain("animate-spin"));
    pending.release?.();
    await waitFor(() => expect(screen.getByTestId("inbox-refresh").querySelector("svg")?.getAttribute("class")).not.toContain("animate-spin"));
  });

  it("opens one disclosure at a time", async () => {
    renderWithAppProviders(<Inbox onViewChange={vi.fn()} view="attention" />);
    await screen.findByTestId("inbox-sections");
    const consoleToggle = screen.getByRole("button", { name: "Mission Control console" });
    const logToggle = screen.getByRole("button", { name: "Activity log" });
    fireEvent.click(consoleToggle);
    expect(consoleToggle.getAttribute("aria-expanded")).toBe("true");
    fireEvent.click(logToggle);
    expect(logToggle.getAttribute("aria-expanded")).toBe("true");
    expect(consoleToggle.getAttribute("aria-expanded")).toBe("false");
    fireEvent.click(logToggle);
    expect(logToggle.getAttribute("aria-expanded")).toBe("false");
  });

  it("reports the badge it computed once the feed has loaded", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    const onBadgeChange = vi.fn();
    const overview = await agentService.getMissionControlOverview({ limit: 20, sort: "attention" });
    renderWithAppProviders(<Inbox onBadgeChange={onBadgeChange} onViewChange={vi.fn()} view="attention" />);
    await waitFor(() => expect(onBadgeChange).toHaveBeenCalledWith(overview.attention.items.length + 1));
    expect(onBadgeChange).not.toHaveBeenCalledWith(0);
  });

  it("feeds the navigation badge from attention runs plus unread activity", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    seedWebSystemActivityEventForTest("global", "global", "skill_created");
    const overview = await agentService.getMissionControlOverview({ limit: 20, sort: "attention" });
    renderWithAppProviders(<BadgeProbe />);
    await waitFor(() => expect(screen.getByTestId("badge-probe").textContent).toBe(String(overview.attention.items.length + 2)));
  });
});
