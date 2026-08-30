import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { createServer } from "node:http";
import { mkdir, mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const MCP_NAME = "desktop-agent-mcp";
const MCP_FIXTURE = join(process.cwd(), "src-tauri", "tests", "fixtures", "mcp_stdio_server.cjs");
const CLI_AGENTS = ["claude-code", "codex-cli", "opencode"];
const EVIDENCE_DIR = join(process.env.VANEHUB_DESKTOP_RESULT_DIR, "agent-mcp-evidence");
let repository;
let onePieceServer;
let onePieceBaseUrl;
const onePieceRequests = [];

async function settle(operation, message) {
  const result = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, id) => core.invoke("get_operation_status", { operationId: id }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 120_000, interval: 500, timeoutMsg: message });
  assert.equal(result.status, "succeeded", result.error ?? message);
}

async function createSession(agentId, interactionMode, title) {
  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId,
    interactionMode,
    title,
    folder: repository,
    projectPath: repository,
    remoteWorkspace: null,
    worktree: null,
  });
  await settle(operation, `Creating ${title} never settled`);
  return globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((session) => session.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `${title} was not persisted` });
}

function sendMessage(session, content) {
  return invoke(({ core }, payload) => core.invoke("send_message", payload), {
    sessionId: session.id,
    content,
    config: {
      agentId: session.agentId,
      interactionMode: session.interactionMode,
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
}

const messages = (sessionId) => invoke(({ core }, id) => core.invoke("list_messages", {
  sessionId: id,
  limit: null,
  beforeId: null,
}), sessionId);

async function waitForCompletedAssistant(sessionId, before, marker) {
  return globalThis.browser.waitUntil(async () => {
    const assistants = (await messages(sessionId)).filter((message) => message.role === "assistant");
    const next = assistants[before];
    return next?.status === "completed" && next.content.includes(marker) ? next : false;
  }, { timeout: 120_000, interval: 500, timeoutMsg: `No completed reply contained ${marker}` });
}

function startOnePieceFixture() {
  onePieceServer = createServer((request, response) => {
    const chunks = [];
    request.on("data", (chunk) => chunks.push(chunk));
    request.on("end", () => {
      const body = Buffer.concat(chunks).toString("utf8");
      onePieceRequests.push(body);
      response.writeHead(200, { "content-type": "text/event-stream" });
      if (onePieceRequests.length === 1) {
        const parsed = JSON.parse(body);
        const tools = parsed.tools?.map((tool) => tool.function?.name) ?? [];
        assert.ok(tools.includes(`mcp__${MCP_NAME}__fixture_echo`), "OnePiece did not receive the MCP catalog");
        response.write(`data: ${JSON.stringify({ choices: [{ index: 0, delta: { tool_calls: [{
          index: 0,
          id: "onepiece-mcp-call",
          type: "function",
          function: { name: `mcp__${MCP_NAME}__fixture_echo`, arguments: "{\"text\":\"onepiece-mcp-effective\"}" },
        }] }, finish_reason: null }] })}\n\n`);
      } else {
        assert.ok(body.includes("echo: onepiece-mcp-effective"), "MCP result did not return to OnePiece");
        response.write('data: {"choices":[{"index":0,"delta":{"content":"MCP_EFFECTIVE onepiece echo: onepiece-mcp-effective"},"finish_reason":null}]}\n\n');
        response.write('data: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}\n\n');
      }
      response.end("data: [DONE]\n\n");
    });
  });
  return new Promise((resolve, reject) => {
    onePieceServer.once("error", reject);
    onePieceServer.listen(0, "127.0.0.1", () => {
      const address = onePieceServer.address();
      onePieceBaseUrl = `http://127.0.0.1:${address.port}/v1`;
      resolve();
    });
  });
}

globalThis.describe("VaneHub Agent MCP desktop runtime", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    try {
      await globalThis.browser.waitUntil(
        async () => globalThis.browser.execute(() =>
          globalThis.document.getElementById("root")?.dataset.vanehubBootstrap === "ready"),
        { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready" },
      );
    } catch (error) {
      const diagnostic = await globalThis.browser.execute(() => {
        const currentRoot = globalThis.document.getElementById("root");
        return {
          bootstrap: currentRoot?.dataset.vanehubBootstrap ?? null,
          fatalError: currentRoot?.dataset.vanehubFatalError ?? null,
          fatalDetail: currentRoot?.dataset.vanehubFatalErrorDetail ?? null,
          text: currentRoot?.textContent?.slice(0, 500) ?? null,
          scripts: [...globalThis.document.scripts].map((script) => script.src),
        };
      });
      throw new Error(`${error.message}: ${JSON.stringify(diagnostic)}`, { cause: error });
    }
    // Wait for CLI detection to finish before anything asks whether an Agent is available.
    // Creating a session consults the environment snapshot, and the refresh that builds it starts
    // in the background at launch: querying before it lands reports whichever Agents happened to
    // resolve already and `Command 'codex' was not found on PATH` for the rest, with the fixture
    // binaries sitting on PATH the whole time.
    const detection = await invoke(({ core }, agentIds) => core.invoke("refresh_cli_environment", {
      agentIds,
      forceCatalog: false,
    }), CLI_AGENTS);
    await settle({ id: detection.operationId }, "CLI detection never settled");

    repository = await mkdtemp(join(tmpdir(), "vanehub-agent-mcp-"));
    await startOnePieceFixture();
    const settings = await invoke(({ core }) => core.invoke("get_observability_settings"));
    await invoke(({ core }, value) => core.invoke("update_observability_settings", { settings: value }), {
      ...settings,
      mcpRelayEnabled: false,
      otlpAuthToken: null,
    });
    // WDIO retries relaunch the app against the same isolated database. If an earlier attempt
    // persisted the server but failed while starting its process, make setup idempotent so the
    // retry tests the original failure instead of stopping at a duplicate-name validation error.
    await invoke(({ core }, name) => core.invoke("remove_mcp_server", { name }), MCP_NAME)
      .catch(() => undefined);
    await invoke(({ core }, config) => core.invoke("add_mcp_server", { config }), {
      name: MCP_NAME,
      transportType: "stdio",
      // Agent isolation may deliberately remove the host directory containing `node` from PATH
      // when it also contains a real managed Agent. The current runner is the exact runtime that
      // should execute this JavaScript fixture, so address it directly.
      command: process.execPath,
      args: [MCP_FIXTURE, "normal"],
      env: {},
      url: null,
      headers: null,
      description: "Agent MCP desktop fixture",
      active: true,
      scope: "user",
      projectPath: null,
    });
    const operation = await invoke(({ core }, name) => core.invoke("test_mcp_connection", { name }), MCP_NAME);
    await settle(operation, "MCP connection test never settled");
    const status = await invoke(({ core }, name) => core.invoke("get_mcp_server_status", { name }), MCP_NAME);
    assert.deepEqual(status.tools.map((tool) => tool.name), ["fixture_echo"]);
  });

  for (const agentId of CLI_AGENTS) {
    globalThis.it(`uses the created MCP from ${agentId}`, async () => {
      const session = await createSession(agentId, "cli", `MCP single ${agentId}`);
      await sendMessage(session, "Use the managed MCP fixture once.");
      await waitForCompletedAssistant(session.id, 0, `MCP_EFFECTIVE ${agentId === "claude-code" ? "claude" : agentId === "codex-cli" ? "codex" : "opencode"}`);
    });
  }

  globalThis.it("uses the created MCP from OnePiece after explicit approval", async () => {
    const profiles = await invoke(({ core }, input) => core.invoke("save_custom_onepiece_provider_profile", { input }), {
      id: null,
      name: "Agent MCP OnePiece fixture",
      baseUrl: onePieceBaseUrl,
      modelId: "agent-mcp-fixture",
      runtimeKind: "local",
      authenticationMode: "none",
      apiKey: null,
      timeoutMs: 10_000,
      privacyClassification: "local",
      toolCallingCapability: "supported",
      imageInputCapability: "unsupported",
      structuredOutputCapability: "unknown",
      reasoningFieldCapability: "unsupported",
      contextWindowTokens: 8_192,
      reservedOutputTokens: 1_024,
    });
    assert.ok(profiles.profiles.some((profile) => profile.name === "Agent MCP OnePiece fixture" && profile.active));
    const session = await createSession("onepiece", "api", "MCP single onepiece");
    await sendMessage(session, "Use the managed MCP fixture once.");
    const pending = await globalThis.browser.waitUntil(async () => {
      const approvals = await invoke(({ core }) => core.invoke("list_pending_approvals"));
      return approvals.find((approval) => approval.callId === "onepiece-mcp-call") ?? false;
    }, { timeout: 30_000, timeoutMsg: "OnePiece MCP approval was not requested" });
    await invoke(({ core }, input) => core.invoke("resolve_pending_approval", { input }), {
      requestId: pending.id,
      approved: true,
      scope: "once",
    });
    await waitForCompletedAssistant(session.id, 0, "MCP_EFFECTIVE onepiece");
    await mkdir(EVIDENCE_DIR, { recursive: true });
    await writeFile(join(EVIDENCE_DIR, "onepiece.json"), `${JSON.stringify({ requests: onePieceRequests.length })}\n`);
  });

  globalThis.it("keeps the created MCP effective for every heterogeneous multi-Agent seat", async () => {
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const selectedRoles = ["builtin-architect", "builtin-implementer", "builtin-reviewer"]
      .map((id) => roles.find((role) => role.id === id));
    assert.ok(selectedRoles.every(Boolean), "Three built-in roles are required");
    const created = await createSession(CLI_AGENTS[0], "cli", "MCP multi Agent");
    const session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: CLI_AGENTS.map((agentId, index) => ({ agentId, roleId: selectedRoles[index].id })),
    });
    const markers = ["claude", "codex", "opencode"];
    for (const [index, role] of selectedRoles.entries()) {
      const before = (await messages(session.id)).filter((message) => message.role === "assistant").length;
      const handle = role.displayName.split(/\s+/u).filter(Boolean).join("-");
      await sendMessage(session, `@${handle} Use the managed MCP fixture once.`);
      await waitForCompletedAssistant(session.id, before, `MCP_EFFECTIVE ${markers[index]}`);
    }
    for (const marker of markers) {
      const content = await readFile(join(EVIDENCE_DIR, `${marker}.jsonl`), "utf8");
      assert.ok(content.trim().split("\n").length >= 2, `${marker} lacked single and multi-Agent MCP evidence`);
    }
  });

  globalThis.after(async () => {
    onePieceServer?.closeAllConnections?.();
    await new Promise((resolve) => onePieceServer?.close(resolve) ?? resolve());
    await invoke(({ core }) => core.invoke("exit_application"));
  });
});
