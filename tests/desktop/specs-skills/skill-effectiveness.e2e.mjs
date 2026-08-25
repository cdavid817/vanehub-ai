import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, realpath, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const CLI_AGENTS = ["claude-code", "codex-cli", "opencode"];
const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];
const SKILL_ID = "desktop-skill-effectiveness";
const SKILL_MARKER = "VANEHUB_SKILL_EFFECTIVENESS_MARKER_8F3C";
const EXPECTED_REPLY = "VANEHUB_SKILL_ACTIVE_8F3C";
const LOCAL_MODEL_ID = "skill-effectiveness-local-model";
const LOCAL_PROFILE_NAME = "Skill effectiveness local model";
const blocked = [];
const sessions = [];

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : null;

let workspace;
let profileId = null;
let localServer;
let localBaseUrl;
const localRequests = [];

async function settleOperation(operation, timeoutMsg) {
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 90_000, interval: 1_000, timeoutMsg });
  assert.equal(settled.status, "succeeded", settled.error ?? timeoutMsg);
}

async function createRepository() {
  if (!fixtureRoot) throw new Error("VANEHUB_APP_DATA_DIR is required for isolated Skill tests.");
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "skill-effectiveness-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "README.md"), "# Skill effectiveness fixture\n", "utf8");
  await run("git", ["add", "README.md"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return realpath(root);
}

async function createSession(agentId, interactionMode, title) {
  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId,
    interactionMode,
    title,
    folder: workspace,
    projectPath: workspace,
    remoteWorkspace: null,
    worktree: null,
  });
  await settleOperation(operation, `Creating ${title} never settled.`);
  const session = await globalThis.browser.waitUntil(async () => {
    const listed = await invoke(({ core }) => core.invoke("list_sessions"));
    return listed.find((entry) => entry.title === title) ?? false;
  }, { timeout: 30_000, interval: 500, timeoutMsg: `${title} was not created.` });
  sessions.push(session.id);
  return session;
}

const listMessages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

async function sendAndAwaitReply(sessionId, agentId, interactionMode, content, speakerSeatId = null) {
  const before = (await listMessages(sessionId)).filter((message) => message.role === "assistant");
  await invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId,
    content,
    config: {
      agentId,
      interactionMode,
      executionMode: "inherit",
      providerId: null,
      modelId: null,
      reasoningDepth: null,
      streaming: true,
      thinking: false,
      longContext: false,
    },
    fileReferences: null,
    runner: null,
  });
  const reply = await globalThis.browser.waitUntil(async () => {
    const assistants = (await listMessages(sessionId)).filter((message) => message.role === "assistant");
    const candidates = assistants.slice(before.length);
    const current = speakerSeatId
      ? candidates.find((message) => message.speakerSeatId === speakerSeatId)
      : candidates[0];
    return ["completed", "failed", "cancelled"].includes(current?.status) ? current : false;
  }, {
    timeout: 4 * 60_000,
    interval: 2_000,
    timeoutMsg: `${agentId} did not finish its Skill verification turn.`,
  });
  assert.equal(reply.status, "completed", reply.error ?? `${agentId} turn did not complete.`);
  return reply;
}

async function startLocalModel() {
  localServer = createServer((request, response) => {
    let body = "";
    request.setEncoding("utf8");
    request.on("data", (chunk) => { body += chunk; });
    request.on("end", () => {
      localRequests.push({ url: request.url, body });
      if (request.url === "/v1/models") {
        response.writeHead(200, { "content-type": "application/json" });
        response.end(JSON.stringify({ data: [{ id: LOCAL_MODEL_ID }] }));
        return;
      }
      if (request.url === "/v1/chat/completions") {
        const content = body.includes(SKILL_MARKER) ? EXPECTED_REPLY : "SKILL_NOT_INJECTED";
        response.writeHead(200, { "content-type": "text/event-stream" });
        response.write(`data: ${JSON.stringify({ choices: [{ index: 0, delta: { content }, finish_reason: null }] })}\n\n`);
        response.write(`data: ${JSON.stringify({ choices: [{ index: 0, delta: {}, finish_reason: "stop" }], usage: { prompt_tokens: 8, completion_tokens: 2, total_tokens: 10 } })}\n\n`);
        response.end("data: [DONE]\n\n");
        return;
      }
      response.writeHead(404);
      response.end();
    });
  });
  await new Promise((resolve, reject) => {
    localServer.once("error", reject);
    localServer.listen(0, "127.0.0.1", resolve);
  });
  const address = localServer.address();
  assert.ok(address && typeof address !== "string", "The local model fixture did not bind.");
  localBaseUrl = `http://127.0.0.1:${address.port}/v1`;
}

