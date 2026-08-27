import assert from "node:assert/strict";
import { bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  coreInvoke,
  createFeishuSession,
  openSessionIm,
  persistedSessionTitle,
} from "../helpers/feishu-im.mjs";

globalThis.describe("VaneHub AI desktop Feishu IM session access: enable", () => {
  globalThis.after(async () => {
    await coreInvoke("exit_application");
  });

  globalThis.it("defaults off, enables through the rendered panel, and persists a fixture binding", async function enable() {
    this.timeout(240_000);
    await bootDesktopUi();
    const session = await createFeishuSession(persistedSessionTitle);
    const initial = await coreInvoke("get_im_session_binding", { sessionId: session.id });
    assert.equal(initial.access.enabled, false, "native access did not default off");
    assert.equal(initial.binding, null, "a new session unexpectedly had an IM binding");

    // Session creation happens through native IPC, so reload once to make the rendered session
    // collection observe the new row before driving the information panel.
    await globalThis.browser.refresh();
    await bootDesktopUi();
    const accessSwitch = await openSessionIm(session.id);
    assert.equal(await accessSwitch.isSelected(), false, "a new session did not default IM access off");

    await accessSwitch.click();
    await globalThis.browser.waitUntil(async () => {
      const snapshot = await coreInvoke("get_im_session_binding", { sessionId: session.id });
      return snapshot.access.enabled;
    }, { timeout: 30_000, timeoutMsg: "The UI switch never reached native storage." });

    await coreInvoke("fixture_feishu_im_reset");
    const setup = await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });
    assert.deepEqual(setup, { ready: true, connector: "feishu" });
    const bound = await coreInvoke("get_im_session_binding", { sessionId: session.id });
    assert.equal(bound.access.enabled, true);
    assert.equal(bound.binding?.state, "active", "the fixture pairing did not persist an active binding");
  });
});
