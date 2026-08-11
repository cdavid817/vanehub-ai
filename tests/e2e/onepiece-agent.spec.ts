import { expect, test, type Locator, type Page } from "@playwright/test";

async function openAgentConfigurations(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("tab", { name: /OnePiece/ }).click();
  await expect(page.getByRole("tabpanel", { name: "OnePiece" }).getByRole("heading", { name: /^(API 提供商|API providers)$/i })).toBeVisible();
}

function agentButton(dialog: Locator, name: string) {
  return dialog.locator("button").filter({ hasText: name }).first();
}

test.describe("OnePiece native Agent", () => {
  test("configures OnePiece and creates a local API chat without an Agent Terminal", async ({ page }) => {
    await openAgentConfigurations(page);
    const onepiecePanel = page.getByRole("tabpanel", { name: "OnePiece" });
    await onepiecePanel.getByRole("button", { name: "新增配置" }).first().click();
    const addDialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await addDialog.getByRole("button", { name: /Anthropic/ }).click();
    await addDialog.getByLabel("配置名称").fill("Anthropic 主账号");
    await addDialog.getByLabel("模型", { exact: true }).selectOption("claude-sonnet-4-6");
    await addDialog.getByLabel("API 密钥").fill("web-invalid");
    await addDialog.getByRole("button", { name: "验证 API 密钥" }).click();
    await expect(addDialog.getByText("API 密钥被厂商拒绝。")).toBeVisible();
    await addDialog.getByLabel("API 密钥").fill("not-persisted-playwright-secret");
    await addDialog.getByRole("button", { name: "保存 OnePiece" }).click();
    await expect(onepiecePanel.getByText("已可用于本地会话")).toBeVisible();
    await expect(onepiecePanel.getByRole("heading", { name: "Anthropic 主账号" })).toBeVisible();

    await onepiecePanel.getByRole("button", { name: "新增配置" }).first().click();
    const secondDialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await secondDialog.getByRole("button", { name: /OpenRouter/ }).click();
    await secondDialog.getByLabel("配置名称").fill("OpenRouter");
    await secondDialog.getByLabel("模型", { exact: true }).selectOption("anthropic/claude-sonnet-4.6");
    await secondDialog.getByLabel("API 密钥").fill("another-playwright-secret");
    await secondDialog.getByRole("button", { name: "保存 OnePiece" }).click();
    await expect(onepiecePanel.getByRole("heading", { name: "OpenRouter" })).toBeVisible();
    await onepiecePanel.getByRole("button", { name: "启用" }).click();
    const activateDialog = page.getByRole("dialog", { name: "启用提供商" });
    await activateDialog.getByRole("button", { name: "确认启用" }).click();
    await expect(onepiecePanel.getByText("OnePiece 提供商已启用。")).toBeVisible();

    const activeCard = onepiecePanel.getByRole("heading", { name: "OpenRouter" }).locator("xpath=ancestor::article[1]");
    await activeCard.getByRole("button", { name: "编辑配置" }).click();
    const editDialog = page.getByRole("dialog", { name: "编辑 OnePiece 配置" });
    await expect(editDialog.getByLabel("API 密钥")).toHaveValue("");
    await editDialog.getByLabel("模型", { exact: true }).selectOption("openai/gpt-5.4");
    await editDialog.getByRole("button", { name: "保存 OnePiece" }).click();
    await expect(onepiecePanel.getByText("openai/gpt-5.4")).toBeVisible();

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    const dialog = page.getByRole("dialog");
    await agentButton(dialog, "OnePiece").click();
    await expect(dialog.getByText(/当前选中：OnePiece|Selected Agent: OnePiece/)).toBeVisible();
    await dialog.getByPlaceholder(/code.*project/).fill("D:\\onepiece-workspace");
    await dialog.getByPlaceholder("新会话").fill("OnePiece API 会话");
    await dialog.getByRole("button", { name: "创建", exact: true }).click();

    const conversationHeader = page.getByTestId("session-conversation-header");
    await expect(conversationHeader.getByText("OnePiece", { exact: true })).toHaveCount(0);
    await expect(conversationHeader.getByText("onepiece", { exact: true })).toHaveCount(0);
    await expect(conversationHeader.getByText("api", { exact: true })).toBeVisible();
    await expect(page.getByTestId("info-pane-basic").getByText("OnePiece", { exact: true }).first()).toBeVisible();
    await expect(page.getByPlaceholder("输入指令，下发任务给当前 Agent...")).toBeVisible();
    await expect(page.getByLabel("Agent CLI 工作区")).toHaveCount(0);
  });

  test("keeps Agent Configuration free of registered-Agent management and all built-in CLIs selectable", async ({ page }) => {
    await openAgentConfigurations(page);
    await expect(page.getByRole("heading", { name: "已注册 Agent" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "注册 API Agent" })).toHaveCount(0);

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("内置 CLI")).toBeVisible();
    for (const name of ["Claude Code", "Gemini CLI", "Codex CLI", "OpenCode"]) {
      const button = agentButton(dialog, name);
      await expect(button).toBeVisible();
      await expect(button).not.toHaveAttribute("aria-disabled", "true");
    }
  });

  test("keeps the API provider dialog usable at narrow width", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openAgentConfigurations(page);
    const onepiecePanel = page.getByRole("tabpanel", { name: "OnePiece" });
    await onepiecePanel.getByRole("button", { name: "新增配置" }).first().click();
    const dialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await expect(dialog.getByLabel("搜索厂商")).toBeVisible();
    await expect(dialog.getByRole("button", { name: /Anthropic/ })).toBeVisible();
    await dialog.getByLabel("搜索厂商").fill("Mistral");
    const mistral = dialog.getByRole("button", { name: /Mistral AI/ });
    await expect(mistral).toBeVisible();
    await expect(mistral.getByRole("img", { name: "Mistral AI" })).toBeVisible();
    await expect(dialog.getByLabel("提供商")).toHaveCount(0);
    await expect(dialog.getByLabel("Base URL")).toHaveCount(0);
    await expect(dialog.getByRole("button", { name: "取消" })).toBeVisible();
    await expect(dialog.getByRole("button", { name: "保存 OnePiece" })).toBeVisible();
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});
