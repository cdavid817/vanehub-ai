import { expect, test, type Page } from "@playwright/test";

/** Keep in sync with `WEB_MOCK_QUESTION_TRIGGER` in `src/services/web-agent-client.ts`. */
const QUESTION_TRIGGER = "[ask-me]";

/**
 * The clarification card only exists in an API chat, and OnePiece is the only API-capable mock
 * Agent — it ships `unavailable` until a provider is configured, so a question test has to stand
 * one up first.
 */
async function createOnePieceChat(page: Page, title: string) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 配置|Agent Configurations)$/ }).click();
  await page.getByRole("button", { name: /OnePiece/ }).click();

  const panel = page.getByRole("region", { name: "OnePiece" });
  await panel.getByRole("button", { name: "新增配置" }).first().click();
  const dialog = page.getByRole("dialog", { name: "新增 OnePiece 配置" });
  await dialog.getByRole("button", { name: /Anthropic/ }).click();
  await dialog.getByLabel("配置名称").fill("提问测试配置");
  await dialog.getByLabel("模型", { exact: true }).selectOption("claude-sonnet-4-6");
  await dialog.getByLabel("API 密钥").fill("playwright-question-secret");
  await dialog.getByRole("button", { name: "保存 OnePiece" }).click();
  await expect(panel.getByText("已可用于本地会话")).toBeVisible();

  await page.getByRole("button", { name: "返回", exact: true }).click();
  await page.getByRole("button", { name: /新建/ }).click();
  const create = page.getByRole("dialog");
  await create.locator("button").filter({ hasText: "OnePiece" }).first().click();
  await create.getByPlaceholder(/code.*project/).fill("D:\\onepiece-workspace");
  await create.getByPlaceholder("新会话").fill(title);
  await create.getByRole("button", { name: "创建", exact: true }).click();

  return page.getByPlaceholder("输入指令，下发任务给当前 Agent...");
}

async function askQuestion(page: Page, title: string) {
  const composer = await createOnePieceChat(page, title);
  await composer.fill(`重构这个模块 ${QUESTION_TRIGGER}`);
  await page.getByRole("button", { name: "发送", exact: true }).click();

  // Scoped to the card: the question text also appears in the collapsed JSON detail dump.
  const card = page.getByRole("region", { name: "工具活动" }).getByTestId("tool-question");
  await expect(card.getByText("Which approach should the simulated agent take?")).toBeVisible();
  return card;
}

test.describe("agent clarification round trip", () => {
  test("presents the model's options rather than an approval control", async ({ page }) => {
    const card = await askQuestion(page, "澄清提问会话");

    const chosen = card.getByRole("button", { name: "Patch it in place" });
    await expect(chosen).toBeVisible();
    await expect(card.getByRole("button", { name: "Rewrite the module" })).toBeVisible();
    // The approval affordance belongs to a different status and must not appear here.
    await expect(card.getByText("该工具调用需要你的确认才能执行")).toHaveCount(0);

    await chosen.click();
    await expect(card.getByRole("button", { name: "Patch it in place" })).toHaveCount(0);
  });

  test("accepts an answer the offered options do not cover", async ({ page }) => {
    const card = await askQuestion(page, "自由文本回答会话");

    const freeText = card.getByRole("textbox", { name: "用你自己的话回答" });
    await expect(freeText).toBeVisible();
    await freeText.fill("都不是，先补测试");
    await card.getByRole("button", { name: "发送", exact: true }).click();

    await expect(card.getByRole("button", { name: "Patch it in place" })).toHaveCount(0);
  });
});
