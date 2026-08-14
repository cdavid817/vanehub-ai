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
    const composer = page.getByPlaceholder("输入指令，下发任务给当前 Agent...");
    await expect(composer).toBeVisible();
    await expect(page.getByLabel("Agent CLI 工作区")).toHaveCount(0);
    await expect(page.getByTestId("effective-execution-policy")).toContainText("危险操作需要审批");

    await page.getByTitle("运行模式：继承").click();
    await page.getByRole("menuitemradio", { name: /计划.*只读/ }).click();
    await expect(page.getByTitle(/运行模式：计划.*只读/)).toBeVisible();
    await expect(page.getByTestId("effective-execution-policy")).toContainText("最终行为：只读");

    await page.getByTitle(/运行模式：计划.*只读/).click();
    await page.getByRole("menuitemradio", { name: /继承.*Agent 权限策略/ }).click();
    await expect(page.getByTestId("effective-execution-policy")).toContainText("危险操作需要审批");

    await composer.fill("检查项目并总结当前状态");
    await page.getByRole("button", { name: "发送", exact: true }).click();
    const toolActivity = page.getByRole("region", { name: "工具活动" });
    await expect(toolActivity).toBeVisible();
    await expect(toolActivity.getByText(/待确认 \d+/)).toBeVisible();
    await expect(toolActivity.getByText(/^完成 \d+$/)).toBeVisible();
    await expect(toolActivity.getByText("该工具调用需要你的确认才能执行").first()).toBeVisible();
    await expect(toolActivity.getByText("Shell 命令").first()).toBeVisible();
    await expect(toolActivity.getByText("echo mock").first()).toBeVisible();

    const completedHistory = toolActivity.getByTestId("completed-tool-history");
    await expect(completedHistory).not.toHaveAttribute("open", "");
    await completedHistory.locator(":scope > summary").click();
    await expect(completedHistory).toHaveAttribute("open", "");
    await expect(completedHistory.getByText("读取文件")).toBeVisible();

    await toolActivity.getByRole("button", { name: "拒绝", exact: true }).first().click();
    await expect(toolActivity.getByText(/^失败 1$/)).toBeVisible();
    const failedHistory = toolActivity.getByTestId("failed-tool-history");
    await expect(failedHistory).not.toHaveAttribute("open", "");
    await expect(failedHistory.locator(":scope > summary")).toContainText("Shell 命令");
    await expect(failedHistory.locator(":scope > summary")).toContainText("echo mock");
    await failedHistory.locator(":scope > summary").click();
    await expect(failedHistory).toHaveAttribute("open", "");
    await expect(failedHistory.getByText("失败", { exact: true })).toBeVisible();

    await toolActivity.getByRole("button", { name: "拒绝", exact: true }).click();
    const activityToggle = toolActivity.getByRole("button", { name: "展开工具活动" });
    await expect(activityToggle).toHaveAttribute("aria-expanded", "false");
    await expect(toolActivity.getByText(/^失败 2$/)).toBeVisible();
    await expect(toolActivity.getByTestId("tool-activity-content")).toBeHidden();
    await activityToggle.click();
    await expect(toolActivity.getByTestId("tool-activity-content")).toBeVisible();

    const feedback = page.getByTestId("message-feedback-controls").last();
    await expect(feedback).toBeVisible();
    await feedback.getByRole("button", { name: "有帮助", exact: true }).click();
    await expect(feedback.getByRole("button", { name: "有帮助", exact: true })).toHaveAttribute("aria-pressed", "true");
    await feedback.getByRole("button", { name: "提出纠正" }).click();
    await feedback.getByLabel("需要纠正什么？").fill("请先概括风险，再请求工具审批。");
    page.once("dialog", (dialog) => dialog.accept());
    await feedback.getByRole("button", { name: "保存" }).click();
    await expect(feedback.getByRole("button", { name: "提出纠正" })).toHaveAttribute("aria-pressed", "true");
    page.once("dialog", (dialog) => dialog.accept());
    await feedback.getByRole("button", { name: "清除反馈" }).click();
    await expect(feedback.getByRole("button", { name: "清除反馈" })).toHaveCount(0);
  });

  test("keeps Agent Configuration free of registered-Agent management and all built-in CLIs selectable", async ({ page }) => {
    await openAgentConfigurations(page);
    await expect(page.getByRole("heading", { name: "已注册 Agent" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "注册 API Agent" })).toHaveCount(0);
    const expectedTabs = [
      "Claude Code",
      "Codex CLI",
      "OpenCode",
      "Antigravity CLI",
      "Gemini CLI",
      "OnePiece",
    ];
    const tabs = page.getByRole("tablist").getByRole("tab");
    for (let index = 0; index < expectedTabs.length; index += 1) {
      await expect(tabs.nth(index)).toContainText(expectedTabs[index]);
    }

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog.getByText("内置 CLI")).toBeVisible();
    const cliNames = ["Claude Code", "Codex CLI", "OpenCode", "Antigravity", "Gemini CLI"];
    for (const name of cliNames) {
      const button = agentButton(dialog, name);
      await expect(button).toBeVisible();
      await expect(button).not.toHaveAttribute("aria-disabled", "true");
    }
    const renderedCliNames = await dialog.locator("button").evaluateAll((buttons, expected) => (
      buttons.flatMap((button) => {
        const text = button.textContent ?? "";
        return expected.filter((name) => text.includes(name));
      })
    ), cliNames);
    expect(renderedCliNames).toEqual(cliNames);
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
