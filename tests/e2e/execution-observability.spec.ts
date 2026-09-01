import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("execution observability", () => {
  test("validates safe Web settings defaults at a narrow viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    // Below `lg` the sidebar is hidden in favor of a searchable sheet (task 12.9): open it first.
    await page.getByRole("button", { name: /^(切换设置页面|Switch settings page)/ }).click();
    await page.getByRole("button", { name: /执行可观测性|Execution Observability/ }).click();

    await expect(
      page.getByRole("heading", {
        level: 2,
        name: /^(执行可观测性|Execution Observability)$/,
      }),
    ).toBeVisible();
    await expect(
      page.getByRole("combobox", { name: /^(采集策略|Collection policy)$/ }),
    ).toHaveValue("metadata_only");
    await expect(page.getByRole("checkbox", { name: /调用级 MCP 中继|invocation-scoped MCP relay/ })).toBeDisabled();

    const retention = page.getByRole("spinbutton", { name: /保留天数|Retention/ });
    await retention.fill("0");
    await expect(page.getByText(/保留天数必须是 1 到 90|Retention must be an integer from 1 to 90/)).toBeVisible();
    await expect(
      page.getByRole("button", {
        name: /^(保存可观测性设置|Save observability settings)$/,
      }),
    ).toBeDisabled();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  test("never shows a saved OTLP auth token again, even to the form that collected it (task 12.20)", async ({ page }) => {
    const secret = "playwright-e2e-otlp-bearer-token";
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^(切换设置页面|Switch settings page)/ }).click();
    await page.getByRole("button", { name: /执行可观测性|Execution Observability/ }).click();

    // Deliberately leave "启用 OTLP/HTTP protobuf 导出" unchecked: `webExecutionObservabilityClient`
    // throws "Native OTLP export ... unavailable in Web preview" on any save with it on, so a save
    // that exercises the OTLP-enabled path can never succeed against this suite's Web/mock backend.
    // Staging the collector address and token first (matching the section's own "设置对后续运行生效"
    // copy) is a real, supported flow and still exercises the same submit-then-never-echo-back path.
    await page.getByRole("textbox", { name: "Collector 地址" }).fill("https://otel.example.com/v1/traces");
    // `type="password"` has no implicit ARIA role, so `getByRole("textbox", ...)` cannot find it.
    await page.getByLabel("Bearer 令牌（由操作系统凭据服务保存）").fill(secret);
    await page.getByRole("button", { name: "保存可观测性设置" }).click();
    await expect(page.getByText(/可观测性配置已保存/)).toBeVisible();
    await expect(page.locator("body")).not.toContainText(secret);

    // The just-saved token is never re-populated. Unlike SSH's password field, the Web/mock backend
    // never persists an "already configured" credential flag for this native-only capability -- it
    // unconditionally nulls `otlpAuthToken` server-side (see `webExecutionObservabilityClient.updateSettings`)
    // without ever flipping `otlpAuthConfigured`, so the field falls back to its normal empty-state
    // placeholder rather than SSH's "already configured" one. What matters here -- the raw secret is
    // never echoed back -- still holds.
    const tokenField = page.getByLabel("Bearer 令牌（由操作系统凭据服务保存）");
    await expect(tokenField).toHaveValue("");
    await expect(tokenField).toHaveAttribute("placeholder", "可选令牌");
    await expect(page.locator("body")).not.toContainText(secret);
  });

  test("shows successful, failed, incomplete, opaque, and paginated timelines", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await createSession(page, "可观测链路测试");
    await page.getByRole("tab", { name: /链路|Traces/ }).click();

    await expect(page.getByRole("heading", { name: /执行时间线|Execution timeline/ })).toBeVisible();
    await expect(page.getByText(/execute_tool search/)).toBeVisible();
    await expect(page.getByText(/mcp.client request/)).toBeVisible();
    await expect(page.getByText(/^推断$|^Inferred$/)).toBeVisible();
    await expect(page.getByText(/^不可见$|^Opaque$/)).toBeVisible();
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
