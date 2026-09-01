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

  test("never shows a saved password again, even to the form that collected it (task 12.20)", async ({ page }) => {
    const secret = "playwright-e2e-ssh-password";
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "SSH 连接" }).click();

    await page.getByRole("button", { name: "新增连接" }).click();
    const addDialog = page.getByRole("dialog");
    await addDialog.getByRole("textbox", { name: /^名称/ }).fill("密码认证服务器");
    await addDialog.getByRole("textbox", { name: /^主机/ }).fill("10.0.0.9");
    await addDialog.getByRole("textbox", { name: /^用户/ }).fill("deploy");
    await addDialog.getByRole("textbox", { name: /^默认路径/ }).fill("/srv/app");
    await addDialog.locator("select").selectOption("password");
    // `type="password"` has no implicit ARIA role, so `getByRole("textbox", ...)` cannot find it.
    // `exact` because the authMode <select>'s own accessible name also happens to end in "密码".
    await addDialog.getByLabel("密码", { exact: true }).fill(secret);
    await addDialog.getByRole("button", { name: "保存" }).click();
    await expect(addDialog).toBeHidden();
    await expect(page.locator("body")).not.toContainText(secret);

    const card = page.locator("article", { hasText: "密码认证服务器" });
    await card.getByRole("button", { name: /的操作$/ }).click();
    await card.getByRole("menuitem", { name: "编辑" }).click();
    const editDialog = page.getByRole("dialog");
    await expect(editDialog).toBeVisible();

    // The just-saved password is never re-populated -- only a "replace it" placeholder, never
    // the value itself, matching this session's own already-proven MCP env-var masking pattern.
    const passwordField = editDialog.getByLabel("替换密码");
    await expect(passwordField).toHaveValue("");
    await expect(passwordField).toHaveAttribute("placeholder", "已配置，仅在替换时输入新值");
    await expect(page.locator("body")).not.toContainText(secret);
  });
});
