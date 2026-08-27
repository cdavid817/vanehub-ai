// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { MoveWorkItemInput, UpdateWorkItemInput, WorkItem, WorkItemFilters } from "../types/work-board";

const mocks = vi.hoisted(() => ({
  archive: vi.fn(),
  compact: false,
  items: [] as WorkItem[],
  list: vi.fn(),
  move: vi.fn(),
  update: vi.fn(),
}));

vi.mock("../hooks/use-media-query", () => ({ useMediaQuery: () => mocks.compact }));
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: {
    archiveWorkItem: mocks.archive,
    createWorkItem: vi.fn(),
    deleteWorkItem: vi.fn(),
    linkWorkItemSource: vi.fn(),
    listWorkItems: mocks.list,
    moveWorkItem: mocks.move,
    restoreWorkItem: vi.fn(),
    updateWorkItem: mocks.update,
  },
}));

import { WorkBoard } from "./work-board";

const fixture = (): WorkItem => ({
  id: "work-1", title: "发布版本", description: "检查构建", stage: "inbox",
  priority: "high", rank: 1_000, projectPath: "D:/app", dueAt: null,
  archived: false, createdAt: "2026-01-01", updatedAt: "2026-01-01",
  sources: [
    { sourceKind: "session", sourceId: "session-1", relation: "execution", title: "发布会话", status: "idle", available: true, projectPath: "D:/app", updatedAt: null },
    { sourceKind: "scheduled_task", sourceId: "task-1", relation: "automation", title: "发布任务", status: "idle", available: true, projectPath: "D:/app", updatedAt: null },
  ],
});

beforeEach(() => {
  mocks.compact = false;
  mocks.items = [fixture()];
  mocks.list.mockReset().mockImplementation(async ({ archived = false }: WorkItemFilters = {}) => mocks.items.filter((item) => item.archived === archived));
  mocks.move.mockReset().mockImplementation(async ({ workItemId, stage }: MoveWorkItemInput) => {
    const item = mocks.items.find((candidate) => candidate.id === workItemId);
    if (item) item.stage = stage;
    return item;
  });
  mocks.archive.mockReset().mockImplementation(async (id: string) => {
    const item = mocks.items.find((candidate) => candidate.id === id);
    if (item) item.archived = true;
    return item;
  });
  mocks.update.mockReset().mockImplementation(async ({ workItemId, ...values }: UpdateWorkItemInput) => {
    const item = mocks.items.find((candidate) => candidate.id === workItemId);
    if (item) Object.assign(item, values);
    return item;
  });
});

describe("WorkBoard", () => {
  it("keeps multi-source work on one accessible card and supports filtering, movement, and archive", async () => {
    render(<WorkBoard />);
    const card = await screen.findByTestId("work-item-work-1");
    expect(within(card).getAllByRole("listitem")).toHaveLength(2);

    fireEvent.change(screen.getByLabelText("按来源筛选"), { target: { value: "scheduled_task" } });
    expect(screen.getByTestId("work-item-work-1")).toBeTruthy();
    fireEvent.change(screen.getByLabelText("工作阶段"), { target: { value: "review" } });
    await waitFor(() => expect(mocks.move).toHaveBeenCalledWith({ workItemId: "work-1", stage: "review" }));

    fireEvent.click(screen.getByRole("button", { name: "归档工作项" }));
    await waitFor(() => expect(screen.queryByTestId("work-item-work-1")).toBeNull());
  });

  it("strips the Windows extended-length prefix from every path it shows", async () => {
    mocks.items = [{ ...fixture(), projectPath: "\\\\?\\D:\\cdavid\\Documents\\code\\cc-switch" }];
    render(<WorkBoard />);

    const card = await screen.findByTestId("work-item-work-1");
    expect(within(card).getByTitle("D:\\cdavid\\Documents\\code\\cc-switch")).toBeTruthy();
    expect(card.textContent).not.toContain("\\\\?\\");

    const projectFilter = screen.getByLabelText("按项目筛选") as HTMLSelectElement;
    const option = within(projectFilter).getByRole("option", { name: "D:\\cdavid\\Documents\\code\\cc-switch" });
    // The label is normalized but the value stays the stored path, or filtering stops matching.
    expect(option.getAttribute("value")).toBe("\\\\?\\D:\\cdavid\\Documents\\code\\cc-switch");
    fireEvent.change(projectFilter, { target: { value: "\\\\?\\D:\\cdavid\\Documents\\code\\cc-switch" } });
    expect(screen.getByTestId("work-item-work-1")).toBeTruthy();
  });

  it("uses one stage column on compact layouts and persists edits", async () => {
    mocks.compact = true;
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    expect(screen.getAllByRole("heading", { level: 2 })).toHaveLength(1);

    fireEvent.click(screen.getByRole("button", { name: "编辑工作项" }));
    const title = screen.getByLabelText("标题");
    fireEvent.change(title, { target: { value: "已编辑版本" } });
    fireEvent.click(screen.getByRole("button", { name: "保存" }));
    await waitFor(() => expect(mocks.update).toHaveBeenCalledWith(expect.objectContaining({ workItemId: "work-1", title: "已编辑版本" })));
  });
});
