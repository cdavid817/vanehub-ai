import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

async function openWorkspace(page: Page) {
  await page.goto("/");
  await expect(page).toHaveURL(/\/workspace\/sessions/);
}

test.describe("workspace routing", () => {
  test("addresses every destination and restores them with Back", async ({ page }) => {
    await openWorkspace(page);

    await page.getByRole("button", { name: "Plan 执行", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/plans$/);
    await page.getByRole("button", { name: "循环工程", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/loops$/);
    await page.getByRole("button", { name: "任务看板", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/work-board$/);

    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/loops$/);
    await expect(page.locator("#loop-center")).toBeVisible();
    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/plans$/);
    await expect(page.locator("#plan-center")).toBeVisible();
  });

  test("opens a destination directly from its URL", async ({ page }) => {
    await page.goto("/workspace/loops");
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    await expect(page.locator("#loop-center")).toBeVisible();
    // A deep link has never been "visited" by a click, so this is what proves the visited flags
    // are derived from the destination rather than set by the activity bar handler.
    await expect(page.getByText("暂无循环定义")).toBeVisible();
  });

  test("falls back to sessions for an unknown destination", async ({ page }) => {
    await page.goto("/workspace/nonsense");
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
  });

  /**
   * The retention guarantee this whole change had to avoid breaking: React Router unmounts the
   * previous route element by default, which would reset the Loop Center on every return trip.
   */
  test("preserves destination state across navigation away and back", async ({ page }) => {
    await page.goto("/workspace/loops");
    // Waiting on the shell first: under load `goto` resolves before React mounts, and the
    // 10s element timeout is not always enough to cover a cold Vite compile on its own.
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    const loopCenter = page.locator("#loop-center");
    await expect(loopCenter).toBeVisible();
    await loopCenter.getByRole("button", { name: "新建循环定义" }).click();
    await expect(page.getByRole("heading", { name: /循环定义|新建循环/ })).toBeVisible();
    await page.keyboard.press("Escape");

    await page.getByRole("button", { name: "任务看板", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/work-board$/);
    await page.getByRole("button", { name: "循环工程", exact: true }).click();

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
  test("resumes the previous destination on relaunch", async ({ page }) => {
    await openWorkspace(page);
    await page.getByRole("button", { name: "循环工程", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/loops$/);
    // The location is recorded from an effect, so reloading on the URL alone races the write.
    await expect(page.locator("#loop-center")).toBeVisible();

    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/loops$/);
    await expect(page.locator("#loop-center")).toBeVisible();
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
