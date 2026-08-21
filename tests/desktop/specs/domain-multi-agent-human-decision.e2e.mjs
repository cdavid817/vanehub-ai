import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];
const stamp = Date.now().toString(36);

/**
 * The half of `@用户 handoff` no other spec reaches: a seat asks the human to decide, the round
 * actually stops, and the human's answer starts it again.
 *
 * `parse_human_handoff` and `apply_human_handoff` have unit tests, and the project flow observed a
 * real pause when a seat hit a permission gate — but nothing checked, live, that the pause
 * *suppresses the teammate the same reply names*, or that a human message resumes the round rather
 * than leaving it parked. Those are the two halves a person actually experiences: the round waits
 * for me, and then my answer moves it.
 *
 * The pause is asserted as an absence, so it is asserted over a window rather than at an instant:
 * the coordinator polls terminals every 200ms (`seat_turn_coordinator.rs`), so a round that failed
 * to stop would dispatch the named teammate within a second or two. Thirty seconds of silence is
 * therefore evidence, not luck.
 *
 * The asker is claude-code and the teammate codex-cli, both seated for words only — the pause has
 * to happen before any provider work, so neither seat needs write permission and this spec stays
 * out of the acting-seat permission constraints the business flow documents.
 */
const ASKER = "claude-code";
const TEAMMATE = "codex-cli";
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer"];
/** Long enough that a round which failed to stop would have dispatched; short enough to be cheap. */
const PAUSE_OBSERVATION_MS = 30_000;

async function settleOperation(operation, timeoutMsg) {
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 90_000, timeoutMsg });
  assert.equal(settled.status, "succeeded", settled.error ?? timeoutMsg);
  return settled;
}

async function createRepository() {
  const root = await mkdtemp(join(tmpdir(), "vanehub-multiagent-decision-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

let repository = null;
const flow = { session: null, seats: [], handles: [], failed: false };

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

function sendUser(content) {
  return invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: flow.session.id,
    content,
    config: {
      agentId: flow.session.agentId,
      interactionMode: "cli",
      executionMode: "inherit",
      providerId: null,
      modelId: null,
      reasoningDepth: null,
      streaming: true,
      thinking: false,
      longContext: false,
    },
    fileReferences: null,
  });
}

/** The `ordinal`-th assistant turn, settled. Ordinal-addressed because the asker speaks twice. */
async function settledTurn(stage, ordinal, seat, handle) {
  const dispatched = await globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(flow.session.id));
    return rows.length > ordinal ? rows[ordinal] : false;
  }, {
    timeout: 120_000,
    interval: 2_000,
    timeoutMsg: `turn ${ordinal} (${handle}) was never dispatched`,
  }).catch(() => null);
  if (!dispatched) {
    flow.failed = true;
    assert.fail(`turn ${ordinal} for @${handle} (${seat.agentId}) was never dispatched`);
  }
  assert.equal(
    dispatched.speakerSeatId,
    seat.seatId,
    `turn ${ordinal} belongs to a seat other than @${handle}`,
  );
  const settled = await globalThis.browser.waitUntil(async () => {
    const row = assistantsOf(await listMessages(flow.session.id))[ordinal];
    return ["completed", "failed", "cancelled"].includes(row?.status) ? row : false;
  }, {
    timeout: 180_000,
    interval: 2_000,
    timeoutMsg: `turn ${ordinal} (${handle}) never settled`,
  }).catch(() => null);
  if (!settled || settled.status !== "completed" || !settled.content.trim()) {
    blocked.push(`${stage}: the ${seat.agentId} turn ended ${settled?.status ?? "never"}; that `
      + "seat's runtime failed, not the handoff");
    return null;
  }
  return settled;
}

