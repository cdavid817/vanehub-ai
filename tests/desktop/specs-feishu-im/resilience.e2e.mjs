import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  coreInvoke,
  createFeishuSession,
  events,
  listMessages,
} from "../helpers/feishu-im.mjs";

async function setupSession(title) {
  const session = await createFeishuSession(title);
  await coreInvoke("fixture_feishu_im_reset");
  await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });
  return session;
}

globalThis.describe("VaneHub AI desktop Feishu IM resilience", () => {
  globalThis.before(async () => bootDesktopUi());

  globalThis.it("deduplicates events and drops a malformed recorded protocol frame", async () => {
    const session = await setupSession("Feishu resilience dedup");
    const retryEvent = {
      ...events.singleAgent,
      eventId: "evt-sanitized-resilience-dedup-v1",
    };
    const delivered = await coreInvoke("fixture_feishu_im_inject", { input: retryEvent });
    assert.equal(delivered.status, "delivered");
    const messageCount = (await listMessages(session.id)).length;
    const duplicate = await coreInvoke("fixture_feishu_im_inject", { input: retryEvent });
    assert.equal(duplicate.status, "duplicate");
    assert.equal((await listMessages(session.id)).length, messageCount);

    const malformed = await coreInvoke("fixture_feishu_im_inject", { input: events.malformed });
    assert.deepEqual({ status: malformed.status, safeErrorCode: malformed.safeErrorCode }, {
      status: "malformed",
      safeErrorCode: "fixture-event-invalid",
    });
    assert.equal((await listMessages(session.id)).length, messageCount);
  });

  globalThis.it("allows an admitted turn to finish but rejects work after disable completes", async () => {
    const session = await setupSession("Feishu resilience disable race");
    const raced = await globalThis.browser.tauri.execute(async ({ core }, args) => {
      const delivery = core.invoke("fixture_feishu_im_inject", { input: args.event });
      let admitted = false;
      for (let attempt = 0; attempt < 100; attempt += 1) {
        const messages = await core.invoke("list_messages", {
          sessionId: args.sessionId,
          limit: null,
          beforeId: null,
        });
        if (messages.length > 0) {
          admitted = true;
          break;
        }
        await new Promise((resolve) => globalThis.setTimeout(resolve, 25));
      }
      if (!admitted) throw new Error("the fixture turn was never admitted");
      const access = await core.invoke("set_im_session_access", {
        sessionId: args.sessionId,
        connector: "feishu",
        enabled: false,
      });
      return { access, delivery: await delivery };
    }, { sessionId: session.id, event: events.disableRace });
    assert.equal(raced.access.enabled, false);
    assert.equal(raced.delivery.status, "delivered");
    const admittedCount = (await listMessages(session.id)).length;

    const rejected = await coreInvoke("fixture_feishu_im_inject", { input: events.afterDisable });
    assert.equal(rejected.status, "rejected");
    assert.equal(rejected.safeErrorCode, "im-session-disabled");
    assert.equal((await listMessages(session.id)).length, admittedCount);
  });

  globalThis.it("recovers from disconnect and reports ordered oversized and failed delivery", async () => {
    const session = await setupSession("Feishu resilience transport");
    await coreInvoke("fixture_feishu_im_set_fault", { fault: "disconnected" });
    const disconnected = await coreInvoke("fixture_feishu_im_inject", { input: events.reconnect });
    assert.equal(disconnected.status, "reconnecting");
    await coreInvoke("fixture_feishu_im_set_fault", { fault: "none" });
    assert.equal((await coreInvoke("fixture_feishu_im_inject", { input: events.recovered })).status,
      "delivered");

    const oversized = await coreInvoke("fixture_feishu_im_inject", { input: events.oversized });
    assert.equal(oversized.status, "delivered");
    assert.equal(oversized.outboundChunks, 2, "the oversized Unicode response was not split in order");

    await coreInvoke("fixture_feishu_im_set_fault", { fault: "outbound-failure" });
    const before = (await listMessages(session.id)).length;
    const failed = await coreInvoke("fixture_feishu_im_inject", {
      input: {
        ...events.outboundFailure,
        eventId: "evt-sanitized-resilience-outbound-failure-v1",
      },
    });
    assert.equal(failed.status, "outbound-failed");
    assert.equal(failed.safeErrorCode, "fixture-outbound-failed");
    assert.equal((await listMessages(session.id)).length, before + 2,
      "the terminal Agent turn was lost after outbound failure");
  });

  globalThis.it("leaves an owned process marker for the layer's clean shutdown check", async () => {
    const ledger = await coreInvoke("fixture_feishu_im_ledger");
    const allowedKeys = ["duplicate", "outboundChunks", "safeErrorCode", "sequence", "status"];
    assert.ok(ledger.length > 0, "the retained fixture ledger was empty");
    for (const entry of ledger) {
      assert.deepEqual(Object.keys(entry).sort(), allowedKeys,
        "the fixture ledger exposed fields beyond safe delivery metadata");
    }
    await writeFile(
      join(globalThis.process.env.VANEHUB_DESKTOP_RESULT_DIR, "feishu-fixture-ledger.json"),
      `${JSON.stringify(ledger, null, 2)}\n`,
      "utf8",
    );
    const marker = JSON.parse(await readFile(
      join(globalThis.process.env.VANEHUB_APP_DATA_DIR, "desktop-e2e-process.json"),
      "utf8",
    ));
    assert.equal(marker.runId, globalThis.process.env.VANEHUB_TEST_RUN_ID);
    assert.equal(marker.state, "running");
    assert.ok(Number.isInteger(marker.pid) && marker.pid > 0);
  });
});
