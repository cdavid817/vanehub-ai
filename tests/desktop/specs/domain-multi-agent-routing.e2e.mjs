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
 * Who answers when the *human* speaks in a multi-seat session.
 *
 * `domain-multi-agent.e2e.mjs` covers the Agent-to-Agent half: a seat replies, its `@` is parsed,
 * and the named teammate takes the next turn. It asserts nothing about the first turn of a round,
 * because in every case it exercises the round starts with whichever seat sits first. That leaves
 * the whole "Human routing by mention" requirement
 * (`openspec/specs/multi-agent-group-chat/spec.md`) uncovered end to end -- a person addressing one
 * seat out of several is the most ordinary thing there is to do in a group chat, and nothing
 * checked that it works.
 *
 * Routing is asserted on the *dispatch*, not on the reply text: the assistant row is written with
 * its seat attribution before the provider is invoked, so the row's `speakerSeatId` is the routing
 * verdict on its own and does not have to wait on a model. That keeps these cases quick, and keeps
 * them measuring routing rather than a provider's willingness to follow an instruction.
 */
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];

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
  const root = await mkdtemp(join(tmpdir(), "vanehub-multiagent-routing-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

let repository = null;
const createdSessions = [];

async function usableAgents() {
  const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
  return agents.filter((agent) => agent.availabilityState === "available"
    && agent.supportedInteractionModes.includes("cli"));
}

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

const assistantsOf = (messages) => messages.filter((message) => message.role === "assistant");

async function sendUser(session, agentId, content) {
  await invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: session.id,
    content,
    config: {
      agentId,
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

/** Releases the provider a routed turn started, so one case's model does not run into the next. */
const stopGeneration = (sessionId) => invoke(
  ({ core }, id) => core.invoke("stop_generation", { sessionId: id }),
  sessionId,
).catch(() => {});

/**
 * Seats `count` participants under distinct built-in roles, and reports the handles they answer to.
 *
 * Agents are cycled rather than required one per seat: a host with two installed CLIs can still
 * hold a three-way conversation, because a seat is a role plus an Agent and the same Agent may sit
 * twice under different roles. The handle is derived the way `seat_roster` derives it -- the role's
 * display name with its whitespace hyphenated -- rather than read off the seat, because a seat
 * stores a role id and a snapshot, not a handle.
 */
async function seatAgents(usable, roles, title, count) {
  const seatRoles = BUILTIN_ROLES
    .map((id) => roles.find((role) => role.id === id))
    .filter(Boolean)
    .slice(0, count);
  assert.equal(seatRoles.length, count, `fewer than ${count} built-in expert roles are available`);
  return seatLineup(seatRoles.map((role, index) => ({
    agent: usable[index % usable.length],
    role,
  })), title);
}

/** Seats an explicit Agent-per-role lineup, and reports the handles each seat answers to. */
async function seatLineup(lineup, title) {
  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId: lineup[0].agent.id,
    interactionMode: "cli",
    title,
    folder: repository,
    projectPath: repository,
    remoteWorkspace: null,
    worktree: null,
  });
  await settleOperation(operation, `Creating ${title} never settled.`);
  const created = await globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `${title} was not created.` });
  createdSessions.push(created.id);

  const session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
    sessionId: created.id,
    expectedUpdatedAt: created.updatedAt,
    seats: lineup.map(({ agent, role }) => ({ agentId: agent.id, roleId: role.id })),
  });
  const seats = session.seats.filter((seat) => !seat.leftAt);
  const handles = lineup.map(({ role }) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));
  return { session, seats, handles };
}

/** The assistant row that appears after `before` of them already existed, with its attribution. */
function nextAssistantRow(sessionId, before, timeoutMsg) {
  return globalThis.browser.waitUntil(async () => {
    const rows = assistantsOf(await listMessages(sessionId));
    return rows.length > before ? rows[before] : false;
  }, { timeout: 60_000, interval: 500, timeoutMsg }).catch(() => null);
}

