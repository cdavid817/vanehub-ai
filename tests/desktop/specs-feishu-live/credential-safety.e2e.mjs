import assert from "node:assert/strict";
import { bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  qualifyLiveScenario as qualify,
  recordLiveScenario as record,
  safeLiveFailureCode as safeFailureCode,
  visibleUiSafeErrorCode,
} from "../helpers/feishu-live.mjs";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

async function navigate(target) {
  await globalThis.browser.execute((path) => {
    globalThis.history.pushState({}, "", path);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, target);
}

async function feishuView() {
  const connectors = await invoke(({ core }) => core.invoke("list_im_connectors"));
  return connectors.find((connector) => connector.descriptor.kind === "feishu");
}

async function openFeishuSettings() {
  await navigate("/settings?section=im");
  const section = await globalThis.$('[data-connector="feishu"]');
  await section.waitForDisplayed({ timeout: 60_000 });
  const disclosure = await section.$('button[aria-expanded="false"]');
  if (await disclosure.isExisting()) await disclosure.click();
  return section;
}

async function enterCredentials(section, appId, appSecret) {
  const appIdInput = await section.$('input[type="text"]');
  const appSecretInput = await section.$('input[type="password"]');
  await appIdInput.setValue(appId);
  await appSecretInput.setValue(appSecret);
  const save = await section.$('.//button[.//*[contains(@class,"lucide-save")]]');
  await save.waitForEnabled({ timeout: 20_000 });
  await save.click();
}

async function clickConnectorToggle(selected) {
  const section = await globalThis.$('[data-connector="feishu"]');
  const toggle = await section.$('input[type="checkbox"]');
  await globalThis.browser.waitUntil(async () => (
    await toggle.isEnabled() && await toggle.isSelected() === selected
  ), { timeout: 30_000, timeoutMsg: "the Feishu lifecycle toggle was not ready" });
  await toggle.click();
}

async function clickConnectorAction(actionName) {
  const section = await globalThis.$('[data-connector="feishu"]');
  const action = await section.$(`[data-im-action="${actionName}"]`);
  await action.waitForEnabled({ timeout: 30_000 });
  await action.click();
}

async function waitForOperationSuccess() {
  await globalThis.browser.waitUntil(async () => {
    const safeErrorCode = await visibleUiSafeErrorCode();
    if (safeErrorCode) throw new Error(safeErrorCode);
    const notice = await globalThis.$('div[aria-live="polite"].ucd-status-success');
    return await notice.isExisting() && await notice.isDisplayed() && (await notice.getText()).length > 0;
  }, { timeout: 25_000, interval: 250, timeoutMsg: "the connector operation did not complete" });
}

async function waitForLifecycle(expected, previousUpdatedAt) {
  return globalThis.browser.waitUntil(async () => {
    const safeErrorCode = await visibleUiSafeErrorCode();
    if (safeErrorCode) throw new Error(safeErrorCode);
    const view = await feishuView();
    if (view?.health.lifecycle === "failed") {
      throw new Error(view.health.safeErrorCode ?? "connector-lifecycle-failed");
    }
    return view?.health.lifecycle === expected
      && (!previousUpdatedAt || view.health.updatedAt !== previousUpdatedAt)
      ? view
      : false;
  }, { timeout: 60_000, interval: 500, timeoutMsg: `Feishu did not reach ${expected}.` });
}

globalThis.describe("VaneHub AI live Feishu credential safety", () => {
  globalThis.it("qualifies credentials and connector lifecycle without retaining secrets", async function liveQualification() {
    this.timeout(300_000);
    const appId = globalThis.process.env.VANEHUB_FEISHU_APP_ID;
    const appSecret = globalThis.process.env.VANEHUB_FEISHU_APP_SECRET;
    const runId = globalThis.process.env.VANEHUB_TEST_RUN_ID;
    assert.ok(appId && appSecret && runId, "live credential prerequisites disappeared before UI entry");

    await bootDesktopUi();
    const section = await openFeishuSettings();
    await enterCredentials(section, appId, appSecret);

    const expectedReference = `feishu/desktop-live-${runId}`.toLowerCase();
    await qualify("credential-isolation", async () => {
      await globalThis.browser.waitUntil(async () => {
        const feishu = await feishuView();
        return feishu?.hasCredentials && feishu.config.credentialRef === expectedReference;
      }, { timeout: 30_000, timeoutMsg: "the settings form did not create a run-owned credential" });
      await globalThis.browser.waitUntil(async () => {
        const currentSection = await globalThis.$('[data-connector="feishu"]');
        return await (await currentSection.$('input[type="password"]')).getValue() === "";
      }, { timeout: 20_000, timeoutMsg: "the write-only secret remained in the form" });
    });

    await qualify("authentication", async () => {
      await clickConnectorAction("test");
      await waitForOperationSuccess();
    });
    // Feishu can take a short interval to release the just-tested WebSocket before accepting the
    // long-lived socket for the same application identity.
    await globalThis.browser.pause(3_000);

    try {
      await clickConnectorToggle(false);
      await waitForLifecycle("connected");
      await record("connection-start", "PASSED");
    } catch (reason) {
      const failed = await feishuView();
      const safeErrorCode = failed?.health.safeErrorCode
        ? safeFailureCode(failed.health.safeErrorCode)
        : safeFailureCode(reason);
      await record("connection-start", "FAILED", safeErrorCode);
      throw new Error(`connection start failed (${safeErrorCode})`, { cause: reason });
    }

    const restarted = await qualify("connection-restart", async () => {
      const beforeRestart = await feishuView();
      await clickConnectorAction("restart");
      return waitForLifecycle("connected", beforeRestart?.health.updatedAt);
    });

    await qualify("disable-re-enable", async () => {
      await clickConnectorToggle(true);
      const disabled = await waitForLifecycle("disabled", restarted.health.updatedAt);
      await clickConnectorToggle(false);
      await waitForLifecycle("connected", disabled.health.updatedAt);
    });

  });
});
