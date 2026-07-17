import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("unified notifications", () => {
  test("opens and dismisses the empty notification center", async ({ page }) => {
    await page.goto("/");

    await page.getByRole("button", { name: "通知" }).click();
    const center = page.getByRole("dialog", { name: "通知" });
    await expect(center).toBeVisible();
    await expect(center.getByText("暂无未处理通知")).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(center).toBeHidden();

    await page.getByRole("button", { name: "通知" }).click();
    await page.getByRole("heading", { name: "VaneHub AI" }).click();
    await expect(center).toBeHidden();
  });

  test("keeps an expired session toast in notification history", async ({ page }) => {
    await page.goto("/");

    await createSession(page, "通知测试会话");

    const toast = page.getByRole("status").filter({ hasText: "会话创建成功" });
    await expect(toast).toBeVisible();
    await expect(page.getByRole("button", { name: "1 条未读通知" })).toBeVisible();
    await expect(toast).toBeHidden({ timeout: 7_000 });

    await page.getByRole("button", { name: "1 条未读通知" }).click();
    const center = page.getByRole("dialog", { name: "通知" });
    await expect(center.getByText("会话创建成功")).toBeVisible();
    await expect(center.getByText("通知测试会话 已准备就绪。")).toBeVisible();

    await center.getByRole("button", { name: "全部标为已读" }).click();
    await expect(page.getByRole("button", { name: "通知", exact: true })).toBeVisible();
  });

  test("clears in-memory notification history after a reload", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "重载通知测试");
    await expect(page.getByRole("button", { name: "1 条未读通知" })).toBeVisible();

    await page.reload();
    await page.getByRole("button", { name: "通知", exact: true }).click();
    const center = page.getByRole("dialog", { name: "通知" });
    await expect(center.getByText("暂无未处理通知")).toBeVisible();
    await expect(center.getByText("重载通知测试 已准备就绪。")).toHaveCount(0);
  });

  test("uses English minimal-theme chrome within a narrow viewport", async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem(
        "vanehub.appSettings",
        JSON.stringify({ applicationLanguage: "en", theme: "minimal" }),
      );
    });
    await page.setViewportSize({ width: 375, height: 720 });
    await page.goto("/");

    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    await page.getByRole("button", { name: "Notifications" }).click();
    const center = page.getByRole("dialog", { name: "Notifications" });
    await expect(center.getByText("You're all caught up")).toBeVisible();

    const box = await center.boundingBox();
    expect(box).not.toBeNull();
    expect(box?.x ?? -1).toBeGreaterThanOrEqual(0);
    expect((box?.x ?? 0) + (box?.width ?? 0)).toBeLessThanOrEqual(375);
  });
});
