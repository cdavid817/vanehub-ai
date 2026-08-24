import { expect, test, type Page } from "@playwright/test";

async function openCliParameters(page: Page) {
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await selectCliParametersPage(page);
}

// A reload keeps the settings view, so there is no Settings button to click a second time.
async function selectCliParametersPage(page: Page) {
  await page.getByText(/^(CLI 参数|CLI Parameters)$/).click();
}

test.describe("CLI parameter settings", () => {
  test("saves a per-CLI parameter and restores it after reload", async ({ page }) => {
    await page.goto("/");
    await openCliParameters(page);
    await page.getByRole("button", { name: /Codex CLI/ }).click();

    const reasoningEffort = page.getByRole("combobox", { name: /推理强度|Reasoning effort/ });
    await reasoningEffort.selectOption("high");

    // The preview is tokenized: the flag and its config assignment are separate argv entries, and
    // a joined command line is never shown.
    await expect(page.getByText("--config", { exact: true })).toBeVisible();
    await expect(page.getByText('model_reasoning_effort="high"', { exact: true })).toBeVisible();

    // Policy-governed parameters are not editable here, and the page says where they live.
    await expect(page.getByRole("combobox", { name: /沙箱|Sandbox/ })).toHaveCount(0);
    await expect(
      page.getByText(/沙箱、审批和自动批准由|Sandbox, approval, and auto-approve behavior/),
    ).toBeVisible();

    // Switching CLIs keeps the draft.
    await page.getByRole("button", { name: /Claude Code/ }).click();
    await page.getByRole("button", { name: /Codex CLI/ }).click();
    await expect(reasoningEffort).toHaveValue("high");

    await page.getByRole("button", { name: /保存更改|Save changes/ }).click();
    await expect(page.getByText(/CLI 参数已保存|CLI parameters saved/)).toBeVisible();

    await page.reload();
    await selectCliParametersPage(page);
    await page.getByRole("button", { name: /Codex CLI/ }).click();
    await expect(page.getByRole("combobox", { name: /推理强度|Reasoning effort/ })).toHaveValue(
      "high",
    );

    await page.getByRole("button", { name: /恢复为继承|Restore inherited values/ }).click();
    await page.getByRole("dialog").getByRole("button", { name: /^(确认|Confirm)$/ }).click();
    // Inheritance is its own value, not a provider option named "default".
    await expect(page.getByRole("combobox", { name: /推理强度|Reasoning effort/ })).toHaveValue(
      "__inherit__",
    );
  });

  test("refuses an empty custom value and keeps the draft out of transport", async ({ page }) => {
    await page.goto("/");
    await openCliParameters(page);

    const model = page.getByRole("combobox", { name: /^(模型|Model)$/ });
    await model.selectOption("__custom__");
    // Choosing Custom switched the editor and wrote nothing, so there is nothing to save.
    await expect(page.getByRole("button", { name: /保存更改|Save changes/ })).toBeDisabled();

    const custom = page.getByRole("textbox", { name: /输入您的 CLI 支持的模型标识符|Enter a model/ });
    await custom.fill("claude-opus-5");
    await expect(page.getByRole("button", { name: /保存更改|Save changes/ })).toBeEnabled();

    await custom.fill("");
    await expect(page.getByRole("button", { name: /保存更改|Save changes/ })).toBeDisabled();
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
    // OnePiece is linked rather than tabbed: it has no CLI and no argv.
    await expect(
      page.getByText("OnePiece retrieval parameters live on the Agent Configuration page."),
    ).toBeVisible();

    const codexButton = page.getByRole("button", { name: /Codex CLI/ });
    await codexButton.focus();
    await expect(codexButton).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.getByRole("combobox", { name: "Reasoning effort" })).toBeVisible();
    await expect(page.getByRole("combobox", { name: "Sandbox" })).toHaveCount(0);
    await expect(page.getByText(/Sandbox, approval, and auto-approve behavior/)).toBeVisible();

    await page.getByRole("button", { name: /OpenCode/ }).click();
    const thinking = page.getByRole("switch", { name: "Thinking output" });
    await expect(thinking).toHaveAttribute("aria-checked", "false");
    await thinking.focus();
    await page.keyboard.press("Space");
    await expect(thinking).toHaveAttribute("aria-checked", "true");
    await expect(page.getByRole("switch", { name: "Automatic approval" })).toHaveCount(0);

    // Scope is an explicit control, and switching it changes which fields apply.
    const scopes = page.getByRole("group", { name: "Launch scope" });
    await scopes.getByRole("button", { name: "Interactive" }).click();
    await expect(page.getByRole("switch", { name: "Thinking output" })).toHaveCount(0);

    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    const layout = await page.evaluate(() => ({
      bodyOverflow: document.body.scrollWidth > window.innerWidth,
    }));
    expect(layout.bodyOverflow).toBe(false);
  });
});