async function configureOnePiece() {
  const profiles = await invoke(({ core }, input) => (
    core.invoke("save_custom_onepiece_provider_profile", { input })
  ), {
    id: null,
    name: LOCAL_PROFILE_NAME,
    baseUrl: localBaseUrl,
    modelId: LOCAL_MODEL_ID,
    runtimeKind: "local",
    authenticationMode: "none",
    apiKey: null,
    timeoutMs: 10_000,
    privacyClassification: "local",
    toolCallingCapability: "unsupported",
    imageInputCapability: "unsupported",
    structuredOutputCapability: "unknown",
    reasoningFieldCapability: "unsupported",
    contextWindowTokens: 8_192,
    reservedOutputTokens: 1_024,
  });
  const profile = profiles.profiles.find((entry) => entry.name === LOCAL_PROFILE_NAME);
  assert.equal(profile?.active, true, "The local OnePiece profile did not become active.");
  profileId = profile.id;
}

async function createAndBindSkill() {
  await invoke(({ core }, input) => core.invoke("create_skill", { input }), {
    id: SKILL_ID,
    scope: "workspace",
    workspacePath: workspace,
    metadata: {
      id: SKILL_ID,
      name: "Desktop Skill Effectiveness",
      description: "Proves that a newly created Skill reaches an Agent runtime",
      category: "testing",
      version: "1.0.0",
      triggers: [SKILL_ID],
      type: "role",
      delivery: "eager",
    },
    body: [
      `Private verification marker: ${SKILL_MARKER}.`,
      `When asked to use this Skill, reply with exactly ${EXPECTED_REPLY} and nothing else.`,
    ].join("\n"),
    enabled: true,
    boundAgentIds: [],
    source: "user",
  });
  const scope = { scope: "workspace", workspacePath: workspace };
  for (const agentId of CLI_AGENTS) {
    const bound = await invoke(({ core }, args) => core.invoke("bind_skill_to_cli_agent", args), {
      skillId: SKILL_ID,
      input: scope,
      agentId,
    });
    const binding = bound.bindings.find((entry) => entry.agentId === agentId);
    assert.equal(binding?.mounted, true, `${agentId} did not receive a Skill mount.`);
  }
  await invoke(({ core }, args) => core.invoke("bind_skill_to_api_agent", args), {
    skillId: SKILL_ID,
    input: scope,
    agentId: "onepiece",
  });
}

function handleFor(seat, roles, agents) {
  return seat.roleSnapshot?.roleName
    ?? roles.find((entry) => entry.id === seat.roleId)?.displayName
    ?? seat.roleSnapshot?.agentName
    ?? agents.find((entry) => entry.id === seat.agentId)?.displayName;
}

