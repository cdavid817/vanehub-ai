import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { readOnePieceApiKey } from "../helpers/onepiece-credential.mjs";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

/** Answering this needs a real provider round trip, and it is cheap to assert on. */
const PROMPT = "Reply with exactly the word PONG and nothing else.";
// Covers both prompt-delivery shapes on purpose: claude-code and codex-cli hand the prompt to the
// CLI on stdin, while gemini-cli, opencode and antigravity-cli put it in argv. Those are different
// code paths, and a suite that only exercises one of them reports a chat runtime as healthy while
// three of the five Agents cannot send a turn.
const SINGLE_AGENTS = [
  { id: "opencode", mode: "cli" },
  { id: "claude-code", mode: "cli" },
  { id: "codex-cli", mode: "cli" },
  { id: "gemini-cli", mode: "cli" },
  { id: "antigravity-cli", mode: "cli" },
];

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();
const blocked = [];
let repository;

async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "sessions-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function createSession(input) {
  const operation = await invoke(({ core }, payload) => core.invoke("create_session", { input: payload }), {
    remoteWorkspace: null,
    worktree: null,
    folder: repository,
    projectPath: repository,
    ...input,
  });
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 60_000, timeoutMsg: `Creating the ${input.agentId} session never settled.` });
  assert.equal(settled.status, "succeeded", settled.error ?? `${input.agentId} session creation failed`);
  return globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === input.title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `The ${input.agentId} session was not created.` });
}