globalThis.describe("VaneHub AI desktop multi-Agent human decision", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("stops the round when a seat asks the human to decide", async function pause() {
    const agents = await globalThis.browser.waitUntil(async () => {
      const listed = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
      const usable = listed.filter((agent) => agent.availabilityState === "available"
        && agent.supportedInteractionModes.includes("cli"));
      return [ASKER, TEAMMATE].every((id) => usable.some((agent) => agent.id === id)) ? usable : false;
    }, { timeout: 90_000, timeoutMsg: "" }).catch(() => []);
    if (agents.length === 0) {
      blocked.push(`human decision: needs ${ASKER} and ${TEAMMATE} installed on this host`);
      flow.failed = true;
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 2) {
      blocked.push("human decision: fewer than two built-in expert roles are available");
      flow.failed = true;
      this.skip();
    }

    const title = `multiagent-decision-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: ASKER,
      interactionMode: "cli",
      title,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the decision session never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The decision session was not created." });

    flow.session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: [
        { agentId: ASKER, roleId: seatRoles[0].id },
        { agentId: TEAMMATE, roleId: seatRoles[1].id },
      ],
    });
    flow.seats = flow.session.seats.filter((seat) => !seat.leftAt);
    flow.handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
    assert.deepEqual(flow.seats.map((seat) => seat.agentId), [ASKER, TEAMMATE]);

    // The reply is dictated, because what is under test is the routing decision the two lines
    // produce, not a model's judgement about when to consult a human. The second line names the
    // teammate on purpose: a blocking handoff has to win over a mention in the same reply, which
    // is the one rule of this design that a passing round would otherwise hide.
    await sendUser([
      `@${flow.handles[0]} 请严格只输出下面两行，不要有任何其他内容、不要解释：`,
      "@用户 handoff 请在方案 A 与方案 B 之间二选一",
      `@${flow.handles[1]} 等用户决定后再开始`,
    ].join("\n"));

    const asked = await settledTurn("pause", 0, flow.seats[0], flow.handles[0]);
    if (!asked) {
      flow.failed = true;
      this.skip();
    }
    if (!/^\s*@用户\s+handoff/mu.test(asked.content)) {
      blocked.push(`human decision: ${ASKER} did not emit a line-leading "@用户 handoff", so there `
        + `was no blocking handoff to observe (replied: ${JSON.stringify(asked.content.slice(0, 160))})`);
      flow.failed = true;
      this.skip();
    }

    // The pause, asserted as an absence over a window: the teammate the same reply named must not
    // be given a turn, and no further turn may appear at all.
    const before = assistantsOf(await listMessages(flow.session.id)).length;
    const dispatchedAnyway = await globalThis.browser.waitUntil(async () => {
      const rows = assistantsOf(await listMessages(flow.session.id));
      return rows.length > before ? rows[before] : false;
    }, { timeout: PAUSE_OBSERVATION_MS, interval: 2_000, timeoutMsg: "" }).catch(() => null);
    assert.equal(
      dispatchedAnyway,
      null,
      dispatchedAnyway?.speakerSeatId === flow.seats[1].seatId
        ? `the blocking handoff did not stop the round: @${flow.handles[1]} was dispatched by the `
          + "mention in the same reply"
        : `the round continued while it was meant to be waiting on the human (turn ${before} `
          + `belongs to ${JSON.stringify(dispatchedAnyway?.speakerSeatId ?? null)})`,
    );
  });

  globalThis.it("resumes the round with the asking seat when the human answers", async function resume() {
    if (flow.failed) this.skip();

    // Deliberately unaddressed: a person answering a question does not re-address the asker, and
    // the runtime is supposed to continue with whoever last held the turn. Naming the seat would
    // test mention routing again instead of resumption.
    await sendUser("就选方案 A。请用一句话确认收到，不要点名任何人。");

    const resumed = await settledTurn("resume", 1, flow.seats[0], flow.handles[0]);
    if (!resumed) this.skip();
    assert.ok(resumed.content.trim(), "the resumed turn produced no words");

    // Exactly two Agent turns, both the asker's: the teammate never spoke, before or after.
    const speakers = assistantsOf(await listMessages(flow.session.id))
      .map((row) => row.speakerSeatId);
    assert.deepEqual(
      speakers,
      [flow.seats[0].seatId, flow.seats[0].seatId],
      "the thread is not the asking seat pausing and then resuming on its own",
    );
  });

  globalThis.after(async () => {
    if (flow.session) {
      await invoke(({ core }, id) => core.invoke("stop_generation", { sessionId: id }), flow.session.id)
        .catch(() => {});
      if (globalThis.process?.env?.VANEHUB_DESKTOP_KEEP_SESSIONS !== "1") {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), flow.session.id)
          .catch(() => {});
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
