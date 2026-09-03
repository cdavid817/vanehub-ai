// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { WorkItem } from "../types/work-board";
import { WorkBoardBatchPanel, type WorkBoardBatchPanelProps } from "./work-board-batch-panel";

const item = (overrides: Partial<WorkItem> = {}): WorkItem => ({
  id: "a", title: "发布版本", description: "", stage: "inbox", priority: "none",
  rank: 1, projectPath: null, dueAt: null, archived: false,
  createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
  ...overrides,
});

function renderPanel(overrides: Partial<WorkBoardBatchPanelProps> = {}) {
  const props = {
    items: [item({ id: "a", stage: "inbox" }), item({ id: "b", stage: "inbox", archived: true }), item({ id: "c", stage: "done" })],
    onArchive: vi.fn(),
    onClearSelection: vi.fn(),
    onExit: vi.fn(),
    onMove: vi.fn(),
    onSelectAllVisible: vi.fn(),
    outcome: null,
    running: false,
    selectedIds: new Set(["a", "b", "c"]),
    ...overrides,
  };
  const utils = render(<WorkBoardBatchPanel {...props} />);
  return { ...utils, props };
}

describe("WorkBoardBatchPanel", () => {
  it("shows the selected count and a live eligibility preview for archive", () => {
    renderPanel();
    expect(screen.getByText("已选 3 项")).toBeTruthy();
    // a (inbox) and c (done) can be archived; b is already archived -- 2 of 3.
    expect(screen.getByText("3 项中 2 项可执行")).toBeTruthy();
  });

  it("recomputes the move eligibility preview when the target stage changes", () => {
    renderPanel();
    // Default target is the first stage, inbox: only c (done) is eligible to move there; a is
    // already in inbox and b is archived -- 1 of 3.
    expect(screen.getByRole("button", { name: "移动到收件箱" })).toBeTruthy();
    expect(screen.getAllByText("3 项中 1 项可执行")).toHaveLength(1);

    fireEvent.change(screen.getByLabelText("移动到"), { target: { value: "done" } });
    // Moving to "done": only a (inbox) is eligible; b is archived, c is already done.
    expect(screen.getByRole("button", { name: "移动到已完成" })).toBeTruthy();
  });

  it("disables Archive when nothing selected is eligible", () => {
    renderPanel({ selectedIds: new Set(["b"]) });
    expect((screen.getByRole("button", { name: "归档所选" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("enables Archive once at least one selected item is eligible, and calls onArchive", () => {
    const { props } = renderPanel({ selectedIds: new Set(["a"]) });
    const button = screen.getByRole("button", { name: "归档所选" }) as HTMLButtonElement;
    expect(button.disabled).toBe(false);
    fireEvent.click(button);
    expect(props.onArchive).toHaveBeenCalledOnce();
  });

  it("calls onMove with the currently chosen target stage", () => {
    const { props } = renderPanel({ selectedIds: new Set(["c"]) });
    fireEvent.click(screen.getByRole("button", { name: "移动到收件箱" }));
    expect(props.onMove).toHaveBeenCalledWith("inbox");
  });

  it("disables both actions while a batch run is in flight", () => {
    renderPanel({ running: true });
    expect((screen.getByRole("button", { name: "归档所选" }) as HTMLButtonElement).disabled).toBe(true);
    expect((screen.getByRole("button", { name: "移动到收件箱" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("wires Select all visible, Clear selection, and Exit to their own handlers", () => {
    const { props } = renderPanel();
    fireEvent.click(screen.getByRole("button", { name: "全选当前" }));
    expect(props.onSelectAllVisible).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "取消选择" }));
    expect(props.onClearSelection).toHaveBeenCalledOnce();
    fireEvent.click(screen.getByRole("button", { name: "退出" }));
    expect(props.onExit).toHaveBeenCalledOnce();
  });

  it("renders a per-item outcome list after a run, distinguishing success, error, and skipped", () => {
    renderPanel({
      outcome: [
        { id: "a", title: "发布版本", result: "success" },
        { id: "c", title: "另一项", result: "error" },
        { id: "b", title: "已归档项", result: "skipped" },
      ],
    });
    const list = screen.getByRole("list", { name: "结果" });
    expect(within(list).getByText("成功")).toBeTruthy();
    expect(within(list).getByText("失败")).toBeTruthy();
    expect(within(list).getByText("已跳过(不符合条件)")).toBeTruthy();
  });

  it("renders no outcome list before any run has happened", () => {
    renderPanel({ outcome: null });
    expect(screen.queryByRole("list", { name: "结果" })).toBeNull();
  });
});
