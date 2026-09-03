// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { MoveWorkItemInput, WorkItem, WorkItemFilters } from "../types/work-board";
import { readWorkBoardSavedViews } from "./work-board-saved-views";

const mocks = vi.hoisted(() => ({
  archive: vi.fn(),
  items: [] as WorkItem[],
  list: vi.fn(),
  move: vi.fn(),
}));

vi.mock("../hooks/use-media-query", () => ({ useMediaQuery: () => false }));
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: {
    archiveWorkItem: mocks.archive,
    createWorkItem: vi.fn(),
    deleteWorkItem: vi.fn(),
    linkWorkItemSource: vi.fn(),
    listWorkItems: mocks.list,
    moveWorkItem: mocks.move,
    restoreWorkItem: vi.fn(),
    updateWorkItem: vi.fn(),
  },
}));

import { WorkBoard } from "./work-board";

const fixture = (overrides: Partial<WorkItem> = {}): WorkItem => ({
  id: "work-1", title: "任务一", description: "", stage: "inbox", priority: "none",
  rank: 1_000, projectPath: null, dueAt: null, archived: false,
  createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
  ...overrides,
});

beforeEach(() => {
  localStorage.clear();
  mocks.items = [
    fixture({ id: "work-1", title: "任务一", stage: "inbox" }),
    fixture({ id: "work-2", title: "任务二", stage: "done" }),
  ];
  mocks.list.mockReset().mockImplementation(async ({ archived = false }: WorkItemFilters = {}) => mocks.items.filter((entry) => entry.archived === archived));
  mocks.move.mockReset().mockImplementation(async ({ workItemId, stage }: MoveWorkItemInput) => {
    const entry = mocks.items.find((candidate) => candidate.id === workItemId);
    if (entry) entry.stage = stage;
    return entry;
  });
  mocks.archive.mockReset().mockImplementation(async (id: string) => {
    const entry = mocks.items.find((candidate) => candidate.id === id);
    if (entry) entry.archived = true;
    return entry;
  });
});

function enterBatchMode() {
  fireEvent.click(screen.getByRole("button", { name: "批量" }));
}

function checkbox(testId: string) {
  return within(screen.getByTestId(testId)).getByRole("checkbox", { name: "选择工作项" }) as HTMLInputElement;
}

