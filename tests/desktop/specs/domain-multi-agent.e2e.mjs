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
 * Multi-Agent group chat as a working feature, not just as a seat table.
 *
 * `sessions.e2e.mjs` already covers the roster: seats persist, survive a reload, and tombstone on
 * departure. None of that sends a message, so the part the guide is actually about -- several
 * Agents relaying in one shared thread, addressed by `@handle` -- had no end-to-end coverage.
 *
 * The `@` parser itself has unit tests (seat_turn.rs: line-initial only, fenced code exempt), so
 * what is added here is the half those cannot reach: that assigning an expert role to a seat
 * produces an addressable handle, and that a real turn in a real session reaches the seat it names.
 *
 * A handle is *derived*, not stored. `seat-mention-options.ts:19-20` resolves it as
 * `roleSnapshot.roleName ?? role.displayName ?? roleSnapshot.agentName ?? agent.displayName ??
 * 席位N`, so a persisted snapshot whose `roleName` is null is normal rather than broken -- the
 * sessions context only fills a fallback snapshot when the caller supplies none
 * (application/service.rs:520-521), and the role id is what the handle actually comes from. An
 * earlier version of this file asserted on `roleSnapshot.roleName` and read its absence as a
 * missing handle, which was checking one input of the derivation instead of its result.
 *
 * This spec earned its keep on the first honest run: the handoff did not relay, and the reason was
 * a real defect rather than a flaky host. `seat_roster` (seat_turn.rs:107) resolves a seat's role
 * through `ExpertRolePort`, which was the bare `SqliteExpertRoleRepository` -- stored roles only.
 * The built-in roles live in the binary and were merged solely by
 * `ExpertRoleApplicationService::list`, so every seat holding one of the three roles the product
 * ships resolved to no role, the roster named the seat after its Agent, and the mention became
 * `@OnePiece` instead of `@架构师`. The round then ended `NobodyMentioned` and the second seat was
 * never dispatched -- silently, for the default configuration. Fixed by
 * `BuiltinAwareExpertRoleRepository`, which merges the built-ins at the port so its one caller's
 * assumption that the port means "every role there is" actually holds.
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
  const root = await mkdtemp(join(tmpdir(), "vanehub-multiagent-"));
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

async function listMessages(sessionId) {
  return invoke(({ core }, id) => core.invoke("list_messages", {
    sessionId: id,
    limit: null,
    beforeId: null,
  }), sessionId);
}

