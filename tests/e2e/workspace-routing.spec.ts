import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

async function openWorkspace(page: Page) {
  await page.goto("/");
  await expect(page).toHaveURL(/\/workspace\/sessions/);
}

test.describe("workspace routing", () => {
  test("addresses every destination and restores them with Back", async ({ page }) => {
    await openWorkspace(page);

    await page.getByRole("button", { name: "自动化", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/automations\/loops$/);
    await page.getByRole("button", { name: "收件箱", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);

    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/automations\/loops$/);
    await expect(page.locator("#loop-center")).toBeVisible();
    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/sessions$/);
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
  });

  test("opens a destination sub-view directly from its URL", async ({ page }) => {
    await page.goto("/workspace/automations/loops");
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    await expect(page.locator("#loop-center")).toBeVisible();
    // A deep link has never been "visited" by a click, so this is what proves the visited flags
    // are derived from the destination rather than set by the activity bar handler.
    await expect(page.getByText("暂无循环定义")).toBeVisible();

    await page.goto("/workspace/automations/goals");
    await expect(page.getByRole("tab", { name: "目标中心" })).toHaveAttribute("aria-selected", "true");
    await expect(page.locator("#goal-center")).toBeVisible();
  });

  test("falls back to sessions for an unknown destination and to the default view for an unknown view", async ({ page }) => {
    await page.goto("/workspace/nonsense");
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
    await page.goto("/workspace/inbox/nonsense");
    await expect(page).toHaveURL(/\/workspace\/inbox\/nonsense$/);
    await expect(page.getByTestId("inbox")).toBeVisible();
    await expect(page.getByTestId("inbox-sections")).toBeVisible();
  });

  test("redirects retired destinations to their new hosts", async ({ page }) => {
    await page.goto("/workspace/loops");
    await expect(page).toHaveURL(/\/workspace\/automations\/loops$/);
    await expect(page.locator("#loop-center")).toBeVisible();

    await page.goto("/workspace/goals");
    await expect(page).toHaveURL(/\/workspace\/automations\/goals$/);
    await expect(page.locator("#goal-center")).toBeVisible();

    await page.goto("/workspace/work-board");
    await expect(page).toHaveURL(/\/workspace\/inbox\/board$/);
    await expect(page.locator("#todo-board")).toBeVisible();

    await page.goto("/workspace/mission-control");
    await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);
    // The Board choice made by the work-board redirect above persists, so Inbox is still in Board
    // mode here; switching back to List is a preference change, not a route change.
    await expect(page.getByTestId("inbox")).toBeVisible();
    await page.getByTestId("inbox-view-list").click();
    await expect(page.getByTestId("inbox-sections")).toBeVisible();

    await page.goto("/workspace/system-activity");
    await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);

    await page.goto("/workspace/evaluations");
    await expect(page).toHaveURL(/\/settings\?section=evaluation$/);
    await expect(page.getByTestId("evaluation-center")).toBeVisible();
  });

  test("redirects a remembered retired location on launch", async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.workspace.location.v1", "/workspace/goals");
    });
    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/automations\/goals$/);
    await expect(page.locator("#goal-center")).toBeVisible();
  });

  /**
   * The retention guarantee this whole change had to avoid breaking: React Router unmounts the
   * previous route element by default, which would reset the Loop Center on every return trip.
   */
  test("preserves destination state across navigation away and back", async ({ page }) => {
    await page.goto("/workspace/automations/loops");
    // Waiting on the shell first: under load `goto` resolves before React mounts, and the
    // 10s element timeout is not always enough to cover a cold Vite compile on its own.
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    const loopCenter = page.locator("#loop-center");
    await expect(loopCenter).toBeVisible();
    await loopCenter.getByRole("button", { name: "新建循环定义" }).click();
    await expect(page.getByRole("heading", { name: /循环定义|新建循环/ })).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByRole("button", { name: "收件箱", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/inbox\/attention$/);
    await page.getByRole("button", { name: "自动化", exact: true }).click();

    // Still mounted: the panel is present immediately rather than replaying its loading state.
    await expect(loopCenter).toBeVisible();
    await expect(loopCenter.getByText("正在加载循环工程...")).toHaveCount(0);
  });

  test("puts the active session in the URL", async ({ page }) => {
    await openWorkspace(page);
    await createSession(page, "路由会话");

    await expect(page).toHaveURL(/\/workspace\/sessions\/.+/);
    await expect(page.getByTestId("session-conversation-header").getByText("路由会话")).toBeVisible();
  });

  /**
   * Only the location is asserted after the reload. Web/mock session state lives in module
   * memory and does not survive a full document load, so asserting the session would test the
   * mock's lifetime rather than the restore behaviour.
   */
  test("resumes the previous destination and tab on relaunch", async ({ page }) => {
    await openWorkspace(page);
    await page.getByRole("button", { name: "自动化", exact: true }).click();
    await page.getByRole("tab", { name: "定时任务" }).click();
    await expect(page).toHaveURL(/\/workspace\/automations\/scheduled$/);
    // The location is recorded from an effect, so reloading on the URL alone races the write.
    await expect(page.getByTestId("scheduled-tasks-panel")).toBeVisible();

    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/automations\/scheduled$/);
    await expect(page.getByTestId("scheduled-tasks-panel")).toBeVisible();
  });

  test("falls back to the session list for a session that does not exist", async ({ page }) => {
    await page.goto("/workspace/sessions/session-does-not-exist");
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
  });

  test("expresses session creation as a route", async ({ page }) => {
    await page.goto("/workspace/sessions/new");
    await expect(page.getByRole("dialog", { name: "创建会话" })).toBeVisible();

    await page.getByRole("button", { name: "取消", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "创建会话" })).toHaveCount(0);
    await expect(page).toHaveURL(/\/workspace\/sessions/);
  });
});
