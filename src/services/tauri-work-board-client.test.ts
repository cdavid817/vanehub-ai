import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
import { tauriWorkBoardClient } from "./tauri-work-board-client";

describe("Tauri Work Board adapter", () => {
  beforeEach(() => invokeMock.mockReset().mockResolvedValue({}));

  it("maps every service operation to the native command contract", async () => {
    const create = { title: "Ship" };
    const update = { workItemId: "work-1", priority: "high" as const };
    const move = { workItemId: "work-1", stage: "review" as const };
    const link = { workItemId: "work-1", sourceKind: "session" as const, sourceId: "session-1", relation: "execution" as const };
    await tauriWorkBoardClient.listWorkItems();
    await tauriWorkBoardClient.createWorkItem(create);
    await tauriWorkBoardClient.updateWorkItem(update);
    await tauriWorkBoardClient.moveWorkItem(move);
    await tauriWorkBoardClient.linkWorkItemSource(link);
    await tauriWorkBoardClient.archiveWorkItem("work-1");
    await tauriWorkBoardClient.restoreWorkItem("work-1");
    await tauriWorkBoardClient.deleteWorkItem("work-1");
    expect(invokeMock.mock.calls).toEqual([
      ["list_work_items", { filters: {} }], ["create_work_item", { input: create }],
      ["update_work_item", { input: update }], ["move_work_item", { input: move }],
      ["link_work_item_source", { input: link }], ["archive_work_item", { workItemId: "work-1" }],
      ["restore_work_item", { workItemId: "work-1" }], ["delete_work_item", { workItemId: "work-1" }],
    ]);
  });
});
