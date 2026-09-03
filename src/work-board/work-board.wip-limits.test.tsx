// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { CreateWorkItemInput, WorkItem, WorkItemFilters } from "../types/work-board";

const mocks = vi.hoisted(() => ({
  create: vi.fn(),
  items: [] as WorkItem[],
  list: vi.fn(),
}));

vi.mock("../hooks/use-media-query", () => ({ useMediaQuery: () => false }));
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: {
    archiveWorkItem: vi.fn(),
    createWorkItem: mocks.create,
    deleteWorkItem: vi.fn(),
    linkWorkItemSource: vi.fn(),
    listWorkItems: mocks.list,
    moveWorkItem: vi.fn(),
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
  mocks.items = [fixture()];
  mocks.list.mockReset().mockImplementation(async ({ archived = false }: WorkItemFilters = {}) => mocks.items.filter((entry) => entry.archived === archived));
  mocks.create.mockReset().mockImplementation(async (input: CreateWorkItemInput) => {
    const created: WorkItem = {
      id: "work-new", title: input.title, description: input.description ?? "",
      stage: input.stage ?? "inbox", priority: input.priority ?? "none", rank: 1_000,
      projectPath: input.projectPath ?? null, dueAt: input.dueAt ?? null, archived: false,
      createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
    };
    mocks.items.push(created);
    return created;
  });
});

function setInboxLimit(limit: string) {
  fireEvent.click(screen.getByRole("button", { name: "WIP 上限" }));
  fireEvent.change(screen.getByLabelText("收件箱上限"), { target: { value: limit } });
  fireEvent.click(screen.getByRole("button", { name: "保存" }));
}

describe("WorkBoard WIP limits (14.14)", () => {
  it("shows no over-limit badge with no limit configured", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    expect(screen.queryByText(/超出上限/)).toBeNull();
  });

  it("shows a presentation-only over-limit badge once a configured per-stage limit is exceeded, and persists it", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");

    setInboxLimit("1");
    // Exactly at the limit: no badge yet.
    expect(screen.queryByText(/超出上限/)).toBeNull();

    fireEvent.click(screen.getByRole("button", { name: "新建工作项" }));
    fireEvent.change(screen.getByLabelText("标题"), { target: { value: "第二项" } });
    fireEvent.click(screen.getByRole("button", { name: "创建" }));
    await screen.findByTestId("work-item-work-new");

    // Now over the configured limit of 1 (2 inbox items) -- a clearly-labeled soft indicator, not
    // a blocking state: the create above already succeeded past the limit with no error, no
    // confirmation, and no disabled control anywhere.
    expect(screen.getByText("超出上限(2/1)")).toBeTruthy();
    expect(mocks.create).toHaveBeenCalledOnce();
    expect(screen.queryByRole("alert")).toBeNull();

    expect(JSON.parse(localStorage.getItem("vanehub.work-board.wip-limits.v1") ?? "{}")).toEqual({ version: 1, limits: { inbox: 1 } });
  });

  it("states plainly in the menu that the limit is optional and non-blocking", () => {
    render(<WorkBoard />);
    fireEvent.click(screen.getByRole("button", { name: "WIP 上限" }));
    expect(screen.getByText(/超出上限不会阻止移动或创建工作项/)).toBeTruthy();
  });

  it("clearing a limit back to blank removes the badge even while still over the old threshold", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    setInboxLimit("1");

    fireEvent.click(screen.getByRole("button", { name: "新建工作项" }));
    fireEvent.change(screen.getByLabelText("标题"), { target: { value: "第二项" } });
    fireEvent.click(screen.getByRole("button", { name: "创建" }));
    await screen.findByTestId("work-item-work-new");
    await waitFor(() => expect(screen.queryByText(/超出上限/)).toBeTruthy());

    setInboxLimit("");
    expect(screen.queryByText(/超出上限/)).toBeNull();
  });
});
