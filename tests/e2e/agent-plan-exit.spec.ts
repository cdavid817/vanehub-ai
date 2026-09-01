import { expect, test, type Page } from "@playwright/test";

/** Keep in sync with `WEB_MOCK_PLAN_EXIT_TRIGGER` in `src/services/web-agent-client.ts`. */
const PLAN_EXIT_TRIGGER = "[plan-done]";

/**
 * Same setup as the clarification spec: the card only exists in an API chat, and OnePiece is the
 * only API-capable mock Agent, which ships `unavailable` until a provider is configured.
 */
async function createOnePieceChat(page: Page, title: string) {
  await page.goto("/", { waitUntil: "domcontentloaded" });
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("button", { name: /OnePiece/ }).click();

  const panel = page.getByRole("region", { name: "OnePiece" });
  await panel.getByRole("button", { name: "新增配置" }).first().click();
  const dialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
  await dialog.getByRole("button", { name: /Anthropic/ }).click();
  await dialog.getByLabel("配置名称").fill("计划批准测试配置");
  await dialog.getByLabel("模型", { exact: true }).selectOption("claude-sonnet-4-6");
  await dialog.getByLabel("API 密钥").fill("playwright-plan-exit-secret");
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

async function requestPlanExit(page: Page, title: string) {
  const composer = await createOnePieceChat(page, title);
  await composer.fill(`把这个模块理清楚 ${PLAN_EXIT_TRIGGER}`);
  await page.getByRole("button", { name: "发送", exact: true }).click();

  // Scoped to the card: the plan text also appears in the collapsed JSON detail dump.
  const card = page.getByRole("region", { name: "工具活动" }).getByTestId("tool-plan-exit");
  await expect(card.getByText("Rewrite the parser, then update its three callers.")).toBeVisible();
  return card;
}

test.describe("agent plan exit request", () => {
  test.describe.configure({ timeout: 120_000 });

  test("keeps Plan mode inside the OnePiece session composer", async ({ page }) => {
    await createOnePieceChat(page, "会话栏计划模式");

    // Execution mode now lives in the advanced Run configuration popover (10.16), not directly
    // on the toolbar.
    await page.getByTestId("composer-config-trigger").click();
    const mode = page.getByRole("button", { name: /继承|计划 · 只读|Agent · 可写/ });
    await mode.click();
    await page.getByRole("menuitemradio", { name: /计划 · 只读/ }).click();
    await expect(mode).toHaveAttribute("title", "运行模式：计划 · 只读");

    await expect(page.getByRole("button", { name: "Plan 执行" })).toHaveCount(0);
  });

  // Both tools block as `awaiting_input`, and the card is chosen by tool name. Rendering the
  // question card here would parse no options and show no controls at all, leaving the request
  // unanswerable — which unit and type checks would both have called green.
  test("shows approve and decline rather than the clarification controls", async ({ page }) => {
    const card = await requestPlanExit(page, "计划批准会话");

    await expect(card.getByRole("button", { name: "批准并开始" })).toBeVisible();
    await expect(card.getByRole("button", { name: "继续规划" })).toBeVisible();
    // Affordances belonging to the other two awaiting states must not appear here.
    await expect(card.getByRole("textbox", { name: "用你自己的话回答" })).toHaveCount(0);
    await expect(card.getByText("该工具调用需要你的确认才能执行")).toHaveCount(0);
  });

  test("resolves the request when the plan is approved", async ({ page }) => {
    const card = await requestPlanExit(page, "计划批准通过会话");

    await card.getByRole("button", { name: "批准并开始" }).click();
    await expect(card.getByRole("button", { name: "批准并开始" })).toHaveCount(0);
  });

  test("resolves the request when the plan is declined", async ({ page }) => {
    const card = await requestPlanExit(page, "计划继续规划会话");

    await card.getByRole("button", { name: "继续规划" }).click();
    await expect(card.getByRole("button", { name: "继续规划" })).toHaveCount(0);
  });
});
