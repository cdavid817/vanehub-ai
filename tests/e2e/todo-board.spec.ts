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

  // 14.13: compact width shows a grouped Stage List (every non-empty stage as its own vertical,
  // labeled section) instead of one Kanban column behind a stage-select dropdown -- movement goes
  // through the same WorkItemStageMenu the wide Board uses, with no horizontal drag/scroll at all.
  test("shows a compact grouped Stage List and moves a card between stage groups without a dropdown or drag", async ({ page }) => {
    await page.setViewportSize({ width: 700, height: 720 });
    await page.goto("/");
    await page.getByRole("button", { name: "计划", exact: true }).click();

    // The old single-column stage-select dropdown is gone entirely.
    await expect(page.getByLabel("工作阶段")).toHaveCount(0);

    await page.getByRole("button", { name: "新建工作项" }).click();
    await page.getByLabel("标题").fill("压缩视图任务");
    await page.getByRole("button", { name: "创建", exact: true }).click();

    let card = page.getByTestId(/work-item-web-/).filter({ hasText: "压缩视图任务" });
    await expect(card).toBeVisible();
    await expect(page.getByRole("heading", { name: "收件箱", level: 2 })).toBeVisible();

    await card.getByRole("button", { name: "收件箱" }).click();
    await card.getByRole("option", { name: "已完成" }).click();

    card = page.getByTestId(/work-item-web-/).filter({ hasText: "压缩视图任务" });
    await expect(card).toBeVisible();
    // A second, independently labeled stage group is now visible alongside the first -- proving
    // this is a grouped list (every stage reachable at once), not one column at a time.
    await expect(page.getByRole("heading", { name: "已完成", level: 2 })).toBeVisible();

    // 14.1/14.4: filters (including on a compact viewport) live behind FilterPopover's trigger.
    await page.getByRole("button", { name: "筛选条件" }).click();
    await expect(page.getByLabel("按来源筛选")).toBeVisible();
    await expect(page.getByLabel("按阶段筛选")).toBeVisible();
    await expect(page.getByLabel("按项目筛选")).toBeVisible();
  });

  // 20.19: drives a real stage move through `WorkItemStageMenu` (work-item-stage-menu.tsx, tasks
  // 14.8-14.9) with no pointer at all. That component's own `useMenuList` hook (src/ui/actions/
  // use-menu-list.ts) resets `activeIndex` to 0 on every open, so the freshly created item's own
  // "收件箱" (inbox, index 0 of `workItemStages`) trigger opens with its own listbox option already
  // focused -- one ArrowDown reaches "已计划" (planned, index 1) deterministically. `.focus()`
  // establishes the trigger as this test's own starting point (this file's usual house style is
  // `.click()`-only; the established keyboard-test convention documented in loop-engineering.spec.ts
  // and elsewhere in this codebase is `.focus()` on the entry point, then real key presses for
  // everything downstream of it).
  test("moves a card between stages using only the keyboard, through WorkItemStageMenu", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "计划", exact: true }).click();
    await page.getByRole("tab", { name: "任务看板" }).click();

    await page.getByRole("button", { name: "新建工作项" }).click();
    await page.getByLabel("标题").fill("键盘移动任务");
    await page.getByRole("button", { name: "创建", exact: true }).click();

    const card = page.getByTestId(/work-item-web-/).filter({ hasText: "键盘移动任务" });
    await expect(card).toBeVisible();
    const trigger = card.getByRole("button", { name: "收件箱" });
    await trigger.focus();
    await expect(trigger).toBeFocused();

    await page.keyboard.press("Enter");
    const listbox = card.getByRole("listbox", { name: "移至阶段" });
    await expect(listbox).toBeVisible();
    await expect(card.getByRole("option", { name: "收件箱" })).toBeFocused();

    await page.keyboard.press("ArrowDown");
    const plannedOption = card.getByRole("option", { name: "已计划" });
    await expect(plannedOption).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(listbox).toHaveCount(0);
    await expect(card.getByRole("button", { name: "已计划" })).toBeVisible();
  });
});

/**
 * 20.2/20.17: Work Board had no theme-paired visual coverage at all before this pass (unlike the 7
 * files task 20.2's own research found already carrying it), and its own compact breakpoint
 * (`work-board.tsx`'s `useMediaQuery("(max-width: 900px)")`, 20.1's own evidence) sits on a named
 * width from that task's own list (1600/1440/1280/1100/1024/900/768/640, plus the Tauri minWidth
 * that duplicates 1100 in that same list) -- 1600 and 1100 stay in the wide Kanban mode (1100
 * additionally proving no breakage at this app's real Tauri floor, src-tauri/tauri.conf.json's own
 * `minWidth`), 900 and 640 both land in the compact Stage List mode (900 exactly at the threshold,
 * 640 the narrowest named width). 1440/1280/1024/768 are deliberately not each given their own
 * entry: every one of them falls inside a mode a kept width already demonstrates, so a dedicated
 * screenshot for each would only re-capture an already-proven mode at a slightly different total
 * width, not a new state.
 *
 * Kanban vs. Stage List is asserted structurally, not just visually: `WorkBoardColumn` (Kanban)
 * always renders all 5 stage headers, even an empty one (its own empty-state branch) --
 * `WorkBoardList` in `grouping="stage"` mode (the compact Stage List, work-board.tsx's own 14.13)
 * calls `groupWorkItemsByStage`, which filters empty stages out entirely (work-board-query.ts). A
 * single seeded item (left in "收件箱") makes this a real, checkable difference: Kanban must still
 * show "已完成" (done, empty) as its own column; Stage List must not.
 */
test.describe("Work Board visual theme/width matrix (20.2/20.17)", () => {
  for (const variant of [
    { compact: false, name: "futuristic-wide", theme: "futuristic" as const, width: 1600 },
    { compact: false, name: "minimal-wide", theme: "minimal" as const, width: 1600 },
    { compact: false, name: "futuristic-floor", theme: "futuristic" as const, width: 1100 },
    { compact: false, name: "minimal-floor", theme: "minimal" as const, width: 1100 },
    { compact: true, name: "futuristic-compact-edge", theme: "futuristic" as const, width: 900 },
    { compact: true, name: "minimal-compact-edge", theme: "minimal" as const, width: 900 },
    { compact: true, name: "futuristic-narrow", theme: "futuristic" as const, width: 640 },
    { compact: true, name: "minimal-narrow", theme: "minimal" as const, width: 640 },
  ]) {
    test(`Work Board visual ${variant.name}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: variant.width, height: 900 });
      await page.addInitScript((theme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme })), variant.theme);
      await page.goto("/");
      await page.getByRole("button", { name: "计划", exact: true }).click();
      await page.getByRole("tab", { name: "任务看板" }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

      await page.getByRole("button", { name: "新建工作项" }).click();
      await page.getByLabel("标题").fill("响应式矩阵任务");
      await page.getByRole("button", { name: "创建", exact: true }).click();
      const card = page.getByTestId(/work-item-web-/).filter({ hasText: "响应式矩阵任务" });
      await expect(card).toBeVisible();

      const doneHeading = page.getByRole("heading", { name: "已完成", level: 2 });
      if (variant.compact) {
        // Stage List: an empty stage's own group section is filtered out entirely.
        await expect(doneHeading).toHaveCount(0);
        await expect(page.getByRole("heading", { name: "收件箱", level: 2 })).toBeVisible();
      } else {
        // Kanban: every stage renders its own column, even an empty one.
        await expect(doneHeading).toBeVisible();
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.locator("#todo-board").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
    });
  }
});