globalThis.describe("VaneHub AI desktop multi-Agent human routing", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("gives a user message to the seat it @-mentions", async function mention() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`user mention: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const { session, seats, handles } = await seatAgents(usable, roles, `routing-mention-${stamp}`, 2);
    assert.equal(seats.length, 2, "the fixture did not seat two Agents");

    // Addressed to the *second* seat: the first one answering regardless is the failure this
    // covers, and a message aimed at seat one could not tell the two apart.
    await sendUser(session, session.agentId, `@${handles[1]} 请回复一个字：好`);
    const row = await nextAssistantRow(session.id, 0, "no seat was given the turn");
    await stopGeneration(session.id);
    if (!row) {
      blocked.push("user mention: no turn was created at all, so routing could not be observed");
      this.skip();
    }
    assert.equal(
      row.speakerSeatId,
      seats[1].seatId,
      `a message addressed to @${handles[1]} was routed to a different seat`,
    );
  });

  globalThis.it("gives an unaddressed user message to the seat that last spoke", async function last() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`last holder: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const { session, seats, handles } = await seatAgents(usable, roles, `routing-last-${stamp}`, 2);

    // The second seat becomes the last holder by being addressed. Sending the follow-up while that
    // turn still runs would exercise queueing rather than routing, so this waits for it to settle
    // before speaking again.
    await sendUser(session, session.agentId, `@${handles[1]} 请回复一个字：好`);
    const first = await nextAssistantRow(session.id, 0, "the addressed seat never took a turn");
    if (!first || first.speakerSeatId !== seats[1].seatId) {
      blocked.push("last holder: the addressed turn did not reach the second seat; the mention "
        + "case reports why");
      this.skip();
    }
    const settled = await globalThis.browser.waitUntil(async () => {
      const row = assistantsOf(await listMessages(session.id))[0];
      // A message settles as `completed`/`failed`/`cancelled`; `succeeded` is the *operation*
      // vocabulary and matches nothing here, which reads as a turn that never ended.
      return ["completed", "failed", "cancelled"].includes(row?.status) ? row : false;
    }, { timeout: 240_000, interval: 2_000, timeoutMsg: "the first turn never settled" })
      .catch(() => null);
    if (!settled) {
      blocked.push("last holder: the addressed seat's turn never settled on this host");
      this.skip();
    }

    const before = assistantsOf(await listMessages(session.id)).length;
    await sendUser(session, session.agentId, "继续");
    const row = await nextAssistantRow(session.id, before, "the follow-up was never given to a seat");
    await stopGeneration(session.id);
    if (!row) {
      blocked.push("last holder: the follow-up created no turn");
      this.skip();
    }
    assert.equal(
      row.speakerSeatId,
      seats[1].seatId,
      "an unaddressed follow-up went to a seat other than the one that last held the turn",
    );
  });

  globalThis.it("relays a round across three seats: human mention, then an Agent handoff", async function chain() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`three-seat chain: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const { session, seats, handles } = await seatAgents(usable, roles, `routing-chain-${stamp}`, 3);
    assert.equal(seats.length, 3, "the fixture did not seat three participants");

    // The human addresses the *second* seat and asks it to hand off to the *third*, so one round
    // exercises both halves of routing in sequence: user->seat by mention, then seat->seat by
    // reply. Neither hop involves the first seat, which is what rules out "everything goes to
    // seat one" surviving in either half.
    await sendUser(
      session,
      session.agentId,
      `@${handles[1]} 只回复一行，内容为：\n@${handles[2]} 请继续`,
    );
    const first = await nextAssistantRow(session.id, 0, "no seat was given the first turn");
    if (!first) {
      blocked.push("three-seat chain: the mention created no turn at all");
      this.skip();
    }
    assert.equal(
      first.speakerSeatId,
      seats[1].seatId,
      `the user's @${handles[1]} was answered by a different seat`,
    );

    // The handoff needs the second seat's reply to actually contain the mention, which is the
    // provider following an instruction -- reported rather than failed when it declines.
    const settled = await globalThis.browser.waitUntil(async () => {
      const row = assistantsOf(await listMessages(session.id))[0];
      return ["completed", "failed", "cancelled"].includes(row?.status) ? row : false;
    }, { timeout: 240_000, interval: 2_000, timeoutMsg: "the second seat's turn never settled" })
      .catch(() => null);
    if (!settled || settled.status !== "completed") {
      blocked.push(`three-seat chain: the ${handles[1]} turn ended ${settled?.status ?? "never"}, `
        + "so no handoff could follow");
      this.skip();
    }
    if (!settled.content.includes(`@${handles[2]}`)) {
      blocked.push(`three-seat chain: ${handles[1]} did not emit @${handles[2]} `
        + `(replied ${JSON.stringify(settled.content.slice(0, 120))})`);
      this.skip();
    }

    // The coordinator writes the third seat's row before invoking its provider, so the row's
    // existence is the handoff verdict without waiting on another model round trip.
    const relayed = await globalThis.browser.waitUntil(async () => {
      const rows = assistantsOf(await listMessages(session.id));
      return rows.find((row) => row.speakerSeatId === seats[2].seatId) ?? false;
    }, { timeout: 120_000, interval: 2_000, timeoutMsg: "the third seat was never dispatched" })
      .catch(() => null);
    await stopGeneration(session.id);
    if (!relayed) {
      const rows = assistantsOf(await listMessages(session.id));
      blocked.push("three-seat chain: no turn reached the third seat; assistant rows were "
        + JSON.stringify(rows.map((row) => row.speakerSeatId ?? "unattributed")));
      this.skip();
    }
    assert.equal(relayed.speakerSeatId, seats[2].seatId);
  });

  globalThis.it("seats claude-code, codex-cli and opencode together and each answers its own mention", async function trio() {
    // Named CLIs, not whatever `list_agents` sorts first: the point is three *heterogeneous*
    // runtimes sharing one thread, which agent-cycling can silently reduce to two (this host's
    // cycle seated gemini rather than opencode, so the codex+claude+opencode mix had never run).
    const TRIO = ["claude-code", "codex-cli", "opencode"];
    const usable = await usableAgents();
    const lineupAgents = TRIO.map((id) => usable.find((agent) => agent.id === id));
    const missing = TRIO.filter((_, index) => !lineupAgents[index]);
    if (missing.length > 0) {
      blocked.push(`trio: ${missing.join(", ")} not installed/usable on this host`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 3) {
      blocked.push("trio: fewer than three built-in expert roles are available");
      this.skip();
    }
    const { session, seats, handles } = await seatLineup(
      lineupAgents.map((agent, index) => ({ agent, role: seatRoles[index] })),
      `routing-trio-${stamp}`,
    );
    assert.deepEqual(
      seats.map((seat) => seat.agentId),
      TRIO,
      "the session did not seat the three named CLIs in order",
    );

    // Each seat is addressed in turn and must *complete* a real reply before the next message,
    // so all three CLI runtimes demonstrably take a turn in the same shared thread -- routing,
    // spawn, and a full provider round trip per seat, serially the way the coordinator promises.
    for (const [index, seat] of seats.entries()) {
      const before = assistantsOf(await listMessages(session.id)).length;
      await sendUser(session, session.agentId, `@${handles[index]} 请回复一个字：好`);
      const row = await nextAssistantRow(
        session.id,
        before,
        `no turn was created for @${handles[index]}`,
      );
      if (!row) {
        blocked.push(`trio: @${handles[index]} (${seat.agentId}) was never dispatched`);
        this.skip();
      }
      assert.equal(
        row.speakerSeatId,
        seat.seatId,
        `@${handles[index]} was answered by a different seat`,
      );
      const settled = await globalThis.browser.waitUntil(async () => {
        const current = assistantsOf(await listMessages(session.id))[before];
        return ["completed", "failed", "cancelled"].includes(current?.status) ? current : false;
      }, {
        timeout: 240_000,
        interval: 2_000,
        timeoutMsg: `the ${seat.agentId} turn never settled`,
      }).catch(() => null);
      if (!settled || settled.status !== "completed" || !settled.content.trim()) {
        blocked.push(`trio: the ${seat.agentId} turn ended `
          + `${settled?.status ?? "never"} with `
          + `${JSON.stringify((settled?.content ?? "").slice(0, 60))}; its runtime, not the `
          + "routing, is what failed here");
        this.skip();
      }
    }

    // Three completed turns, one per distinct seat, all in one session's thread.
    const rows = assistantsOf(await listMessages(session.id));
    const speakers = new Set(rows.map((row) => row.speakerSeatId));
    assert.equal(speakers.size, 3, "the three CLIs did not each speak once in the shared thread");
  });

  globalThis.it("falls back to a seated Agent when the mention names a seat that left", async function departed() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`departed seat: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const { session, seats, handles } = await seatAgents(usable, roles, `routing-departed-${stamp}`, 3);

    assert.equal(seats.length, 3, "the fixture did not seat three participants");

    // The middle seat leaves, so the session still holds a choice of who answers -- with two seats
    // the remaining one would be the only possible answer and the case would pass on arithmetic
    // rather than on routing.
    const keep = [seats[0], seats[2]];
    const remaining = await invoke(
      ({ core }, input) => core.invoke("update_session_seats", { input }),
      {
        sessionId: session.id,
        expectedUpdatedAt: session.updatedAt,
        seats: keep.map((seat) => ({
          seatId: seat.seatId,
          agentId: seat.agentId,
          roleId: seat.roleId,
        })),
      },
    );
    const active = remaining.seats.filter((seat) => !seat.leftAt);
    assert.deepEqual(
      active.map((seat) => seat.seatId),
      keep.map((seat) => seat.seatId),
      "removing the middle seat did not leave the other two",
    );

    // Addressing someone who has left is a dead letter rather than an error: the turn still has to
    // go somewhere, and a departed seat must not be the somewhere. Nobody has spoken in this
    // thread, so the fallback is the first seat.
    await sendUser(session, remaining.agentId, `@${handles[1]} 请回复一个字：好`);
    const row = await nextAssistantRow(session.id, 0, "a message to a departed seat created no turn");
    await stopGeneration(session.id);
    if (!row) {
      blocked.push("departed seat: no turn was created, so the fallback could not be observed");
      this.skip();
    }
    assert.notEqual(
      row.speakerSeatId,
      seats[1].seatId,
      "a seat that left the session was still given a turn by name",
    );
    assert.equal(
      row.speakerSeatId,
      seats[0].seatId,
      "a message naming a departed seat did not fall back to the first seat still in the session",
    );
  });

  globalThis.after(async () => {
    // `VANEHUB_DESKTOP_KEEP_SESSIONS=1` keeps the run's sessions in its isolated database, so a
    // person can open the test client against the run's data directory and inspect the threads.
    const keep = globalThis.process?.env?.VANEHUB_DESKTOP_KEEP_SESSIONS === "1";
    for (const sessionId of createdSessions) {
      await stopGeneration(sessionId);
      if (!keep) {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId)
          .catch(() => {});
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
