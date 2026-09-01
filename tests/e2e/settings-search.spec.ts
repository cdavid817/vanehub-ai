import { expect, test } from "@playwright/test";

test.describe("cross-page settings search (task 12.20)", () => {
  test("selecting a page-title result navigates straight to that page", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();

    const searchBox = page.locator("header").getByRole("combobox");
    await searchBox.fill("SSH 连接");
    // The option's accessible name concatenates its label span with its description span (the same
    // sibling-text-folds-into-name shape already documented for nav-entry status dots), so an
    // anchored prefix regex is required instead of an exact match.
    await page.getByRole("option", { name: /^SSH 连接/ }).click();

    await expect(page.getByRole("button", { name: "新增连接" })).toBeVisible();
    await expect(page.getByRole("listbox")).toHaveCount(0);
  });

  test("a keyword synonym absent from the page's own title and description still finds it", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    const searchBox = page.locator("header").getByRole("combobox");

    // "wechat" is one of `im`'s search keywords (settings-page-search-metadata.ts) but appears in
    // neither its nav label ("IM 能力") nor its description ("通过机器人令牌、Webhook 与健康检查连接聊天平台。") --
    // only a real keyword-index match, not an incidental label/description substring hit, can find it.
    await searchBox.fill("wechat");
    await expect(page.getByRole("option", { name: /^IM 能力/ })).toBeVisible();

    // Same proof for "sandbox", one of `extensions`'s keywords, absent from its own label
    // ("扩展能力") and description ("安装、启动并测试可选的运行时扩展框架。").
    await searchBox.fill("sandbox");
    await expect(page.getByRole("option", { name: /^扩展能力/ })).toBeVisible();
  });

  test("a query matching nothing shows the no-results state, not an empty or crashed dropdown", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    const searchBox = page.locator("header").getByRole("combobox");

    await searchBox.fill("zzz-no-such-setting-exists-12345");
    // Scoped to `<header>`: an unscoped `role=status` also matches whatever page-local loading
    // status the currently active settings page happens to render at the same time.
    await expect(page.locator("header").getByRole("status")).toHaveText("没有匹配的设置项。");
    await expect(page.getByRole("listbox")).toHaveCount(0);
    await expect(page.getByRole("option")).toHaveCount(0);
  });

  test("ArrowDown moves the highlighted result before Enter selects it", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    const searchBox = page.locator("header").getByRole("combobox");

    await searchBox.fill("CLI");
    const options = page.getByRole("option");
    await expect(options.first()).toBeVisible();
    const count = await options.count();
    expect(count).toBeGreaterThanOrEqual(2);
    // Read the second result's own label directly from its label span (not the folded accessible
    // name) so this assertion holds regardless of which pages currently match "CLI".
    const secondOptionLabel = (await options.nth(1).locator("span").first().innerText()).trim();

    await searchBox.press("ArrowDown");
    await expect(options.nth(1)).toHaveAttribute("aria-selected", "true");
    await searchBox.press("Enter");

    await expect(page.getByRole("heading", { level: 2, name: secondOptionLabel })).toBeVisible();
    await expect(page.getByRole("listbox")).toHaveCount(0);
  });
});
