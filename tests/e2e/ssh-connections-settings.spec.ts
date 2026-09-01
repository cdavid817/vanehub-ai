import { expect, test } from "@playwright/test";

test.describe("SSH Connections settings", () => {
  test("add-connection dialog is a real dialog, defers validation until submit, and closes on Escape", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "SSH 连接" }).click();

    await page.getByRole("button", { name: "新增连接" }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("textbox", { name: /^名称/ })).toBeFocused();
    await expect(dialog.getByText("请输入连接名称。")).toHaveCount(0);

    await dialog.getByRole("button", { name: "保存" }).click();
    await expect(dialog.getByText("请输入连接名称。")).toBeVisible();

    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
  });

  test("creating and testing a connection shows a color-coded status badge", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "SSH 连接" }).click();

    await page.getByRole("button", { name: "新增连接" }).click();
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("textbox", { name: /^名称/ }).fill("回归测试服务器");
    await dialog.getByRole("textbox", { name: /^主机/ }).fill("10.0.0.5");
    await dialog.getByRole("textbox", { name: /^用户/ }).fill("deploy");
    await dialog.getByRole("textbox", { name: /^默认路径/ }).fill("/srv/app");
    await dialog.locator("select").selectOption("key");
    await dialog.getByRole("textbox", { name: /^密钥路径/ }).fill("~/.ssh/id_ed25519");
    await dialog.getByRole("button", { name: "保存" }).click();
    await expect(dialog).toBeHidden();

    const card = page.locator("article", { hasText: "回归测试服务器" });
    const notTestedBadge = card.getByText("未测试", { exact: true });
    await expect(notTestedBadge).toBeVisible();
    await expect(notTestedBadge).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");

    // Task 12.18: per-row actions moved behind a shared "..." ActionMenu.
    await card.getByRole("button", { name: /的操作$/ }).click();
    await card.getByRole("menuitem", { name: "测试" }).click();
    const succeededBadge = card.getByText("成功", { exact: true });
    await expect(succeededBadge).toBeVisible();
    await expect(succeededBadge).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  });
});
