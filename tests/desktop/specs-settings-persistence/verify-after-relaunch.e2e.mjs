import assert from "node:assert/strict";
import { assertNoFatalError, bootDesktopUi, CLI_TERMINAL_THEME_TARGET, FONT_SIZE_TARGET } from "../helpers/native-ui.mjs";

// Runs against a freshly launched application sharing the previous spec's application-data
// directory, so what is asserted here survived a real process restart rather than a re-render.
globalThis.describe("VaneHub AI desktop settings persistence: relaunch", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("restores the setting from native storage and renders it", async function () {
    this.timeout(240000);
    const root = await bootDesktopUi();

    const settings = await globalThis.browser.tauri.execute(({ core }) => core.invoke("get_settings"));
    assert.equal(settings.fontSize, FONT_SIZE_TARGET, "the setting did not survive the relaunch");

    const settingsButton = await globalThis.$('//button[@aria-label="设置"]');
    await settingsButton.waitForClickable({ timeout: 30000 });
    await settingsButton.click();

    const fontSize = await globalThis.$('//select[@aria-label="字体大小"]');
    await fontSize.waitForExist({ timeout: 30000 });
    assert.equal(await fontSize.getValue(), FONT_SIZE_TARGET, "the restored setting is not what the UI presents");

    assert.equal(settings.cliTerminalTheme, CLI_TERMINAL_THEME_TARGET, "the CLI terminal theme did not survive the relaunch");
    const cliTheme = await globalThis.$('//select[@aria-label="CLI 会话主题"]');
    await cliTheme.waitForExist({ timeout: 30000 });
    assert.equal(await cliTheme.getValue(), CLI_TERMINAL_THEME_TARGET, "the restored CLI terminal theme is not what the UI presents");
    const rootAttribute = await globalThis.browser.execute(() => globalThis.document.documentElement.dataset.cliTerminalTheme ?? null);
    assert.equal(rootAttribute, CLI_TERMINAL_THEME_TARGET, "the restored CLI terminal theme was not applied to the document");

    await assertNoFatalError(root);
  });
});
