import { expect, test, type Locator, type Page } from "@playwright/test";

/**
 * Slash commands are gated to native OnePiece sessions (`agentId === "onepiece"`), so every case
 * below needs one. The provider config is seeded directly through the web mock client instead of
 * the full Settings dialog flow, mirroring the lighter-weight session setup already proven in
 * onepiece-retrieval.spec.ts's second test rather than the full configuration UI walked in
 * onepiece-agent.spec.ts (whose subject is the configuration flow itself, not what runs after it).
 */
async function seedOnePieceProvider(page: Page) {
  await page.goto("/");
  await page.evaluate(async () => {
    const module = await import("/src/services/web-agent-client.ts");
    await module.webAgentClient.saveOnePieceProviderConfig({
      provider: "Anthropic",
      modelId: "claude-opus-4-8",
      interfaceFormat: "anthropic",
      baseUrl: null,
      apiKey: "playwright-local-only-key",
    });
  });
}

function agentButton(dialog: Locator, name: string) {
  return dialog.locator("button").filter({ hasText: name }).first();
}

async function createOnePieceSession(page: Page, title: string) {
  await seedOnePieceProvider(page);
  await page.getByRole("button", { name: /新建/ }).click();
  const dialog = page.getByRole("dialog");
  await agentButton(dialog, "OnePiece").click();
  await dialog.getByPlaceholder(/code.*project/).fill("D:\\example-workspace");
  await dialog.getByPlaceholder("新会话").fill(title);
  await dialog.getByRole("button", { name: "创建", exact: true }).click();
  await expect(page.getByTestId("wechat-style-composer")).toBeVisible();
}

test.describe("OnePiece slash commands", () => {
  test("/help lists commands without sending a message", async ({ page }) => {
    await createOnePieceSession(page, "斜杠命令帮助会话");
    const composer = page.getByTestId("wechat-style-composer");
    const input = composer.getByRole("textbox");

    await input.fill("/help");
    await input.press("Enter");

    const output = page.getByTestId("slash-command-output");
    await expect(output).toBeVisible();
    await expect(output).toContainText("/status");
    await expect(input).toHaveValue("");
    // A fresh session renders no message-scroll-region until it has at least one message
    // (MessageList falls back to WelcomeScreen for an empty list), so its absence is a
    // structural proof that /help never reached `model.submit()`.
    await expect(page.getByTestId("message-scroll-region")).toHaveCount(0);
  });

  test("an unknown command is reported and not sent", async ({ page }) => {
    await createOnePieceSession(page, "斜杠未知命令会话");
    const input = page.getByTestId("wechat-style-composer").getByRole("textbox");
    await input.fill("/definitelynotacommand");
    await input.press("Enter");

    const output = page.getByTestId("slash-command-output");
    await expect(output).toHaveAttribute("data-tone", "error");
    await expect(page.getByTestId("message-scroll-region")).toHaveCount(0);
  });

  test("typing a slash offers completions", async ({ page }) => {
    await createOnePieceSession(page, "斜杠补全会话");
    const input = page.getByTestId("wechat-style-composer").getByRole("textbox");
    await input.fill("/st");
    await expect(page.getByRole("button", { name: /\/status/ })).toBeVisible();
  });
});
