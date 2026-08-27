import assert from "node:assert/strict";
import { bootDesktopUi } from "../helpers/native-ui.mjs";
import {
  coreInvoke,
  createMultiAgentSession,
  listMessages,
} from "../helpers/feishu-im.mjs";

const event = (eventId, text) => ({ eventId, text, direct: true });
const assistants = (messages) => messages.filter(({ role }) => role === "assistant");

globalThis.describe("VaneHub AI desktop Feishu IM multi-Agent routing", () => {
  globalThis.it("routes mentioned and default turns and safely rejects an invalid seat", async function routing() {
    this.timeout(240_000);
    await bootDesktopUi();
    const session = await createMultiAgentSession();
    await coreInvoke("fixture_feishu_im_reset");
    await coreInvoke("fixture_feishu_im_setup", { sessionId: session.id });

    const mentioned = await coreInvoke("fixture_feishu_im_inject", {
      input: event("evt-sanitized-multi-mentioned-v1", `@${session.handles[1]} fixture mentioned turn`),
    });
    assert.equal(mentioned.status, "delivered");
    let rows = assistants(await listMessages(session.id));
    assert.equal(rows.length, 1);
    assert.equal(rows[0].speakerSeatId, session.seats[1].seatId, "the mentioned seat did not answer");

    const fallback = await coreInvoke("fixture_feishu_im_inject", {
      input: event("evt-sanitized-multi-default-v1", "fixture default turn"),
    });
    assert.equal(fallback.status, "delivered");
    rows = assistants(await listMessages(session.id));
    assert.equal(rows.length, 2);
    assert.equal(rows[1].speakerSeatId, session.seats[1].seatId, "the current owner did not answer");

    const invalid = await coreInvoke("fixture_feishu_im_inject", {
      input: event("evt-sanitized-multi-invalid-v1", "@missing-seat fixture invalid turn"),
    });
    assert.equal(invalid.status, "system-reply");
    assert.equal(assistants(await listMessages(session.id)).length, 2,
      "an invalid seat mention silently dispatched an Agent");
  });
});
