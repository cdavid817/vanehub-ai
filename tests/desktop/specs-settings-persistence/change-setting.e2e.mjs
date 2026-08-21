import assert from "node:assert/strict";
import { assertNoFatalError, bootDesktopUi, FONT_SIZE_TARGET } from "../helpers/native-ui.mjs";

globalThis.describe("VaneHub AI desktop settings persistence: change", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("writes a UI-driven setting change through to native storage", async function () {
    this.timeout(240000);
    const root = await bootDesktopUi();

    const settingsButton = await globalThis.$('//button[@aria-label="设置"]');
    await settingsButton.waitForClickable({ timeout: 30000 });
    await settingsButton.click();

    const fontSize = await globalThis.$('//select[@aria-label="字体大小"]');
    await fontSize.waitForExist({ timeout: 30000 });
    const before = await globalThis.browser.tauri.execute(({ core }) => core.invoke("get_settings"));
    assert.notEqual(before.fontSize, FONT_SIZE_TARGET, "the fixture value must differ from the default");

    // Driven by dispatching the change event the control actually listens for, rather than through
    // WebDriver's select interaction: a native select popup is not reliably automatable across
    // WebKitGTK, WKWebView, and WebView2, and this layer has to pass on all three. Everything the
    // layer exists to verify — handler, service boundary, IPC, native storage, relaunch — stays real.
    const dispatched = await globalThis.browser.execute(
      (value) => {
        const select = globalThis.document.querySelector('select[aria-label="字体大小"]');
        if (!select) return false;
        select.value = value;
        select.dispatchEvent(new globalThis.Event("change", { bubbles: true }));
        return true;
      },
      FONT_SIZE_TARGET,
    );
    assert.ok(dispatched, "the font size control was not present to change");
    // The control is controlled, and React re-renders synchronously inside the discrete event, so
    // reading the element back here returns the pre-save value. Native storage is the only honest
    // signal that the change took, and it is what this layer is about anyway.

    // Asserted through the settings service rather than browser storage: the desktop client
    // persists natively, and a localStorage assertion cannot fail for a defect in that path.
    await globalThis.browser.waitUntil(async () => {
      const settings = await globalThis.browser.tauri.execute(({ core }) => core.invoke("get_settings"));
      return settings.fontSize === FONT_SIZE_TARGET;
    }, { timeout: 30000, timeoutMsg: "The changed setting never reached native storage." });

    await assertNoFatalError(root);
  });
});
