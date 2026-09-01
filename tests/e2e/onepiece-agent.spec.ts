import { expect, test, type Locator, type Page } from "@playwright/test";

async function openAgentConfigurations(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("button", { name: /OnePiece/ }).click();
  await expect(page.getByRole("region", { name: "OnePiece" }).getByRole("heading", { name: /^(API 提供商|API providers)$/i })).toBeVisible();
}

function agentButton(dialog: Locator, name: string) {
  return dialog.locator("button").filter({ hasText: name }).first();
}

test.describe("OnePiece native Agent", () => {
  test("configures OnePiece and creates a local API chat without an Agent Terminal", async ({ page }) => {
    await openAgentConfigurations(page);
    const onepiecePanel = page.getByRole("region", { name: "OnePiece" });
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
    // Task 11.3-11.7's 4-step wizard: Step 1 (mode) -> Step 2 (Agent identity, the card lives
    // here now) -> Step 3 (workspace) -> Step 4 (review + name).
    const dialog = page.getByRole("dialog");
    const dialogNext = dialog.getByRole("button", { name: "下一步" });
    await dialogNext.click();
    await agentButton(dialog, "OnePiece").click();
    // The card's own pressed state is the selection signal; the separate "selected agent"
    // echo row it used to assert duplicated the already-highlighted card.
    await expect(agentButton(dialog, "OnePiece")).toHaveAttribute("aria-pressed", "true");
    await dialogNext.click();
    const dialogProjectPath = dialog.getByPlaceholder(/code.*project/);
    await dialogProjectPath.fill("D:\\onepiece-workspace");
    await dialogProjectPath.press("Tab");
    await expect(dialogNext).toBeEnabled({ timeout: 10_000 });
    await dialogNext.click();
    await dialog.getByPlaceholder("新会话").fill("OnePiece API 会话");
    await dialog.getByRole("button", { name: "创建", exact: true }).click();

    const conversationHeader = page.getByTestId("session-conversation-header");
    await expect(conversationHeader.getByText("OnePiece", { exact: true })).toHaveCount(0);
    await expect(conversationHeader.getByText("onepiece", { exact: true })).toHaveCount(0);
    // The subtitle renders the localized interaction mode, not the raw enum value.
    await expect(conversationHeader.getByText("API", { exact: true })).toBeVisible();
    // The inspector is closed by default now (workbench-layout-preferences.ts), and at this
    // content width it hosts in a Sheet rather than inline — its own close button is dismissed
    // afterward, or the Sheet's backdrop would intercept every click for the rest of this test.
    await page.getByTestId("conversation-overflow-trigger").click();
    await page.getByTestId("toggle-info-panel").click();
    const inspector = page.getByTestId("workbench-inspector");
    await expect(inspector.getByText("OnePiece", { exact: true }).first()).toBeVisible();
    await inspector.getByRole("button", { name: "关闭", exact: true }).click();
    const composer = page.getByPlaceholder("输入指令，下发任务给当前 Agent...");
    await expect(composer).toBeVisible();
    await expect(page.getByLabel("Agent CLI 工作区")).toHaveCount(0);
    // The closed-state summary keeps showing effective policy on its own (10.15/10.18), but the
    // Execution mode control that changes it now lives inside the advanced Run configuration
    // popover (10.16) rather than directly on the toolbar.
    await expect(page.getByTestId("effective-execution-policy")).toContainText("危险操作需要审批");

    await page.getByTestId("composer-config-trigger").click();
    await page.getByTitle("运行模式：继承").click();
    await page.getByRole("menuitemradio", { name: /计划.*只读/ }).click();
    await expect(page.getByTitle(/运行模式：计划.*只读/)).toBeVisible();
    await expect(page.getByTestId("effective-execution-policy")).toContainText("最终行为：只读");

    await page.getByTitle(/运行模式：计划.*只读/).click();
    await page.getByRole("menuitemradio", { name: /继承.*Agent 权限策略/ }).click();
    await expect(page.getByTestId("effective-execution-policy")).toContainText("危险操作需要审批");
    // Close the popover before typing: it opens over the composer (same upward placement the
    // individual dropdowns it replaced already used), and would otherwise intercept the fill.
    await page.keyboard.press("Escape");
    await expect(page.getByTestId("composer-config-popover")).toHaveCount(0);

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
    // Replacing an existing rating confirms through an in-application dialog now.
    await feedback.getByRole("button", { name: "保存" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "确认" }).click();
    await expect(feedback.getByRole("button", { name: "提出纠正" })).toHaveAttribute("aria-pressed", "true");
    await feedback.getByRole("button", { name: "清除反馈" }).click();
    await page.getByRole("dialog").getByRole("button", { name: "确认" }).click();
    await expect(feedback.getByRole("button", { name: "清除反馈" })).toHaveCount(0);
  });

  test("keeps Agent Configuration free of registered-Agent management and all built-in CLIs selectable", async ({ page }) => {
    await openAgentConfigurations(page);
    await expect(page.getByRole("heading", { name: "已注册 Agent" })).toHaveCount(0);
    await expect(page.getByRole("heading", { name: "注册 API Agent" })).toHaveCount(0);
    const expectedTargets = [
      "Claude Code",
      "Codex CLI",
      "OpenCode",
      "Antigravity CLI",
      "Gemini CLI",
      "OnePiece",
    ];
    const targets = page.getByRole("navigation", { name: "配置目标 Agent" }).getByRole("button");
    for (let index = 0; index < expectedTargets.length; index += 1) {
      await expect(targets.nth(index)).toContainText(expectedTargets[index]);
    }

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    // Task 11.3-11.7's 4-step wizard: Agent identity (this test's own subject) is Step 2 now.
    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "下一步" }).click();
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
    const onepiecePanel = page.getByRole("region", { name: "OnePiece" });
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

/**
 * 10.22: streaming/stop/focus/touch/high-risk coverage. All five only exist in an API chat --
 * `usesStructuredChat` in `main-layout.tsx` routes the "work" surface to `ChatTab`/`ButtonArea`
 * (the header Stop button, the message selection affordance, the closed-state risk summary) only
 * for `interactionMode === "api"` or a multi-seat session -- so, like `agent-plan-exit.spec.ts`,
 * this reuses OnePiece rather than a CLI session, whose "work" surface is the plain Agent Terminal.
 */
async function createOnePieceChat(page: Page, title: string) {
  await openAgentConfigurations(page);
  const panel = page.getByRole("region", { name: "OnePiece" });
  await panel.getByRole("button", { name: "新增配置" }).first().click();
  const dialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
  await dialog.getByRole("button", { name: /Anthropic/ }).click();
  await dialog.getByLabel("配置名称").fill("会话行为测试配置");
  await dialog.getByLabel("模型", { exact: true }).selectOption("claude-sonnet-4-6");
  await dialog.getByLabel("API 密钥").fill("playwright-behavior-secret");
  await dialog.getByRole("button", { name: "保存 OnePiece" }).click();
  await expect(panel.getByText("已可用于本地会话")).toBeVisible();

  await page.getByRole("button", { name: "返回", exact: true }).click();
  await page.getByRole("button", { name: /新建/ }).click();
  // Task 11.3-11.7's 4-step wizard: Step 1 (mode, single/CLI/local defaults are fine here) ->
  // Step 2 (Agent identity, OnePiece chosen here) -> Step 3 (workspace) -> Step 4 (review + name).
  const create = page.getByRole("dialog");
  const nextButton = create.getByRole("button", { name: "下一步" });
  await nextButton.click();
  await create.locator("button").filter({ hasText: "OnePiece" }).first().click();
  await nextButton.click();
  const projectPath = create.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\onepiece-workspace");
  await projectPath.press("Tab");
  await expect(nextButton).toBeEnabled({ timeout: 10_000 });
  await nextButton.click();
  await create.getByPlaceholder("新会话").fill(title);
  await create.getByRole("button", { name: "创建", exact: true }).click();

  return page.getByPlaceholder("输入指令，下发任务给当前 Agent...");
}

test.describe("OnePiece composer streaming, stop, focus, touch, and high-risk affordances (10.22)", () => {
  test.describe.configure({ timeout: 120_000 });

  test("shows a live streaming status and stops generation from the conversation header's own Stop button", async ({ page }) => {
    const composer = await createOnePieceChat(page, "流式与停止会话");
    // A long echoed message stretches the mock's own token schedule (`web-send-message-response-scheduler.ts`
    // fires `token` events every 90ms and `completed` at 320ms + tokenCount * 90ms) so there is a
    // multi-second window with the message genuinely mid-stream, rather than racing a ~1.6s reply.
    await composer.fill("检查当前会话状态".repeat(25));
    await page.getByRole("button", { name: "发送", exact: true }).click();

    const header = page.getByTestId("session-conversation-header");
    // Exactly two `<article>` rows exist after one send in a fresh session (user, then assistant) --
    // `MessageItem.tsx` is the only chat component that renders one, confirmed by reading the tree.
    const assistantArticle = page.locator("article").last();
    await expect(assistantArticle.getByText("生成中", { exact: true })).toBeVisible();
    await expect(header.getByText("生成中", { exact: true })).toBeVisible();
    // The header's Stop button sets an explicit `aria-label` of `chat.stopTitle` ("停止生成"),
    // distinct from the composer's own Stop button (accessible name "停止" from its text content) --
    // both render at once while streaming, so the header's more specific name disambiguates them.
    const headerStop = header.getByRole("button", { name: "停止生成", exact: true });
    await expect(headerStop).toBeVisible();

    await headerStop.click();

    await expect(headerStop).toHaveCount(0);
    await expect(header.getByText("已停止", { exact: true })).toBeVisible();
    await expect(assistantArticle.getByText("已停止", { exact: true })).toBeVisible();
  });

  test("selects a message via keyboard focus and Enter, the same as a click would", async ({ page }) => {
    const composer = await createOnePieceChat(page, "键盘选择会话");
    await composer.fill("键盘选择测试消息");
    await page.getByRole("button", { name: "发送", exact: true }).click();

    const userArticle = page.locator("article").first();
    const userBubble = userArticle.getByTestId("message-bubble");
    await expect(userBubble).toBeVisible();
    await expect(userBubble).not.toHaveAttribute("aria-current", "true");

    await userBubble.focus();
    await expect(userBubble).toBeFocused();
    await page.keyboard.press("Enter");

    await expect(userBubble).toHaveAttribute("aria-current", "true");
    // The "Selected" badge lives in the message's header row, a sibling of the bubble within the
    // same <article> (MessageItem.tsx) -- not a descendant of the bubble itself.
    await expect(userArticle.getByTestId("message-selected-indicator")).toBeVisible();
  });

  test.describe("touch path", () => {
    test.use({ hasTouch: true });

    test("selects a message bubble with a tap, the same as a click would", async ({ page }) => {
      const composer = await createOnePieceChat(page, "触屏选择会话");
      await composer.fill("触屏选择测试消息");
      await page.getByRole("button", { name: "发送", exact: true }).click();

      const userArticle = page.locator("article").first();
      const userBubble = userArticle.getByTestId("message-bubble");
      await expect(userBubble).toBeVisible();
      await expect(userBubble).not.toHaveAttribute("aria-current", "true");

      // Tap the message text itself, not the bubble wrapper: the wrapper's bounding box also
      // spans the always-visible MessageMemoryMenu row beneath the text (MessageMemoryMenu.tsx),
      // and `.tap()` synthesizes its click at the element's geometric center -- for a short
      // one-line message that center lands on the memory menu, not the text, the same way a real
      // fingertip would land wherever it actually touches rather than the container's midpoint.
      await userBubble.getByText("触屏选择测试消息").tap();

      await expect(userBubble).toHaveAttribute("aria-current", "true");
      await expect(userArticle.getByTestId("message-selected-indicator")).toBeVisible();
    });
  });

  test("keeps the high-risk warning visible in the closed summary once an agent's policy allows automatic execution", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: "Agent 权限策略", exact: true }).click();

    const onepieceRow = page.locator("div.grid.min-h-18").filter({ has: page.getByText("OnePiece", { exact: true }) });
    await onepieceRow.getByRole("button", { name: "Yolo", exact: true }).click();
    const confirmDialog = page.getByRole("dialog", { name: "确认提升这个 Agent 的信任等级?" });
    await confirmDialog.getByRole("button", { name: "确认" }).click();
    await expect(onepieceRow.getByRole("button", { name: "Yolo", exact: true })).toHaveAttribute("aria-pressed", "true");

    // Switches settings pages through the persistent sidebar (`settings-shell.tsx`'s `SettingsSidebar`),
    // not a fresh `page.goto` -- a real navigation would reload the app and wipe the policy
    // assignment just made, since the Web/mock permissions store is in-memory JS state, not persisted.
    await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
    await page.getByRole("button", { name: /OnePiece/ }).click();
    const panel = page.getByRole("region", { name: "OnePiece" });
    await panel.getByRole("button", { name: "新增配置" }).first().click();
    const providerDialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
    await providerDialog.getByRole("button", { name: /Anthropic/ }).click();
    await providerDialog.getByLabel("配置名称").fill("高风险策略测试配置");
    await providerDialog.getByLabel("模型", { exact: true }).selectOption("claude-sonnet-4-6");
    await providerDialog.getByLabel("API 密钥").fill("playwright-high-risk-secret");
    await providerDialog.getByRole("button", { name: "保存 OnePiece" }).click();
    await expect(panel.getByText("已可用于本地会话")).toBeVisible();

    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.getByRole("button", { name: /新建/ }).click();
    // Task 11.3-11.7's 4-step wizard — see createOnePieceChat's own comment above.
    const createDialog = page.getByRole("dialog");
    const createNextButton = createDialog.getByRole("button", { name: "下一步" });
    await createNextButton.click();
    await createDialog.locator("button").filter({ hasText: "OnePiece" }).first().click();
    await createNextButton.click();
    const createProjectPath = createDialog.getByPlaceholder(/code.*project/);
    await createProjectPath.fill("D:\\onepiece-workspace");
    await createProjectPath.press("Tab");
    await expect(createNextButton).toBeEnabled({ timeout: 10_000 });
    await createNextButton.click();
    await createDialog.getByPlaceholder("新会话").fill("高风险策略会话");
    await createDialog.getByRole("button", { name: "创建", exact: true }).click();

    const policySummary = page.getByTestId("effective-execution-policy");
    await expect(policySummary.getByText("高风险", { exact: true })).toBeVisible();
    await expect(policySummary).toContainText("允许自动执行");
  });
});
