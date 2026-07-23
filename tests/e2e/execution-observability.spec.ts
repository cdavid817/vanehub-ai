import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("execution observability", () => {
  test("validates safe Web settings defaults at a narrow viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /执行可观测性|Execution Observability/ }).click();

    await expect(page.getByRole("heading", { name: /执行可观测性|Execution Observability/ })).toBeVisible();
    await expect(page.getByText(/仅元数据|Metadata only/)).toBeVisible();
    await expect(page.getByRole("checkbox", { name: /调用级 MCP 中继|invocation-scoped MCP relay/ })).toBeDisabled();

    const retention = page.getByRole("spinbutton", { name: /保留天数|Retention/ });
    await retention.fill("0");
    await expect(page.getByText(/保留天数必须是 1 到 90|Retention must be an integer from 1 to 90/)).toBeVisible();
    await expect(page.getByRole("button", { name: /^保存$|^Save$/ })).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  test("shows successful, failed, incomplete, opaque, and paginated timelines", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await createSession(page, "可观测链路测试");
    await page.getByRole("tab", { name: /链路|Traces/ }).click();

    await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
    await expect(page.getByText(/execute_tool search/)).toBeVisible();
    await expect(page.getByText(/mcp.client request/)).toBeVisible();
    await expect(page.getByText(/推断|Inferred/)).toBeVisible();
    await expect(page.getByText(/不可见|Opaque/)).toBeVisible();
    await expect(page.getByText(/观测缺口|Observation gap/).first()).toBeVisible();

    await page.getByText(/^失败$|^Failed$/).first().click();
    await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
    await expect(page.getByText(/^失败$|^Failed$/).last()).toBeVisible();

    const loadMore = page.getByRole("button", { name: /加载更早记录|Load earlier runs/ });
    await expect(loadMore).toBeVisible();
    await loadMore.click();
    await expect(loadMore).toBeHidden();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});
