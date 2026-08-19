import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { readOnePieceApiKey } from "../helpers/onepiece-credential.mjs";

const run = promisify(execFile);
const BUILTIN_CLI_AGENTS = ["claude-code", "codex-cli", "gemini-cli", "opencode", "antigravity-cli"];
const blocked = [];

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

async function bootstrapReady() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
  return root;
}

async function createRepository(label) {
  const repository = await mkdtemp(join(tmpdir(), `vanehub-${label}-`));
  await run("git", ["init"], { cwd: repository });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: repository });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: repository });
  await writeFile(join(repository, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: repository });
  await run("git", ["commit", "-m", "fixture"], { cwd: repository });
  return repository;
}

async function createSession(agentId, interactionMode, title, projectPath) {
  await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId,
    interactionMode,
    title,
    folder: projectPath,
    projectPath,
    remoteWorkspace: null,
    worktree: null,
  });
  return globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `Session "${title}" was not created.` });
}

globalThis.describe("VaneHub AI desktop feature sweep", () => {
  globalThis.it("reports availability for every built-in CLI Agent", async () => {
    await bootstrapReady();
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const byId = new Map(agents.map((agent) => [agent.id, agent]));

    for (const id of BUILTIN_CLI_AGENTS) {
      const agent = byId.get(id);
      assert.ok(agent, `built-in Agent ${id} is missing from the native registry`);
      assert.ok(
        ["available", "unavailable"].includes(agent.availabilityState),
        `${id} reported an unexpected availability state: ${agent.availabilityState}`,
      );
      if (agent.availabilityState !== "available") {
        // Detection ran and answered; the CLI simply is not installed on this host. That is a
        // host fact, not a product failure, so it is recorded rather than asserted away.
        blocked.push(`${id}: ${agent.unavailableReason ?? "unavailable"}`);
      }
    }
    assert.ok(byId.has("onepiece"), "the built-in OnePiece Agent is missing");
  });

  globalThis.it("drives a real CLI Agent terminal through open, input, resize, and stop", async function driveTerminal() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const cli = agents.find((agent) => agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli"));
    if (!cli) {
      // Reported as skipped rather than passed: a green tick for a case that never ran reads as
      // coverage this host does not have.
      blocked.push("agent terminal: no installed CLI Agent on this host");
      this.skip();
    }

    const repository = await createRepository("terminal");
    try {
      const session = await createSession(cli.id, "cli", `Terminal sweep ${cli.id}`, repository);
      // Capture the event stream before opening, so the transcript replayed on attach is seen.
      // `browser.tauri.execute` only injects `core.invoke`; the event API is reachable through
      // the global the desktop-e2e config enables (`withGlobalTauri`).
      await globalThis.browser.execute(() => {
        globalThis.__terminalEvents = [];
        globalThis.__TAURI__.event.listen("agent-terminal:event", (message) => {
          globalThis.__terminalEvents.push(message.payload);
        });
      });

      const terminal = await invoke(({ core }, sessionId) => core.invoke("open_agent_terminal", {
        sessionId,
        size: { rows: 24, cols: 80 },
      }), session.id);
      assert.ok(terminal.terminalId, "open_agent_terminal returned no terminal id");

      // A real PTY: the CLI's own banner or prompt has to reach the transcript.
      await globalThis.browser.waitUntil(async () => {
        const events = await globalThis.browser.execute(() => globalThis.__terminalEvents ?? []);
        return events.some((item) => item.type === "output" && item.content.length > 0);
      }, { timeout: 90_000, timeoutMsg: `${cli.id} produced no PTY output.` });

      await invoke(({ core }, terminalId) => core.invoke("send_agent_terminal_input", {
        terminalId,
        content: "\r",
      }), terminal.terminalId);
      await invoke(({ core }, terminalId) => core.invoke("resize_agent_terminal", {
        terminalId,
        size: { rows: 40, cols: 120 },
      }), terminal.terminalId);

      const stopped = await invoke(({ core }, terminalId) => core.invoke("stop_agent_terminal", {
        terminalId,
      }), terminal.terminalId);
      assert.equal(stopped, true, "stop_agent_terminal did not report the terminal as stopped");
      // Reclamation: a second stop must be a no-op rather than an error or a hang.
      const repeated = await invoke(({ core }, terminalId) => core.invoke("stop_agent_terminal", {
        terminalId,
      }), terminal.terminalId);
      assert.equal(repeated, false, "a stopped terminal is still registered");
    } finally {
      await rm(repository, { recursive: true, force: true });
    }
  });

  globalThis.it("sends a real OnePiece message through the configured provider", async function sendLiveMessage() {
    const apiKey = readOnePieceApiKey();
    if (!apiKey) {
      blocked.push("OnePiece live provider: set VANEHUB_ONEPIECE_API_KEY or VANEHUB_ONEPIECE_PROFILE_ID");
      this.skip();
    }

    // A remote provider goes through the preset catalog, not the custom-endpoint command:
    // `save_custom_onepiece_provider_profile` deliberately refuses any runtime that is not local
    // or private. The preset ids mirror the installed profile's `source_*` columns.
    const profiles = await invoke(({ core }, input) => core.invoke("save_onepiece_provider_profile", {
      input,
    }), {
      id: null,
      name: "Desktop sweep DeepSeek",
      providerId: "deepseek",
      endpointType: "openai-chat-completions",
      modelId: "deepseek-v4-flash",
      apiKey,
    });
    assert.equal(
      profiles.profiles.find((profile) => profile.name === "Desktop sweep DeepSeek")?.active,
      true,
      "the DeepSeek profile did not become active",
    );

    const created = profiles.profiles.find((profile) => profile.name === "Desktop sweep DeepSeek");
    const repository = await createRepository("onepiece");
    try {
      const session = await createSession("onepiece", "api", "OnePiece live sweep", repository);
      await invoke(({ core }, sessionId) => core.invoke("send_message", {
        sessionId,
        content: "Reply with exactly the word: PONG",
        config: {
          agentId: "onepiece",
          interactionMode: "api",
          executionMode: "inherit",
          providerId: null,
          modelId: null,
          reasoningDepth: null,
          streaming: true,
          thinking: false,
          longContext: false,
        },
        fileReferences: null,
      }), session.id);

      const reply = await globalThis.browser.waitUntil(async () => {
        const messages = await invoke(({ core }, sessionId) => core.invoke("list_messages", {
          sessionId,
          limit: null,
          beforeId: null,
        }), session.id);
        return messages.find((message) => message.role === "assistant"
          && (message.content ?? "").trim().length > 0) ?? false;
      }, { timeout: 120_000, timeoutMsg: "OnePiece produced no assistant reply from the live provider." });
      assert.match(reply.content, /PONG/i, `unexpected live reply: ${reply.content}`);
    } finally {
      // Spec files in one desktop run share a single isolated data directory, so an active
      // provider profile left here would decide which profile the next spec finds active.
      if (created?.id) {
        await invoke(({ core }, profileId) => core.invoke("delete_onepiece_provider_profile", {
          profileId,
        }), created.id);
      }
      await rm(repository, { recursive: true, force: true });
    }
  });

  globalThis.it("persists sessions and messages in the native database", async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    assert.ok(sessions.length > 0, "no session survived in the native database");
    const usage = await invoke(({ core }) => core.invoke("get_usage_statistics", { range: "all" }));
    assert.equal(usage.range, "all");
    assert.ok(Number.isInteger(usage.countedSessions), "usage statistics did not report a session count");

    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED items on this host:\n  ${blocked.join("\n  ")}`);
    }
    await invoke(({ core }) => core.invoke("exit_application"));
  });
});
