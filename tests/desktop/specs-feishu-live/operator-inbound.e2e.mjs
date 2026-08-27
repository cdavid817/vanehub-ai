import assert from "node:assert/strict";
import { bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  coreInvoke,
  createFeishuSession,
  listMessages,
  promoteToMultiAgentSession,
} from "../helpers/feishu-im.mjs";
import {
  operatorInstruction,
  qualifyLiveScenario as qualify,
  recordLiveScenario as record,
  waitForLiveFinalDelivery,
} from "../helpers/feishu-live.mjs";

const OPERATOR_TIMEOUT = 10 * 60_000;
const operatorEnabled = globalThis.process.env.VANEHUB_FEISHU_LIVE_OPERATOR === "1";
const operatorScenarios = [
  "direct-message-receipt",
  "duplicate-delivery",
  "single-agent-response",
  "multi-agent-mentioned-routing",
  "multi-agent-default-routing",
  "multi-agent-invalid-seat",
  "outbound-chunking",
  "session-disable-re-enable",
];

async function connectorIsConnected() {
  return globalThis.browser.waitUntil(async () => {
    const connectors = await coreInvoke("list_im_connectors");
    return connectors.some(({ descriptor, health }) => (
      descriptor.kind === "feishu" && health.lifecycle === "connected"
    ));
  }, { timeout: 60_000, interval: 500, timeoutMsg: "Feishu did not reconnect for operator phase" })
    .then(() => true, () => false);
}

async function recordOperatorScenarios(status, safeErrorCode) {
  for (const scenario of operatorScenarios) await record(scenario, status, safeErrorCode);
}

async function waitForBinding(sessionId) {
  return globalThis.browser.waitUntil(async () => {
    const snapshot = await coreInvoke("get_im_session_binding", { sessionId, connector: "feishu" });
    return snapshot.binding?.state === "active" ? snapshot : false;
  }, { timeout: OPERATOR_TIMEOUT, interval: 1_000, timeoutMsg: "operator pairing timed out" });
}

async function pairSession(sessionId, replaceExisting = false) {
  await coreInvoke("set_im_session_access", { sessionId, connector: "feishu", enabled: true });
  const pairing = await coreInvoke("begin_im_pairing", {
    sessionId,
    connector: "feishu",
    replaceExisting,
  });
  operatorInstruction(`请在专用飞书单聊中发送：/bind ${pairing.code}`);
  return waitForBinding(sessionId);
}

async function waitForCompletedTurn(sessionId, expectedText, minimum) {
  const completed = await globalThis.browser.waitUntil(async () => {
    const messages = await listMessages(sessionId);
    if (messages.length < minimum) return false;
    const userIndex = messages.findIndex(({ role, content }) => (
      role === "user" && content === expectedText
    ));
    if (userIndex < 0) return false;
    const assistant = messages.slice(userIndex + 1).find(({ role, status }) => (
      role === "assistant" && status === "completed"
    ));
    return assistant ? { messages, assistant } : false;
  }, { timeout: OPERATOR_TIMEOUT, interval: 1_000, timeoutMsg: "operator message timed out" });
  await waitForLiveFinalDelivery(sessionId, completed.assistant.id);
  return completed;
}

async function sendAndWait(session, instruction, expectedText, expectedCount) {
  operatorInstruction(instruction);
  return waitForCompletedTurn(session.id, expectedText, expectedCount);
}

globalThis.describe("VaneHub AI live Feishu operator inbound", () => {
  globalThis.it("qualifies real direct messages and Agent routing", async function operatorInbound() {
    this.timeout(60 * 60_000);
    if (!operatorEnabled) {
      await recordOperatorScenarios("NOT RUN", "live-operator-opt-in-required");
      this.skip();
    }

    await bootDesktopUi();
    if (!await connectorIsConnected()) {
      await recordOperatorScenarios("BLOCKED", "connector-prerequisite-failed");
      this.skip();
    }
    const single = await createFeishuSession("Feishu live multi Agent");
    await pairSession(single.id);
    await qualify("direct-message-receipt", async () => {
      await sendAndWait(single, "发送：VANEHUB_LIVE_DIRECT_CHECK", "VANEHUB_LIVE_DIRECT_CHECK", 2);
    });
    await qualify("single-agent-response", async () => {
      operatorInstruction("看到机器人回复后，再发送：VANEHUB_LIVE_SINGLE_CONFIRMED");
      await waitForCompletedTurn(single.id, "VANEHUB_LIVE_SINGLE_CONFIRMED", 4);
    });

    await record("duplicate-delivery", "BLOCKED", "feishu-platform-retry-not-observed");

    operatorInstruction("发送：VANEHUB_E2E_OVERSIZED；收到两段机器人回复后发送：VANEHUB_LIVE_CHUNK_CONFIRMED");
    await qualify("outbound-chunking", async () => {
      const { messages } = await waitForCompletedTurn(
        single.id,
        "VANEHUB_LIVE_CHUNK_CONFIRMED",
        8,
      );
      assert.ok(messages.some(({ role, content }) => role === "assistant" && content.length > 20_000));
    });

    const multi = await promoteToMultiAgentSession(single);
    const secondHandle = multi.handles[1];
    const mentionedText = `@${secondHandle} live mentioned route`;
    operatorInstruction(`发送：${mentionedText}`);
    await qualify("multi-agent-mentioned-routing", async () => {
      const { assistant } = await waitForCompletedTurn(multi.id, mentionedText, 2);
      assert.equal(assistant.speakerSeatId, multi.seats[1].seatId);
    });
    operatorInstruction("看到回复后，发送：VANEHUB_LIVE_DEFAULT_CHECK");
    await qualify("multi-agent-default-routing", async () => {
      const { assistant } = await waitForCompletedTurn(
        multi.id,
        "VANEHUB_LIVE_DEFAULT_CHECK",
        4,
      );
      assert.equal(assistant.speakerSeatId, multi.seats[1].seatId);
    });
    await qualify("multi-agent-invalid-seat", async () => {
      const before = (await listMessages(multi.id)).length;
      operatorInstruction("发送：@missing-seat live invalid route");
      operatorInstruction("看到有效席位提示后，发送：VANEHUB_LIVE_INVALID_CONFIRMED");
      const { messages } = await waitForCompletedTurn(
        multi.id,
        "VANEHUB_LIVE_INVALID_CONFIRMED",
        before + 2,
      );
      assert.equal(messages.length, before + 2, "invalid seat mention silently dispatched an Agent");
    });

    const beforeDisable = (await listMessages(multi.id)).length;
    await coreInvoke("set_im_session_access", {
      sessionId: multi.id,
      connector: "feishu",
      enabled: false,
    });
    operatorInstruction("发送：VANEHUB_LIVE_DISABLED_CHECK；看到会话已禁用提示后等待 90 秒。");
    await new Promise((resolve) => globalThis.setTimeout(resolve, 90_000));
    assert.equal((await listMessages(multi.id)).length, beforeDisable,
      "disabled session admitted an Agent turn");
    await coreInvoke("set_im_session_access", {
      sessionId: multi.id,
      connector: "feishu",
      enabled: true,
    });
    await qualify("session-disable-re-enable", async () => {
      operatorInstruction("会话已重新启用。发送：VANEHUB_LIVE_REENABLE_CONFIRMED");
      await waitForCompletedTurn(multi.id, "VANEHUB_LIVE_REENABLE_CONFIRMED", beforeDisable + 2);
    });
  });
});
