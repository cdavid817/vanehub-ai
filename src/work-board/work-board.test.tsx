// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import type { CreateWorkItemInput, MoveWorkItemInput, UpdateWorkItemInput, WorkItem, WorkItemFilters } from "../types/work-board";

const mocks = vi.hoisted(() => ({
  archive: vi.fn(),
  compact: false,
  create: vi.fn(),
  items: [] as WorkItem[],
  list: vi.fn(),
  move: vi.fn(),
  update: vi.fn(),
}));

vi.mock("../hooks/use-media-query", () => ({ useMediaQuery: () => mocks.compact }));
vi.mock("../services/runtime-work-board-client", () => ({
  workBoardService: {
    archiveWorkItem: mocks.archive,
    createWorkItem: mocks.create,
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
  mocks.create.mockReset().mockImplementation(async (input: CreateWorkItemInput) => {
    const item: WorkItem = {
      id: "work-new", title: input.title, description: input.description ?? "",
      stage: input.stage ?? "inbox", priority: input.priority ?? "none", rank: 1_000,
      projectPath: input.projectPath ?? null, dueAt: input.dueAt ?? null, archived: false,
      createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
    };
    mocks.items.push(item);
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

  it("optimistically moves a card, rolls back with a per-card error on failure, and leaves other cards enabled", async () => {
    const second: WorkItem = { ...fixture(), id: "work-2", title: "第二项", sources: [] };
    mocks.items = [fixture(), second];
    let rejectMove: (reason: unknown) => void = () => {};
    mocks.move.mockReset().mockImplementation(() => new Promise((_resolve, reject) => { rejectMove = reject; }));

    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    const cardTwo = await screen.findByTestId("work-item-work-2");
    const cardTwoEdit = () => within(cardTwo).getByRole("button", { name: "编辑工作项" }) as HTMLButtonElement;
    // Re-queried by testid fresh each time rather than captured once -- an *optimistic* stage
    // change re-parents the card into a different stage column's own DOM subtree immediately
    // (matching what a real, server-confirmed move already does), so a stale element reference
    // would silently stop reflecting updates once it is detached.
    const cardOne = () => screen.getByTestId("work-item-work-1");
    const cardOneStage = () => within(cardOne()).getByLabelText("工作阶段") as HTMLSelectElement;
    const cardOneEdit = () => within(cardOne()).getByRole("button", { name: "编辑工作项" }) as HTMLButtonElement;

    fireEvent.change(cardOneStage(), { target: { value: "review" } });

    // Optimistic: this card's own pending state is applied -- and with it, the new stage -- before
    // the request settles.
    await waitFor(() => expect(cardOneEdit().disabled).toBe(true));
    expect(cardOneStage().value).toBe("review");
    // Per-card, not page-wide: an unrelated card's own actions stay enabled throughout.
    expect(cardTwoEdit().disabled).toBe(false);

    rejectMove(new Error("移动失败"));

    // Rollback: once the rejection lands, the card reverts to its pre-mutation stage and shows
    // its own dismissible error -- not a page-wide banner.
    await waitFor(() => expect(cardOneEdit().disabled).toBe(false));
    expect(cardOneStage().value).toBe("inbox");
    expect(within(cardOne()).getByRole("alert").textContent).toContain("移动失败");

    fireEvent.click(within(cardOne()).getByRole("button", { name: "关闭" }));
    await waitFor(() => expect(within(cardOne()).queryByRole("alert")).toBeNull());
  });

  it("does not reload the whole board after a single card's own successful mutation", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    const loadCallsAfterMount = mocks.list.mock.calls.length;

    fireEvent.click(screen.getByRole("button", { name: "归档工作项" }));
    await waitFor(() => expect(screen.queryByTestId("work-item-work-1")).toBeNull());

    expect(mocks.list.mock.calls.length).toBe(loadCallsAfterMount);
  });

  it("creates a work item by appending the server's response, without a full board reload", async () => {
    render(<WorkBoard />);
    await screen.findByTestId("work-item-work-1");
    const loadCallsAfterMount = mocks.list.mock.calls.length;

    fireEvent.click(screen.getByRole("button", { name: "新建工作项" }));
    fireEvent.change(screen.getByLabelText("标题"), { target: { value: "新工作项" } });
    fireEvent.click(screen.getByRole("button", { name: "创建" }));

    await screen.findByTestId("work-item-work-new");
    expect(mocks.create).toHaveBeenCalledWith(expect.objectContaining({ title: "新工作项" }));
    expect(mocks.list.mock.calls.length).toBe(loadCallsAfterMount);
  });
});
