import assert from "node:assert/strict";
import { readNativeSettings } from "../helpers/native-settings.mjs";

globalThis.describe("VaneHub AI native desktop smoke", () => {
  globalThis.it("starts the real runtime, crosses IPC, and performs stable navigation", async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
      timeout: 120_000,
      timeoutMsg: "React bootstrap did not become ready.",
    });

    const settings = await readNativeSettings();
    assert.match(settings.applicationLanguage, /^(zh-CN|zh-TW|en|ja|ko)$/);

    const settingsButton = await globalThis.$('[data-testid="desktop-smoke-settings"]');
    await settingsButton.waitForClickable();
    await settingsButton.click();
    await globalThis.browser.waitUntil(async () => (await globalThis.browser.getUrl()).includes("/settings"), {
      timeoutMsg: "The native WebView did not navigate to settings.",
    });
    const settingsRoot = await globalThis.$("main");
    await settingsRoot.waitForExist();
    assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);

    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });
});
