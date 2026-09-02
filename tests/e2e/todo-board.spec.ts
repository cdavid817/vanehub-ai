import { expect, test } from "@playwright/test";

test.describe("Todo Board", () => {
  test("creates, edits, moves, filters, archives, restores, and deletes manual work", async ({ page }) => {
    await page.goto("/");
    // Board is a Plan section now (plan-destination.tsx), not its own activity-bar entry —
    // design.md Decision 1 folded it into Plan's secondary navigation alongside Goals.
    await page.getByRole("button", { name: "计划", exact: true }).click();
    const navigationEntry = page.getByRole("tab", { name: "任务看板" });
    await navigationEntry.click();
    await expect(navigationEntry).toHaveClass(/text-primary/);
    await expect(page.getByRole("heading", { name: "任务看板" })).toBeVisible();

    await page.getByRole("button", { name: "新建工作项" }).click();
    await page.getByLabel("标题").fill("发布 Todo Board");
    await page.getByLabel("描述").fill("验证统一工作流");
    await page.getByLabel("项目路径").fill("D:/todo-board");
    await page.getByLabel("优先级", { exact: true }).selectOption("high");
    await page.getByRole("button", { name: "创建", exact: true }).click();

    let card = page.getByTestId(/work-item-web-/).filter({ hasText: "发布 Todo Board" });
    await expect(card).toBeVisible();
    await expect(card.getByText("人工待办")).toBeVisible();
    // 14.1/14.4: filters now live behind FilterPopover's own trigger, not a permanently-visible grid.
    await page.getByRole("button", { name: "筛选条件" }).click();
    await page.getByLabel("按项目筛选").selectOption("D:/todo-board");
    await expect(card).toBeVisible();

    // 14.6: Edit/Archive/Delete now live in each card's own More menu, not as bare buttons.
    await card.getByRole("button", { name: "更多操作" }).click();
    await card.getByRole("menuitem", { name: "编辑工作项" }).click();
    await page.getByLabel("标题").fill("发布统一任务看板");
    // exact: true -- 14.5's new "已保存视图" (Saved views) trigger substring-matches "保存" too.
    await page.getByRole("button", { name: "保存", exact: true }).click();
    card = page.getByTestId(/work-item-web-/).filter({ hasText: "发布统一任务看板" });
    await expect(card).toBeVisible();

    await card.getByRole("button", { name: "收件箱" }).click();
    await card.getByRole("option", { name: "已计划" }).click();
    await expect(card.getByRole("button", { name: "已计划" })).toBeVisible();
    await card.getByRole("button", { name: "更多操作" }).click();
    await card.getByRole("menuitem", { name: "归档工作项" }).click();
    await expect(card).toHaveCount(0);

    await page.getByRole("button", { name: "归档", exact: true }).click();
    card = page.getByTestId(/work-item-web-/).filter({ hasText: "发布统一任务看板" });
    await expect(card).toBeVisible();
    await card.getByRole("button", { name: "恢复" }).click();
    await expect(card).toHaveCount(0);

    await page.getByRole("button", { name: "当前看板" }).click();
    card = page.getByTestId(/work-item-web-/).filter({ hasText: "发布统一任务看板" });
    await card.getByRole("button", { name: "更多操作" }).click();
    await card.getByRole("menuitem", { name: "归档工作项" }).click();
    await page.getByRole("button", { name: "归档", exact: true }).click();
    card = page.getByTestId(/work-item-web-/).filter({ hasText: "发布统一任务看板" });
    await card.getByRole("button", { name: "更多操作" }).click();
    await card.getByRole("menuitem", { name: "永久删除" }).click();
    await expect(card).toHaveCount(0);
  });

  test("keeps every stage reachable on a compact viewport", async ({ page }) => {
    await page.setViewportSize({ width: 700, height: 720 });
    await page.goto("/");
    await page.getByRole("button", { name: "计划", exact: true }).click();

    const stage = page.getByLabel("工作阶段").first();
    await expect(stage).toBeVisible();
    await stage.selectOption("done");
    await expect(page.getByRole("heading", { name: "已完成", level: 2 })).toBeVisible();
    await expect(page.getByRole("heading", { level: 2 })).toHaveCount(1);
    // 14.1/14.4: filters (including on a compact viewport) live behind FilterPopover's trigger.
    await page.getByRole("button", { name: "筛选条件" }).click();
    await expect(page.getByLabel("按来源筛选")).toBeVisible();
    await expect(page.getByLabel("按阶段筛选")).toBeVisible();
    await expect(page.getByLabel("按项目筛选")).toBeVisible();
  });
});
