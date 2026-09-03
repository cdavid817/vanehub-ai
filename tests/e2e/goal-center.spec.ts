import { expect, test } from "@playwright/test";
import { createAndRunLoop, openLoops } from "./loop-helpers";

/**
 * 20.19: Goal Center has no e2e spec of its own yet -- this is the first one, scoped to the one
 * flow 20.19 names for it: linking an execution target to a Goal through `ExecutionTargetPicker`
 * (execution-target-picker.tsx, tasks 15.4-15.6), driven with no pointer at all for the linking step
 * itself. Setup (creating a Loop to link, then creating the Goal) reuses this codebase's existing
 * `.click()`-driven helpers and GoalForm's own established labels (goal-center.test.tsx's own
 * "新建目标"/"标题"/"创建" flow) -- the same split this whole task's other new keyboard tests use:
 * scene-setting via the already-proven click-driven path, the flow actually under test via real
 * `page.keyboard` interaction.
 */
test.describe("Goal Center", () => {
  test("links a Loop as an execution target using only the keyboard", async ({ page }) => {
    await page.goto("/");
    await openLoops(page);
    await createAndRunLoop(page, "键盘关联循环目标");

    await page.getByRole("button", { name: "计划", exact: true }).click();
    await page.getByRole("tab", { name: "目标中心" }).click();

    await page.getByRole("button", { name: "新建目标" }).click();
    await page.getByLabel("标题").fill("键盘关联目标");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByRole("heading", { level: 2, name: "键盘关联目标" })).toBeVisible();

    // ExecutionTargetPicker defaults to searching the "loop" kind on mount (execution-target-
    // picker.tsx / execution-target-picker.test.tsx's own "defaults to searching the loop kind"
    // case), so no kind-select change is needed before searching for the Loop created above.
    const searchInput = page.getByLabel("搜索关联对象");
    await searchInput.focus();
    await expect(searchInput).toBeFocused();
    await page.keyboard.type("键盘关联循环目标");

    const resultButton = page.getByRole("button", { name: /^键盘关联循环目标/ });
    await expect(resultButton).toBeVisible();
    await resultButton.focus();
    await page.keyboard.press("Enter");

    // Clicking a result only stages it (design.md Decision 12's search-then-confirm, not search-
    // then-instant-link) -- the real mutation only fires once "关联" (Link) is itself activated.
    const linkButton = page.getByRole("button", { name: "关联", exact: true });
    await expect(linkButton).toBeVisible();
    await linkButton.focus();
    await expect(linkButton).toBeFocused();
    await page.keyboard.press("Enter");

    const loopGroup = page.getByRole("heading", { level: 4, name: /^循环/ });
    await expect(loopGroup).toBeVisible();
    await expect(loopGroup).toContainText("1");
    // Back to search: the confirm panel's own "关联"/"更换" pair only ever renders while a result
    // is staged, so its absence proves the picker really returned to its search state afterward.
    await expect(page.getByRole("button", { name: "更换" })).toHaveCount(0);
  });
});
