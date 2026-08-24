import { expect, test } from "@playwright/test";

test("shows branded startup feedback before React is available", async ({ browser, baseURL }) => {
  const context = await browser.newContext({ baseURL, javaScriptEnabled: false });
  const page = await context.newPage();

  try {
    await page.goto("/");

    const shell = page.getByRole("status");
    await expect(shell).toBeVisible();
    await expect(shell.locator(".bootstrap-shell__logo")).toBeVisible();
    await expect(shell.locator(".bootstrap-shell__progress")).toBeVisible();
    await expect(shell.getByText("Starting...", { exact: true })).toBeVisible();
    await expect(shell).not.toContainText(/正在加载功能|Loading feature/);
  } finally {
    await context.close();
  }
});
