import { expect, test, type Page } from "@playwright/test";
import { expectNoSeriousAxeViolations } from "./a11y-helpers";

type Theme = "futuristic" | "minimal";

/**
 * 20.18: first increment of automated axe-core coverage across this app's most-trafficked,
 * recently-stabilized destinations -- deliberately not "every route" in one pass. See this task's
 * own tasks.md evidence for the full accounting of what is/isn't covered yet and why these four.
 *
 * A dedicated file, not an addition to each destination's own existing spec
 * (`todo-board.spec.ts`, `goal-center.spec.ts`, `mission-control.spec.ts`): all three were under
 * sustained, real concurrent edits from other agents sharing this same worktree for this task's
 * entire duration (their own 20.2/20.17 viewport-matrix and theme-parity work, confirmed via
 * repeated `git status`/`git diff` checks throughout) -- editing them directly risked losing or
 * mis-attributing that in-flight work at commit time (which happened once here to a different file
 * during this same task and was caught and undone before landing, never pushed). Evaluation
 * Center's own axe test *did* land inside its existing spec (`evaluation-center.spec.ts`) because
 * that file had no concurrent activity for this task's entire duration.
 *
 * Every destination below scans both shipped themes, not just one: Evaluation Center's own pass
 * (evaluation-center.spec.ts) found two real `color-contrast` failures that only existed in one
 * theme each, and this file's own first draft (single-theme) missed a third, real, minimal-theme-
 * only failure (`--primary` on `bg-nav-active-soft`, the active-tab highlight both Work Board and
 * Goal Center render) that only surfaced once a second theme was actually tried -- all three fixed
 * in styles.css. Theme is a real, load-bearing axis for this rule, not incidental variation.
 */

async function setTheme(page: Page, language: "en" | "zh-CN", theme: Theme) {
  await page.addInitScript((settings) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify(settings));
  }, { applicationLanguage: language, theme });
}

for (const theme of ["futuristic", "minimal"] as const) {
  test(`Mission Control has no serious or critical automated accessibility violations (${theme})`, async ({ page }) => {
    await setTheme(page, "en", theme);
    await page.goto("/");
    // Mission Control is a Runs section (runs-destination.tsx), not its own activity-bar entry --
    // clicking "Runs" already lands on its default "attention" section, where it renders.
    await page.getByRole("button", { name: "Runs", exact: true }).click();
    await expect(page.getByTestId("mission-control")).toBeVisible();
    // Populated, realistic state: the Web mock seeds fixture Runs by default (no setup needed), and
    // selecting one renders the detail pane and its facet navigation too, not just the list.
    await page.locator("[data-testid^='mission-run-']").first().locator("button").first().click();
    await expect(page.getByRole("tablist")).toBeVisible();

    await expectNoSeriousAxeViolations(page);
  });

  test(`Work Board has no serious or critical automated accessibility violations (${theme})`, async ({ page }) => {
    await setTheme(page, "zh-CN", theme);
    await page.goto("/");
    // Board is a Plan section (plan-destination.tsx), not its own activity-bar entry.
    await page.getByRole("button", { name: "计划", exact: true }).click();
    await page.getByRole("tab", { name: "任务看板" }).click();
    await expect(page.getByRole("heading", { name: "任务看板" })).toBeVisible();

    // The Web mock's Work Board starts empty -- a real card (with its stage badges, priority, and
    // source) is created here the same way todo-board.spec.ts's own functional test does, so the
    // scan covers real card rendering plus the toolbar/filters, not just an empty-state message.
    await page.getByRole("button", { name: "新建工作项" }).click();
    await page.getByLabel("标题").fill("无障碍扫描工作项");
    await page.getByLabel("描述").fill("验证 axe 扫描覆盖真实卡片");
    await page.getByLabel("项目路径").fill("D:/a11y-scan");
    await page.getByLabel("优先级", { exact: true }).selectOption("high");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByTestId(/work-item-web-/).filter({ hasText: "无障碍扫描工作项" })).toBeVisible();

    await expectNoSeriousAxeViolations(page);
  });

  test(`Goal Center has no serious or critical automated accessibility violations (${theme})`, async ({ page }) => {
    await setTheme(page, "zh-CN", theme);
    await page.goto("/");
    await page.getByRole("button", { name: "计划", exact: true }).click();
    await page.getByRole("tab", { name: "目标中心" }).click();

    // The Web mock's Goal Center starts empty -- a real Goal is created so the scan covers the
    // selected-detail pane (GoalDetail, including its relationship sections) too, not just the
    // empty list/detail placeholder state.
    await page.getByRole("button", { name: "新建目标" }).click();
    await page.getByLabel("标题").fill("无障碍扫描目标");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByRole("heading", { level: 2, name: "无障碍扫描目标" })).toBeVisible();

    await expectNoSeriousAxeViolations(page);
  });
}
