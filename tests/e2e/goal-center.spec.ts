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

/**
 * 20.2/20.17: Goal Center had no theme-paired visual coverage at all before this pass. Its own
 * list/detail split is Tailwind's plain `md:` breakpoint (768px, `goal-center.tsx`'s own
 * `md:grid-cols-[...]`) -- one of task 20.2's own named widths already, so this reuses it directly
 * rather than picking an arbitrary "narrow" number: 768 keeps both panes side by side (Tailwind's
 * `md:` is min-width, inclusive at 768), 640 is the next named width down and switches to the
 * compact "detail replaces list, with Back" mode (design.md Decision 12's "窄屏 detail 替换 list
 * 并有明确返回", the same pattern `projects.tsx`'s own 13.12 test already covers for that
 * destination). Not a full 9-width sweep: Goal Center's own layout has exactly one real
 * breakpoint, already bracketed tightly by these two named widths.
 */
test.describe("Goal Center visual theme/width matrix (20.2/20.17)", () => {
  for (const variant of [
    { compact: false, name: "futuristic-wide", theme: "futuristic" as const, width: 768 },
    { compact: false, name: "minimal-wide", theme: "minimal" as const, width: 768 },
    { compact: true, name: "futuristic-narrow", theme: "futuristic" as const, width: 640 },
    { compact: true, name: "minimal-narrow", theme: "minimal" as const, width: 640 },
  ]) {
    test(`Goal Center visual ${variant.name}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: variant.width, height: 900 });
      await page.addInitScript((theme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme })), variant.theme);
      await page.goto("/");
      await page.getByRole("button", { name: "计划", exact: true }).click();
      await page.getByRole("tab", { name: "目标中心" }).click();
      await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

      await page.getByRole("button", { name: "新建目标" }).click();
      await page.getByLabel("标题").fill("响应式矩阵目标");
      await page.getByRole("button", { name: "创建", exact: true }).click();
      // Goal Center auto-selects a just-created goal (goal-center.tsx's own create onSuccess), so
      // the detail pane is already showing it -- no separate list-item click is needed here.
      await expect(page.getByRole("heading", { level: 2, name: "响应式矩阵目标" })).toBeVisible();

      const list = page.getByRole("list", { name: "目标列表" });
      const backButton = page.getByRole("button", { name: "返回列表", exact: true });
      if (variant.compact) {
        await expect(list).toHaveCount(0);
        await expect(backButton).toBeVisible();
      } else {
        await expect(list).toBeVisible();
        await expect(backButton).toHaveCount(0);
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.locator("#goal-center").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
    });
  }
});
