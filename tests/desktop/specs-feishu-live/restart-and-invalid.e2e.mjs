import assert from "node:assert/strict";
import { findSession, listMessages } from "../helpers/feishu-im.mjs";
import {
  operatorInstruction,
  qualifyLiveScenario as qualify,
  recordLiveScenario as record,
  waitForLiveFinalDelivery,
  waitForLiveNativeBridge,
} from "../helpers/feishu-live.mjs";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const operatorEnabled = globalThis.process.env.VANEHUB_FEISHU_LIVE_OPERATOR === "1";

async function feishuView() {
  const connectors = await invoke(({ core }) => core.invoke("list_im_connectors"));
  return connectors.find((connector) => connector.descriptor.kind === "feishu");
}

globalThis.describe("VaneHub AI live Feishu restart and invalid credential", () => {
  globalThis.it("restores the live connector and rejects an invalid credential", async function restartAndInvalid() {
    this.timeout(35 * 60_000);
    const appId = globalThis.process.env.VANEHUB_FEISHU_APP_ID;
    assert.ok(appId, "live App ID prerequisite disappeared");
    await waitForLiveNativeBridge();

    let restartFailure;
    if (operatorEnabled) {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      const operatorSession = sessions.find(({ title }) => title === "Feishu live multi Agent");
      if (!operatorSession) {
        await record("desktop-restart", "BLOCKED", "operator-phase-incomplete");
      } else {
        const reconnected = await globalThis.browser.waitUntil(async () => (
          (await feishuView())?.health.lifecycle === "connected"
        ), { timeout: 60_000, interval: 500, timeoutMsg: "Feishu did not reconnect after restart" })
          .then(() => true, () => false);
        if (!reconnected) {
          await record("desktop-restart", "BLOCKED", "connector-prerequisite-failed");
        } else try {
          await qualify("desktop-restart", async () => {
            const session = await findSession("Feishu live multi Agent");
            const before = (await listMessages(session.id)).length;
            operatorInstruction("桌面端已重启。请发送：VANEHUB_LIVE_RESTART_CHECK");
            const messages = await globalThis.browser.waitUntil(async () => {
              const rows = await listMessages(session.id);
              if (rows.length < before + 2) return false;
              const userIndex = rows.findIndex(({ role, content }) => (
                role === "user" && content === "VANEHUB_LIVE_RESTART_CHECK"
              ));
              if (userIndex < 0) return false;
              return rows.slice(userIndex + 1).some(({ role, status }) => (
                role === "assistant" && status === "completed"
              )) ? rows : false;
            }, { timeout: 30 * 60_000, interval: 1_000, timeoutMsg: "post-restart message timed out" });
            assert.ok(messages.some(({ role, content }) => (
              role === "user" && content === "VANEHUB_LIVE_RESTART_CHECK"
            )));
            const assistant = messages.findLast(({ role, status }) => (
              role === "assistant" && status === "completed"
            ));
            assert.ok(assistant, "post-restart Agent response did not complete");
            await waitForLiveFinalDelivery(session.id, assistant.id);
          });
        } catch (reason) {
          restartFailure = reason;
        }
      }
    }

    await invoke(({ core }) => core.invoke("clear_im_connector", { kind: "feishu" }));
    await qualify("invalid-credential", async () => {
      await invoke(({ core }, input) => core.invoke("save_im_connector", { input }), {
        kind: "feishu",
        enabled: false,
        displayName: null,
        publicConfig: {},
        credentials: { appId, appSecret: "vanehub-live-deliberately-invalid" },
      });
      const rejected = await invoke(async ({ core }, input) => {
        try {
          await core.invoke("test_im_connector", input);
          return false;
        } catch {
          return true;
        }
      }, { kind: "feishu" });
      assert.equal(rejected, true, "invalid Feishu credentials were accepted");
      assert.notEqual((await feishuView())?.health.lifecycle, "connected");
    });
    if (restartFailure) throw restartFailure;
  });
});