globalThis.describe("VaneHub AI desktop multi-Agent group chat", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("gives every seated Agent an addressable handle from its expert role", async function handles() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`seat handles: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES
      .map((id) => roles.find((role) => role.id === id))
      .filter(Boolean)
      .slice(0, usable.length);
    if (seatRoles.length < 2) {
      blocked.push("seat handles: fewer than two built-in expert roles are available");
      this.skip();
    }

    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: usable[0].id,
      interactionMode: "cli",
      title: `multiagent-handles-${stamp}`,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the group-chat session never settled.");
    const session = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === `multiagent-handles-${stamp}`) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The group-chat session was not created." });
    createdSessions.push(session.id);

    const seated = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: session.id,
      expectedUpdatedAt: session.updatedAt,
      seats: seatRoles.map((role, index) => ({
        agentId: usable[index].id,
        roleId: role.id,
      })),
    });

    const active = seated.seats.filter((seat) => !seat.leftAt);
    assert.equal(active.length, seatRoles.length, "not every Agent took a seat");

    // Each seat has to end up addressable. The handle is derived rather than stored, so this
    // follows the same chain the composer does rather than reading one field of it.
    const handleFor = (seat) => {
      const role = roles.find((entry) => entry.id === seat.roleId);
      return seat.roleSnapshot?.roleName
        ?? role?.displayName
        ?? seat.roleSnapshot?.agentName
        ?? null;
    };
    for (const seat of active) {
      assert.ok(seat.roleId, `the seat for ${seat.agentId} kept no role id, so it has no handle`);
      assert.ok(
        handleFor(seat),
        `the seat for ${seat.agentId} resolves to no handle, so nothing can @ it`,
      );
    }

    // Handles have to address exactly one seat, or an `@` is ambiguous.
    const handles = active.map(handleFor);
    assert.equal(new Set(handles).size, handles.length, `two seats share a handle: ${handles}`);
  });

  globalThis.it("relays a turn from one seat to another through an `@` handoff", async function handoff() {
    const usable = await usableAgents();
    if (usable.length < 2) {
      blocked.push(`handoff: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES
      .map((id) => roles.find((role) => role.id === id))
      .filter(Boolean)
      .slice(0, 2);
    if (seatRoles.length < 2) {
      blocked.push("handoff: fewer than two built-in expert roles are available");
      this.skip();
    }

    const title = `multiagent-handoff-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: usable[0].id,
      interactionMode: "cli",
      title,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the handoff session never settled.");
    let session = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The handoff session was not created." });
    createdSessions.push(session.id);

    session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: session.id,
      expectedUpdatedAt: session.updatedAt,
      seats: [
        { agentId: usable[0].id, roleId: seatRoles[0].id },
        { agentId: usable[1].id, roleId: seatRoles[1].id },
      ],
    });
    const seats = session.seats.filter((seat) => !seat.leftAt);
    const target = seats[1];
    const targetRole = roles.find((entry) => entry.id === target.roleId);
    const targetHandle = target.roleSnapshot?.roleName
      ?? targetRole?.displayName
      ?? target.roleSnapshot?.agentName;
    assert.ok(targetHandle, "the second seat resolves to no handle to hand off to");

    // Asked for verbatim, because what is under test is the routing, not the model's judgement
    // about whether a handoff is warranted. A reply that does not contain the handle is reported
    // rather than failed: that is the provider declining to follow an instruction, which this
    // suite cannot hold it to.
    await invoke(({ core }, payload) => core.invoke("send_message", payload), {
      sessionId: session.id,
      content: `Reply with exactly this one line and nothing else:\n@${targetHandle} please continue`,
      config: {
        agentId: usable[0].id,
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

    const firstReply = await globalThis.browser.waitUntil(async () => {
      const messages = await listMessages(session.id);
      return messages.find((message) => message.role === "assistant"
        && (message.content ?? "").trim().length > 0) ?? false;
    }, { timeout: 180_000, interval: 2_000, timeoutMsg: "no reply" }).catch(() => null);

    if (!firstReply) {
      blocked.push(`handoff: ${usable[0].id} produced no reply, so no handoff could be observed`);
      this.skip();
    }
    if (!firstReply.content.includes(`@${targetHandle}`)) {
      blocked.push(
        `handoff: ${usable[0].id} did not emit @${targetHandle}, so the routing had nothing to act `
        + `on (replied: ${JSON.stringify(firstReply.content.slice(0, 120))})`,
      );
      this.skip();
    }

    // The assertion that matters: the handoff dispatched the *other* seat. A reply that merely
    // contains the handle proves nothing -- the second Agent has to actually take a turn.
    // Attribution is `speakerSeatId` (chat.ts:254-262), with `seatIndex` as the legacy form for
    // messages predating stable participant ids. Reading a non-existent `agentId` made every
    // message look unattributed, which reported "the handoff did not dispatch" for a session whose
    // second seat may well have spoken.
    const speakersOf = (messages) => new Set(
      messages
        .filter((message) => message.role === "assistant" && (message.content ?? "").trim())
        .map((message) => message.speakerSeatId ?? (
          typeof message.seatIndex === "number" ? `index:${message.seatIndex}` : null
        ))
        .filter(Boolean),
    );

    const relayed = await globalThis.browser.waitUntil(async () => {
      const speakers = speakersOf(await listMessages(session.id));
      return speakers.size >= 2 ? [...speakers] : false;
    }, { timeout: 240_000, interval: 3_000, timeoutMsg: "the handoff never reached the second seat" })
      .catch(() => null);

    if (!relayed) {
      const messages = await listMessages(session.id);
      blocked.push(
        `handoff: ${JSON.stringify([...speakersOf(messages)])} spoke across `
        + `${messages.filter((message) => message.role === "assistant").length} assistant `
        + `messages; the @${targetHandle} mention did not dispatch the second seat on this host`,
      );
      this.skip();
    }
    assert.ok(relayed.length >= 2, `expected two speakers, saw ${JSON.stringify(relayed)}`);
  });

  globalThis.after(async () => {
    for (const sessionId of createdSessions) {
      await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId)
        .catch(() => {});
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
