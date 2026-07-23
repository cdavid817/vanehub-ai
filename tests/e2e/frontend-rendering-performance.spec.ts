import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("frontend rendering performance", () => {
  test("windows a 500+ Prompt Hook inventory and preserves lazy page state", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.addInitScript(() => {
      const hooks = Object.fromEntries(Array.from({ length: 501 }, (_, index) => {
        const id = `user-windowing-${String(index).padStart(3, "0")}`;
        return [id, {
          id,
          name: `Hook ${index}`,
          description: `Virtualized Prompt Hook ${index}`,
          category: "static",
          stage: "session-init",
          order: 1_000 + index,
          version: 1,
          source: "user",
          enabled: true,
          disableable: true,
          cliBindings: ["codex-cli"],
          governance: {
            safetyTier: "editable",
            transparencyTier: "opt-in-view",
            governanceTier: "human-gated",
          },
          templateBody: `Prompt body ${index}`,
          createdAt: "2026-07-23T00:00:00.000Z",
          updatedAt: "2026-07-23T00:00:00.000Z",
        }];
      }));
      window.localStorage.setItem("vanehub.prompt-hooks.v1", JSON.stringify(hooks));
    });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    const list = page.getByTestId("prompt-hook-virtual-list");
    await expect(list).toBeVisible();
    await expect(list).toHaveAttribute("data-virtual-count", "508");
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(30);

    await list.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    const lastCard = list.getByRole("listitem").filter({ hasText: "Hook 500" });
    await expect(lastCard).toBeVisible();
    await lastCard.getByRole("button", { name: "预览 Hook 内容" }).click();
    await expect(page.getByText("Prompt body 500", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "关闭" }).click();

    const filter = page.getByPlaceholder("按 ID、名称、描述、分类或来源搜索");
    await filter.fill("Hook 500");
    await page.getByRole("button", { name: "Agent 管理" }).click();
    await expect(page.getByRole("heading", { name: "Agent 管理" })).toBeVisible();
    await page.getByRole("button", { name: "Prompt Hook" }).click();
    await expect(filter).toHaveValue("Hook 500");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  test("virtualizes Agent logs and locates a timestamp across bounded pages", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await createSession(page, "日志虚拟化测试");
    await page.getByRole("tab", { name: "日志" }).click();

    const list = page.getByTestId("session-log-virtual-list");
    await expect(list).toBeVisible();
    await expect(list).toHaveAttribute("data-virtual-count", "201");
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(50);

    await page.getByRole("button", { name: "定位", exact: true }).click();
    await expect(page.getByText("请输入有效时间。")).toBeVisible();

    await list.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    await page.getByRole("button", { name: "加载更多" }).click();
    await expect(list).toHaveAttribute("data-virtual-count", "401");

    const localTimestamp = await page.evaluate(() => {
      const value = new Date("2026-07-17T00:30:00.000Z");
      const pad = (part: number) => String(part).padStart(2, "0");
      return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}T${pad(value.getHours())}:${pad(value.getMinutes())}`;
    });
    await page.getByLabel("日志时间").fill(localTimestamp);
    await page.getByRole("button", { name: "定位", exact: true }).click();

    const located = page.locator('[data-log-id="web-log-150-history"]');
    await expect(located).toBeVisible();
    await expect(located).toBeFocused();
    await expect(list).toHaveAttribute("data-virtual-count", "600");
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(50);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});
