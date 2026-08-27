import assert from "node:assert/strict";
import { assertNoFatalError, bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  comparablePath,
  coreInvoke,
  createFeishuSession,
  events,
  findSession,
  listMessages,
  openSessionIm,
  persistedSessionTitle,
} from "../helpers/feishu-im.mjs";

globalThis.describe("VaneHub AI desktop Feishu IM session access", () => {
  globalThis.it("restores native access and binding after a real relaunch and completes one Agent round trip", async function access() {
    this.timeout(240_000);
    const root = await bootDesktopUi();
    const session = await findSession(persistedSessionTitle);
    const persisted = await coreInvoke("get_im_session_binding", { sessionId: session.id });
    assert.equal(persisted.access.enabled, true, "native access did not survive the relaunch");
    assert.equal(persisted.binding?.state, "active", "the fixture binding did not survive the relaunch");
    const restoredSwitch = await openSessionIm(session.id);
    assert.equal(await restoredSwitch.isSelected(), true, "native IM access was not restored");

    await coreInvoke("fixture_feishu_im_reset");
    await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });
    assert.equal((await listMessages(session.id)).length, 0);
    const delivered = await coreInvoke("fixture_feishu_im_inject", { input: events.relaunchRoundTrip });
    assert.equal(delivered.status, "delivered");
    assert.equal(delivered.outboundChunks, 1);
    const messages = await listMessages(session.id);
    assert.deepEqual(messages.map(({ role, status }) => ({ role, status })), [
      { role: "user", status: "completed" },
      { role: "assistant", status: "completed" },
    ]);
    await assertNoFatalError(root);
  });

  globalThis.it("gates sanitized inbound events and records only safe delivery metadata", async function fixture() {
    this.timeout(240_000);
    await bootDesktopUi();
    const session = await createFeishuSession();
    await coreInvoke("fixture_feishu_im_reset");
    const setup = await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });
    assert.deepEqual(setup, { ready: true, connector: "feishu" });

    const ignored = await coreInvoke("fixture_feishu_im_inject", { input: events.ignored });
    assert.equal(ignored.status, "ignored");
    const duplicate = await coreInvoke("fixture_feishu_im_inject", { input: events.ignored });
    assert.equal(duplicate.status, "duplicate");
    assert.equal(duplicate.duplicate, true);

    await coreInvoke("set_im_session_access", {
      sessionId: session.id,
      connector: "feishu",
      enabled: false,
    });
    const disabled = await coreInvoke("fixture_feishu_im_inject", { input: events.disabled });
    assert.equal(disabled.status, "rejected");
    assert.equal(disabled.safeErrorCode, "im-session-disabled");

    await coreInvoke("fixture_feishu_im_set_fault", { fault: "disconnected" });
    const reconnecting = await coreInvoke("fixture_feishu_im_inject", { input: events.reconnect });
    assert.equal(reconnecting.status, "reconnecting");
    assert.equal(reconnecting.safeErrorCode, "fixture-disconnected");
    await coreInvoke("fixture_feishu_im_set_fault", { fault: "none" });

    const ledger = await coreInvoke("fixture_feishu_im_ledger");
    assert.deepEqual(ledger, [ignored, duplicate, disabled, reconnecting]);
    const serialized = JSON.stringify(ledger);
    for (const event of Object.values(events)) {
      assert.equal(serialized.includes(event.eventId), false, "ledger leaked an event id");
      assert.equal(serialized.includes(event.text), false, "ledger leaked message content");
    }
    assert.equal(serialized.includes(session.id), false, "ledger leaked a session id");
  });

  globalThis.it("routes one Feishu event through one real CLI Agent turn and one terminal delivery", async function singleAgent() {
    this.timeout(240_000);
    await bootDesktopUi();
    const session = await createFeishuSession();
    await coreInvoke("fixture_feishu_im_reset");
    await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });
    const configurationBefore = await coreInvoke("get_session_chat_config", {
      sessionId: session.id,
    });
    assert.equal(session.agentId, "opencode");
    assert.equal(comparablePath(session.projectPath), comparablePath(session.fixtureProjectPath));
    assert.equal(comparablePath(session.folder), comparablePath(session.fixtureProjectPath));
    assert.equal(session.worktreePath, null);
    assert.equal(configurationBefore.agentId, session.agentId);

    assert.equal((await listMessages(session.id)).length, 0);
    const delivered = await coreInvoke("fixture_feishu_im_inject", { input: events.singleAgent });
    assert.equal(delivered.status, "delivered");
    assert.equal(delivered.outboundChunks, 1);
    const messages = await listMessages(session.id);
    assert.equal(messages.length, 2, "the event did not create exactly one Agent turn");
    assert.deepEqual(messages.map(({ role, status }) => ({ role, status })), [
      { role: "user", status: "completed" },
      { role: "assistant", status: "completed" },
    ]);
    assert.ok(messages[1].content.length > 0, "the terminal Agent response was empty");
    const persistedSession = (await coreInvoke("list_sessions"))
      .find(({ id }) => id === session.id);
    assert.ok(persistedSession, "the bound session disappeared after the Feishu turn");
    assert.deepEqual({
      agentId: persistedSession.agentId,
      folder: persistedSession.folder,
      projectPath: persistedSession.projectPath,
      worktreePath: persistedSession.worktreePath,
    }, {
      agentId: session.agentId,
      folder: session.folder,
      projectPath: session.projectPath,
      worktreePath: session.worktreePath,
    });
    const configurationAfter = await coreInvoke("get_session_chat_config", {
      sessionId: session.id,
    });
    assert.deepEqual(configurationAfter, configurationBefore,
      "the Feishu turn changed the session provider configuration");

    await coreInvoke("fixture_feishu_im_set_fault", { fault: "outbound-failure" });
    const failed = await coreInvoke("fixture_feishu_im_inject", { input: events.outboundFailure });
    assert.equal(failed.status, "outbound-failed");
    assert.equal(failed.safeErrorCode, "fixture-outbound-failed");
    assert.equal((await listMessages(session.id)).length, 4, "the failed delivery did not retain its Agent turn");
    const duplicate = await coreInvoke("fixture_feishu_im_inject", { input: events.outboundFailure });
    assert.equal(duplicate.status, "duplicate");
    assert.equal((await listMessages(session.id)).length, 4, "delivery retry reran the Agent");
  });
});