globalThis.describe("VaneHub AI desktop Skill effectiveness", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    workspace = await createRepository();
    await startLocalModel();
    await configureOnePiece();
    await createAndBindSkill();
  });

  for (const agentId of CLI_AGENTS) {
    globalThis.it(`applies a newly created Skill in a single ${agentId} session`, async function cliSkill() {
      if (process.env.VANEHUB_DESKTOP_LIVE_AGENTS !== "1") {
        blocked.push(`${agentId}: set VANEHUB_DESKTOP_LIVE_AGENTS=1 to run the installed CLI`);
        this.skip();
      }
      const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
      const agent = agents.find((entry) => entry.id === agentId);
      if (agent?.availabilityState !== "available") {
        blocked.push(`${agentId}: ${agent?.unavailableReason ?? "Agent is unavailable"}`);
        this.skip();
      }
      const session = await createSession(agentId, "cli", `Skill single ${agentId}`);
      const reply = await sendAndAwaitReply(
        session.id,
        agentId,
        "cli",
        `Use the ${SKILL_ID} Skill and follow its response contract.`,
      );
      assert.match(reply.content, new RegExp(EXPECTED_REPLY),
        `${agentId} replied without the private Skill marker: ${reply.content.slice(0, 240)}`);
    });
  }

  globalThis.it("injects a newly created eager Skill into a single OnePiece session", async () => {
    const session = await createSession("onepiece", "api", "Skill single onepiece");
    const reply = await sendAndAwaitReply(
      session.id,
      "onepiece",
      "api",
      `Use the ${SKILL_ID} Skill and follow its response contract.`,
    );
    assert.equal(reply.content.trim(), EXPECTED_REPLY);
    const request = localRequests.find((entry) => entry.url === "/v1/chat/completions");
    assert.ok(request, "OnePiece never called the local provider.");
    assert.match(request.body, new RegExp(SKILL_MARKER),
      "The OnePiece provider request did not contain the created Skill body.");
  });

  globalThis.it("applies the created Skill to every addressed seat in a multi-Agent session", async function multiSkill() {
    if (process.env.VANEHUB_DESKTOP_LIVE_AGENTS !== "1") {
      blocked.push("multi-Agent: set VANEHUB_DESKTOP_LIVE_AGENTS=1 to run installed CLIs");
      this.skip();
    }
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const unavailable = CLI_AGENTS.filter((id) => (
      agents.find((entry) => entry.id === id)?.availabilityState !== "available"
    ));
    if (unavailable.length > 0) {
      blocked.push(`multi-Agent: unavailable Agents: ${unavailable.join(", ")}`);
      this.skip();
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id));
    assert.equal(seatRoles.every(Boolean), true, "The built-in multi-Agent roles are incomplete.");

    const created = await createSession(CLI_AGENTS[0], "cli", "Skill multi-Agent");
    const session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: CLI_AGENTS.map((agentId, index) => ({ agentId, roleId: seatRoles[index].id })),
    });
    const seats = session.seats.filter((seat) => !seat.leftAt);
    assert.deepEqual(seats.map((seat) => seat.agentId), CLI_AGENTS);

    for (const seat of seats) {
      const handle = handleFor(seat, roles, agents);
      assert.ok(handle, `${seat.agentId} has no addressable handle.`);
      const reply = await sendAndAwaitReply(
        session.id,
        session.agentId,
        "cli",
        `@${handle}\nUse the ${SKILL_ID} Skill and follow its response contract.`,
        seat.seatId,
      );
      assert.match(reply.content, new RegExp(EXPECTED_REPLY),
        `${seat.agentId} did not apply the shared Skill: ${reply.content.slice(0, 240)}`);
    }
  });

  globalThis.after(async () => {
    for (const sessionId of sessions.reverse()) {
      await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId)
        .catch(() => {});
    }
    if (workspace) {
      await invoke(({ core }, args) => core.invoke("delete_skill", args), {
        skillId: SKILL_ID,
        input: { scope: "workspace", workspacePath: workspace },
      }).catch(() => {});
    }
    if (profileId) {
      await invoke(({ core }, id) => core.invoke("delete_onepiece_provider_profile", {
        profileId: id,
      }), profileId).catch(() => {});
    }
    if (localServer) await new Promise((resolve) => localServer.close(resolve));
    if (blocked.length > 0) globalThis.console.warn(`BLOCKED:\n  ${blocked.join("\n  ")}`);
  });
});