describe("WorkBoard batch mode (14.12)", () => {
  it("enters batch mode from the toolbar, shows a per-card checkbox, and tracks the selected count", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    expect(screen.queryByRole("checkbox", { name: "选择工作项" })).toBeNull();

    enterBatchMode();
    expect(screen.getByRole("region", { name: "批量" })).toBeTruthy();
    expect(screen.getByText("已选 0 项")).toBeTruthy();

    fireEvent.click(checkbox("work-item-work-1"));
    expect(screen.getByText("已选 1 项")).toBeTruthy();
    fireEvent.click(checkbox("work-item-work-2"));
    expect(screen.getByText("已选 2 项")).toBeTruthy();
  });

  it("shows a live eligibility preview that excludes an item already in the chosen target stage", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    enterBatchMode();
    fireEvent.click(checkbox("work-item-work-1")); // inbox
    fireEvent.click(checkbox("work-item-work-2")); // done

    // Default move target is the first stage, inbox: work-1 is already there, only work-2 (done) is eligible.
    expect(screen.getByText("2 项中 1 项可执行")).toBeTruthy();

    fireEvent.change(screen.getByLabelText("移动到"), { target: { value: "done" } });
    // Now the reverse: work-2 is already in done, only work-1 is eligible.
    expect(screen.getByText("2 项中 1 项可执行")).toBeTruthy();
  });

  it("exiting batch mode clears the selection", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    enterBatchMode();
    fireEvent.click(checkbox("work-item-work-1"));
    expect(screen.getByText("已选 1 项")).toBeTruthy();

    // Two "退出" buttons coexist once batch mode is active: the toolbar's own toggle (which
    // relabels itself once active) and the panel's own Exit button -- scope to the panel's.
    fireEvent.click(within(screen.getByRole("region", { name: "批量" })).getByRole("button", { name: "退出" }));
    expect(screen.queryByRole("region", { name: "批量" })).toBeNull();

    enterBatchMode();
    expect(screen.getByText("已选 0 项")).toBeTruthy();
  });

  // 14.16 mutation race: two cards' own optimistic updates and rollbacks race concurrently within
  // one batch run, each independently reconciled through the exact per-card mutation registry
  // 14.10 already established -- not a second, batch-specific mutation mechanism.
  it("runs a batch archive concurrently, reporting per-item success and failure, and rolls the failed card back with its own visible error", async () => {
    let rejectWork2: (reason: unknown) => void = () => {};
    mocks.archive.mockReset().mockImplementation(async (id: string) => {
      if (id === "work-2") return new Promise((_resolve, reject) => { rejectWork2 = reject; });
      const entry = mocks.items.find((candidate) => candidate.id === id);
      if (entry) entry.archived = true;
      return entry;
    });

    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    enterBatchMode();
    fireEvent.click(checkbox("work-item-work-1"));
    fireEvent.click(checkbox("work-item-work-2"));

    fireEvent.click(screen.getByRole("button", { name: "归档所选" }));

    // Both cards leave the active view immediately -- the optimistic `archived: true` patch
    // applies to both before either server call settles, and the active-scope filter (14.10's own
    // `mutateCard`) hides an archived card from this view the instant that patch lands, not only
    // once the request resolves.
    await waitFor(() => expect(screen.queryByTestId("work-item-work-1")).toBeNull());
    await waitFor(() => expect(screen.queryByTestId("work-item-work-2")).toBeNull());

    rejectWork2(new Error("archive failed"));

    // work-1's own archive genuinely succeeded and stays gone; work-2's rejection rolls its own
    // card back to `archived: false`, which reintroduces it into this same active-scope view with
    // its own visible, dismissible error -- proving rollback is per-card even when two cards'
    // mutations were in flight at once.
    await waitFor(() => expect(screen.getByTestId("work-item-work-2")).toBeTruthy());
    expect(within(screen.getByTestId("work-item-work-2")).getByRole("alert")).toBeTruthy();
    expect(screen.queryByTestId("work-item-work-1")).toBeNull();

    const outcome = screen.getByRole("list", { name: "结果" });
    expect(within(outcome).getByText("成功")).toBeTruthy();
    expect(within(outcome).getByText("失败")).toBeTruthy();
    // work-2 itself never actually archived server-side -- rollback restored `archived: false`.
    expect(mocks.items.find((entry) => entry.id === "work-2")?.archived).toBe(false);
  });

  it("does not corrupt a previously saved view when a batch action runs", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");

    fireEvent.click(screen.getByRole("button", { name: "已保存视图" }));
    fireEvent.change(screen.getByLabelText("视图名称"), { target: { value: "我的视图" } });
    fireEvent.click(screen.getByRole("button", { name: "保存当前筛选" }));
    const savedBefore = readWorkBoardSavedViews();
    expect(savedBefore).toHaveLength(1);

    enterBatchMode();
    fireEvent.click(checkbox("work-item-work-1"));
    fireEvent.click(screen.getByRole("button", { name: "归档所选" }));
    await waitFor(() => expect(screen.queryByTestId("work-item-work-1")).toBeNull());

    expect(readWorkBoardSavedViews()).toEqual(savedBefore);
  });

  it("is fully keyboard-operable: tab to the batch toggle, check a card, and run Archive without a mouse", async () => {
    const user = userEvent.setup();
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");

    screen.getByRole("button", { name: "批量" }).focus();
    await user.keyboard("{Enter}");
    expect(screen.getByRole("region", { name: "批量" })).toBeTruthy();

    checkbox("work-item-work-1").focus();
    await user.keyboard(" ");
    expect(screen.getByText("已选 1 项")).toBeTruthy();

    screen.getByRole("button", { name: "归档所选" }).focus();
    await user.keyboard("{Enter}");
    await waitFor(() => expect(screen.queryByTestId("work-item-work-1")).toBeNull());
  });
});