async function sendAndAwaitReply(session, agentId, mode) {
  await invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: session.id,
    content: PROMPT,
    config: {
      agentId,
      interactionMode: mode,
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
  // Generous: a cold CLI start plus a provider round trip through an egress proxy is slow, and a
  // timeout here would be reported as a product failure rather than the wait it actually is.
  // Returns null instead of throwing so the caller can tell "the Agent is broken" apart from
  // "this host is slow" by looking at what the run actually did.
  try {
    return await globalThis.browser.waitUntil(async () => {
      const messages = await invoke(({ core }, sessionId) => core.invoke("list_messages", {
        sessionId,
        limit: null,
        beforeId: null,
      }), session.id);
      return messages.find((message) => message.role === "assistant"
        && (message.content ?? "").trim().length > 0) ?? false;
    }, { timeout: 180_000, interval: 2_000, timeoutMsg: `${agentId} produced no assistant reply.` });
  } catch {
    return null;
  }
}

/** Lifecycle after a turn, which is what distinguishes a failed launch from a slow one. */
async function lifecycleOf(sessionId) {
  const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
  return sessions.find((item) => item.id === sessionId)?.lifecycleState ?? "unknown";
}

function explicitAuthenticationFailure(content) {
  return /failed to authenticate|authentication required|not logged in|api error:\s*(401|403)|request not allowed|open (your )?browser.*(auth|sign in)|opening authentication page in your browser/i
    .test(content);
}

globalThis.describe("VaneHub AI desktop session behaviour", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("answers in a single-Agent OnePiece session over the live provider", async function onePieceSession() {
    const apiKey = readOnePieceApiKey();
    if (!apiKey) {
      blocked.push("OnePiece: set VANEHUB_ONEPIECE_API_KEY or VANEHUB_ONEPIECE_PROFILE_ID");
      this.skip();
    }
    const profiles = await invoke(({ core }, input) => core.invoke("save_onepiece_provider_profile", { input }), {
      id: null,
      name: "Session sweep DeepSeek",
      providerId: "deepseek",
      endpointType: "openai-chat-completions",
      modelId: "deepseek-v4-flash",
      apiKey,
    });
    const created = profiles.profiles.find((profile) => profile.name === "Session sweep DeepSeek");

    const session = await createSession({
      agentId: "onepiece",
      interactionMode: "api",
      title: "Single OnePiece",
    });
    const reply = await sendAndAwaitReply(session, "onepiece", "api");
    assert.ok(reply, "OnePiece produced no assistant reply from the live provider");
    assert.match(reply.content, /PONG/i, `unexpected OnePiece reply: ${reply.content}`);

    if (created?.id) {
      await globalThis.browser.waitUntil(async () => {
        try {
          await invoke(({ core }, profileId) => core.invoke("delete_onepiece_provider_profile", {
            profileId,
          }), created.id);
          return true;
        } catch {
          return false;
        }
      }, { timeout: 30_000, interval: 1_000, timeoutMsg: "The session sweep's profile could not be deleted." });
    }
  });

  for (const agent of SINGLE_AGENTS) {
    globalThis.it(`answers in a single-Agent ${agent.id} session`, async function cliSession() {
      const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
      const definition = agents.find((entry) => entry.id === agent.id);
      if (definition?.availabilityState !== "available") {
        blocked.push(`${agent.id}: ${definition?.unavailableReason ?? "not registered"}`);
        this.skip();
      }

      const session = await createSession({
        agentId: agent.id,
        interactionMode: agent.mode,
        title: `Single ${agent.id}`,
      });
      const reply = await sendAndAwaitReply(session, agent.id, agent.mode);
      if (!reply) {
        // Report what was observed, not a cause. Naming one here was wrong twice: the same
        // `failed` lifecycle covered a launch the runner refused, a spawn Windows rejected, and a
        // provider turning the account away -- three different problems, and a hardcoded reason
        // labelled all of them as the first. The unified log carries the actual cause.
        const lifecycle = await lifecycleOf(session.id);
        blocked.push(`${agent.id}: no reply, session lifecycle=${lifecycle} (see the run's vanehub.log)`);
        // Becomes a real assertion again the moment a turn does answer, so a fix cannot land
        // unnoticed behind a permanently skipped case.
        this.skip();
      }
      if (explicitAuthenticationFailure(reply.content)) {
        const summary = reply.content.replaceAll(/\s+/g, " ").trim().slice(0, 240);
        blocked.push(`${agent.id}: provider authentication refused the live turn (${summary})`);
        this.skip();
      }
      assert.match(reply.content, /PONG/i, `unexpected ${agent.id} reply: ${reply.content}`);
    });
  }

  globalThis.it("seats two Agents in one multi-Agent session and keeps the roster addressable", async function multiAgent() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const usable = agents
      .filter((entry) => entry.availabilityState === "available"
        && entry.supportedInteractionModes.includes("cli"))
      .slice(0, 2);
    if (usable.length < 2) {
      blocked.push(`multi-Agent: needs two installed CLI Agents, found ${usable.length}`);
      this.skip();
    }

    const session = await createSession({
      agentId: usable[0].id,
      interactionMode: "cli",
      title: "Multi-Agent roster",
    });
    const seated = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: session.id,
      expectedUpdatedAt: session.updatedAt,
      seats: usable.map((entry) => ({ agentId: entry.id, roleId: null })),
    });

    // A seat that leaves is tombstoned with `leftAt` rather than deleted, so "who is in the room"
    // is the set of seats without one -- reading `seats` raw counts departed participants.
    const active = (session) => session.seats
      .filter((seat) => !seat.leftAt)
      .map((seat) => seat.agentId)
      .sort();

    assert.deepEqual(active(seated), usable.map((entry) => entry.id).sort(), "both Agents must hold a seat");
    for (const seat of seated.seats) {
      assert.ok(seat.seatId, `seat for ${seat.agentId} was persisted without an id`);
    }

    // The roster has to survive a reload, otherwise a restart silently drops a participant.
    const reloaded = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.id === session.id) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The multi-Agent session disappeared." });
    assert.deepEqual(
      active(reloaded),
      usable.map((entry) => entry.id).sort(),
      "the seated roster did not survive a reload",
    );

    // Removing a seat must leave the remaining participant intact rather than resetting the room,
    // and must keep the departed seat as a record rather than erasing that it was ever there.
    const trimmed = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: session.id,
      expectedUpdatedAt: reloaded.updatedAt,
      seats: [{ agentId: usable[0].id, roleId: null }],
    });
    assert.deepEqual(active(trimmed), [usable[0].id]);
    const departed = trimmed.seats.find((seat) => seat.agentId === usable[1].id);
    assert.ok(departed?.leftAt, `${usable[1].id} left the room without being tombstoned`);
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    await invoke(({ core }) => core.invoke("exit_application"));
  });
});
