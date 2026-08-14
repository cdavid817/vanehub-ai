import { beforeEach, describe, expect, it } from "vitest";
import { resetWebWorkBoardForTest, webWorkBoardClient } from "./web-work-board-client";

describe("Web Work Board adapter", () => {
  beforeEach(() => resetWebWorkBoardForTest());

  it("persists manual work movement and archive independently", async () => {
    const created = await webWorkBoardClient.createWorkItem({ title: "Review release", priority: "high", projectPath: "D:/app" });
    expect(created.stage).toBe("inbox");
    expect(created.sources).toEqual([]);

    const moved = await webWorkBoardClient.moveWorkItem({ workItemId: created.id, stage: "review" });
    expect(moved.stage).toBe("review");
    await webWorkBoardClient.archiveWorkItem(created.id);
    expect((await webWorkBoardClient.listWorkItems()).some((item) => item.id === created.id)).toBe(false);
    expect((await webWorkBoardClient.listWorkItems({ archived: true })).some((item) => item.id === created.id)).toBe(true);
  });

  it("requires archive before permanent deletion and does not duplicate reconciled sources", async () => {
    const created = await webWorkBoardClient.createWorkItem({ title: "Keep safe" });
    await expect(webWorkBoardClient.deleteWorkItem(created.id)).rejects.toThrow(/archive/i);
    const first = await webWorkBoardClient.listWorkItems();
    const second = await webWorkBoardClient.listWorkItems();
    expect(new Set(second.flatMap((item) => item.sources.map((source) => `${source.sourceKind}:${source.sourceId}`))).size)
      .toBe(second.flatMap((item) => item.sources).length);
    expect(second.length).toBe(first.length);
  });
});
