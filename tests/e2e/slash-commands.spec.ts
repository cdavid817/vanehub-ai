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
  // Task 11.3-11.7's 4-step wizard: Step 1 (mode, single/CLI/local defaults are fine here) ->
  // Step 2 (Agent identity, OnePiece chosen here) -> Step 3 (workspace) -> Step 4 (review + name).
  const nextButton = dialog.getByRole("button", { name: "下一步" });
  await nextButton.click();
  await agentButton(dialog, "OnePiece").click();
  await nextButton.click();
  const projectPath = dialog.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\example-workspace");
  await projectPath.press("Tab");
  // Next only enables once the async project-path validation this same fill triggers settles.
  await expect(nextButton).toBeEnabled({ timeout: 10_000 });
  await nextButton.click();
  await dialog.getByPlaceholder("新会话").fill(title);
  await dialog.getByRole("button", { name: "创建", exact: true }).click();
  const composer = page.getByTestId("wechat-style-composer");
  const input = composer.getByRole("textbox");
  await expect(composer).toBeVisible();
  // A composer can remain visible while the new session is still becoming active. Wait for the
  // OnePiece-only completion surface so Enter dispatches against the new session rather than the
  // session that was active before the dialog closed.
  await input.fill("/");
  await expect(page.getByRole("button", { name: /\/help/ })).toBeVisible();
  await input.fill("");
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
    // Full rendered string, not just the invocation half: `SlashCommandOutput` resolves
    // `descriptionKey` as a nested key before interpolating (the one mechanism shared by all 22
    // help entries in all five languages), and a broken resolver would still contain "/status"
    // while rendering the raw key "slash.command.status.description" instead of this text. The
    // app's default language here is zh-CN (see the Chinese button labels used below), so this
    // asserts the zh-CN copy rather than assuming English.
    await expect(output).toContainText("/status — 显示当前运行时开关");
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

  // Navigation commands (`/logs` and eleven siblings) are wired through an optional
  // `navigation` prop on `ApiSessionComposer` (main-layout.tsx) rather than through anything
  // exercised by the composer's own unit tests, which render without that prop by design. If
  // `main-layout.tsx` stopped passing it, all twelve would silently become no-ops — still
  // listed, still offered, doing nothing — and no other test here would notice.
  test("/logs opens the logs workspace tab", async ({ page }) => {
    await createOnePieceSession(page, "斜杠导航命令会话");
    const input = page.getByTestId("wechat-style-composer").getByRole("textbox");
    await input.fill("/logs");
    await input.press("Enter");

    await expect(page.getByRole("tab", { name: "日志" })).toHaveAttribute("aria-selected", "true");
  });
});
