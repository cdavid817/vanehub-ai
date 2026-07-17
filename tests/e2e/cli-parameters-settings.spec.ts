import { expect, test } from "@playwright/test";

test.describe("CLI parameter settings", () => {
  test("saves a per-CLI parameter and restores it after reload", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(CLI 参数|CLI Parameters)$/).click();
    await page.getByRole("button", { name: "Codex CLI" }).click();
    await page.getByRole("combobox", { name: /沙箱|Sandbox/ }).selectOption("read-only");
    await expect(page.getByText(/--sandbox read-only/)).toBeVisible();
    await page.getByRole("button", { name: "Claude Code" }).click();
    await page.getByRole("button", { name: "Codex CLI" }).click();
    await expect(page.getByRole("combobox", { name: /沙箱|Sandbox/ })).toHaveValue("read-only");
    await page.getByRole("button", { name: /保存更改|Save changes/ }).click();
    await expect(page.getByText(/CLI 参数已保存|CLI parameters saved/)).toBeVisible();

    await page.reload();
    await page.getByText(/^(CLI 参数|CLI Parameters)$/).click();
    await page.getByRole("button", { name: "Codex CLI" }).click();
    await expect(page.getByRole("combobox", { name: /沙箱|Sandbox/ })).toHaveValue("read-only");

    page.once("dialog", (dialog) => dialog.accept());
    await page.getByRole("button", { name: /恢复默认值|Restore defaults/ }).click();
    await expect(page.getByRole("combobox", { name: /沙箱|Sandbox/ })).toHaveValue("default");
    await expect(page.getByRole("button", { name: /保存更改|Save changes/ })).toBeDisabled();

    await page.evaluate(() => {
      localStorage.setItem(
        "vanehub.cli-parameter-profiles.v1",
        JSON.stringify({ "codex-cli": { sandbox: "danger-full-access" } }),
      );
    });
    await page.reload();
    await page.getByText(/^(CLI 参数|CLI Parameters)$/).click();
    await expect(page.getByText(/参数 sandbox 的值无效|The value for parameter sandbox is invalid/)).toBeVisible();
  });

  test("supports English minimal theme at a narrow viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("combobox", { name: /应用语言|Application Language/ }).selectOption("en");
    await page.getByRole("combobox", { name: /主题|Theme/ }).selectOption("minimal");
    await page.getByText(/^CLI Parameters$/).click();

    await expect(page.getByRole("heading", { name: "CLI Parameter Management" })).toBeVisible();
    await expect(page.getByText("Safe argument preview")).toBeVisible();
    const codexButton = page.getByRole("button", { name: "Codex CLI" });
    await codexButton.focus();
    await expect(codexButton).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("combobox", { name: "Sandbox" })).toBeVisible();
    await page.getByRole("button", { name: "OpenCode" }).click();
    const automaticApproval = page.getByRole("switch", { name: "Automatic approval" });
    await expect(automaticApproval).toHaveAttribute("aria-checked", "false");
    await automaticApproval.focus();
    await page.keyboard.press("Space");
    await expect(automaticApproval).toHaveAttribute("aria-checked", "true");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    const layout = await page.evaluate(() => ({
      bodyOverflow: document.body.scrollWidth > window.innerWidth,
      foreground: getComputedStyle(document.body).color,
      background: getComputedStyle(document.body).backgroundColor,
    }));
    expect(layout.bodyOverflow).toBe(false);
    expect(layout.foreground).not.toBe(layout.background);
  });
});
