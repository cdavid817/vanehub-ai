import assert from "node:assert/strict";
import { Buffer } from "node:buffer";
import { execFile } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:http";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { readNativeSettings } from "../helpers/native-settings.mjs";

const run = promisify(execFile);

globalThis.describe("VaneHub AI native desktop smoke", () => {
  globalThis.it("starts the real runtime, crosses IPC, and performs stable navigation", async () => {
    const repository = await mkdtemp(join(tmpdir(), "vanehub-review-e2e-"));
    await run("git", ["init"], { cwd: repository });
    await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: repository });
    await run("git", ["config", "user.name", "Desktop E2E"], { cwd: repository });
    await writeFile(join(repository, "review.txt"), "before\n", "utf8");
    await run("git", ["add", "review.txt"], { cwd: repository });
    await run("git", ["commit", "-m", "fixture"], { cwd: repository });
    await writeFile(join(repository, "review.txt"), "after\n", "utf8");
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
      timeout: 120_000,
      timeoutMsg: "React bootstrap did not become ready.",
    });

    const settings = await readNativeSettings();
    assert.match(settings.applicationLanguage, /^(zh-CN|zh-TW|en|ja|ko)$/);
    const update = await globalThis.browser.tauri.execute(({ core }) => (
      core.invoke("get_desktop_update_snapshot")
    ));
    assert.equal(update.phase, "idle");
    assert.match(update.currentVersion, /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/);
    assert.match(update.channel, /^(stable|preview)$/);

    const localRequests = [];
    const localServer = createServer((request, response) => {
      const chunks = [];
      request.on("data", (chunk) => chunks.push(chunk));
      request.on("end", () => {
        localRequests.push({ method: request.method, url: request.url, body: Buffer.concat(chunks).toString("utf8") });
        if (request.url === "/api/tags" || request.url === "/v1/models") {
          response.writeHead(200, { "content-type": "application/json" });
          response.end(JSON.stringify({ models: [{ name: "desktop-local-model" }] }));
          return;
        }
        if (request.url === "/v1/chat/completions") {
          response.writeHead(200, { "content-type": "text/event-stream" });
          response.write('data: {"choices":[{"index":0,"delta":{"content":"Desktop local response"},"finish_reason":null}]}\n\n');
          response.write('data: {"choices":[{"index":0,"delta":{},"finish_reason":"stop"}],"usage":{"prompt_tokens":4,"completion_tokens":3,"total_tokens":7}}\n\n');
          response.end("data: [DONE]\n\n");
          return;
        }
        response.writeHead(404);
        response.end();
      });
    });
    await new Promise((resolve, reject) => {
      localServer.once("error", reject);
      localServer.listen(11434, "127.0.0.1", resolve);
    });
    const discovery = await globalThis.browser.tauri.execute(({ core }) => core.invoke("discover_local_model_endpoints"));
    assert.ok(discovery.candidates.some((candidate) => candidate.models.includes("desktop-local-model")));
    assert.ok(localRequests.every((request) => request.method === "GET" && request.body === ""));
    const discoveryOperation = await globalThis.browser.tauri.execute(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      discovery.operationId,
    );
    assert.equal(discoveryOperation.status, "succeeded");
    const verified = await globalThis.browser.tauri.execute(({ core }) => core.invoke("verify_local_model_endpoint", {
      input: { baseUrl: "http://127.0.0.1:11434/v1", timeoutMs: 5_000 },
    }));
    assert.deepEqual(verified.candidates[0].models, ["desktop-local-model"]);
    const profiles = await globalThis.browser.tauri.execute(({ core }) => core.invoke("save_custom_onepiece_provider_profile", {
      input: {
        id: null,
        name: "Desktop local model",
        baseUrl: "http://127.0.0.1:11434/v1",
        modelId: "desktop-local-model",
        runtimeKind: "local",
        authenticationMode: "none",
        apiKey: null,
        timeoutMs: 5_000,
        privacyClassification: "local",
        toolCallingCapability: "unsupported",
        imageInputCapability: "unsupported",
        structuredOutputCapability: "unknown",
        reasoningFieldCapability: "unsupported",
        contextWindowTokens: 8_192,
        reservedOutputTokens: 1_024,
      },
    }));
    assert.equal(profiles.profiles.find((profile) => profile.name === "Desktop local model")?.active, true);
    const localSessionOperation = await globalThis.browser.tauri.execute(({ core }, projectPath) => core.invoke("create_session", {
      input: {
        agentId: "onepiece",
        interactionMode: "api",
        title: "Desktop local model fixture",
        folder: projectPath,
        projectPath,
        remoteWorkspace: null,
        worktree: null,
      },
    }), repository);
    const terminalLocalSessionOperation = await globalThis.browser.waitUntil(async () => {
      const operation = await globalThis.browser.tauri.execute(
        ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
        localSessionOperation.id,
      );
      return ["succeeded", "failed", "cancelled"].includes(operation.status) ? operation : false;
    }, { timeout: 30_000, timeoutMsg: "Desktop local session operation did not finish." });
    assert.equal(terminalLocalSessionOperation.status, "succeeded", terminalLocalSessionOperation.error ?? "session creation failed");
    const localSession = await globalThis.browser.waitUntil(async () => {
      const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === "Desktop local model fixture") ?? false;
    }, { timeout: 30_000, timeoutMsg: "Desktop local session was not created." });
    await globalThis.browser.tauri.execute(({ core }, sessionId) => core.invoke("send_message", {
      sessionId,
      content: "Reply from the deterministic local server.",
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
    }), localSession.id);
    await globalThis.browser.waitUntil(async () => {
      const messages = await globalThis.browser.tauri.execute(({ core }, sessionId) => core.invoke("list_messages", {
        sessionId,
        limit: null,
        beforeId: null,
      }), localSession.id);
      return messages.some((message) => message.role === "assistant" && message.content === "Desktop local response");
    }, { timeout: 30_000, timeoutMsg: "Desktop local response did not stream to completion." });
    assert.ok(localRequests.some((request) => request.method === "POST" && request.url === "/v1/chat/completions"));
    await new Promise((resolve, reject) => localServer.close((error) => (error ? reject(error) : resolve())));
    assert.equal(localServer.listening, false);

    // The review fixture used to hardcode codex-cli. That worked only while a missing executable
    // could be overruled by an installed managed SDK: on a runner with no `codex` on PATH the
    // registry called it available anyway, and this spec created a CLI session for a binary that
    // was not there. Availability is now a filesystem probe, so the same call is refused --
    // correctly -- and the fixture has to ask the host what it actually has.
    const hostAgents = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const installedCli = hostAgents.find((agent) => agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli"));
    // Code review needs a session rooted in a project, not a subprocess, so the API Agent this
    // spec has already driven above is a sound stand-in where no CLI is installed.
    const fixtureAgent = installedCli
      ? { agentId: installedCli.id, interactionMode: "cli" }
      : { agentId: "onepiece", interactionMode: "api" };

    // A host without the executable is the only place this contract is observable, so assert it
    // there rather than treating the absence as nothing to test.
    const codex = hostAgents.find((agent) => agent.id === "codex-cli");
    if (codex && codex.availabilityState !== "available") {
      assert.match(
        codex.unavailableReason ?? "",
        /codex/,
        "an Agent whose executable does not resolve must name it, so the user knows what to install",
      );
    }

    const reviewSessionOperation = await globalThis.browser.tauri.execute(({ core }, input) => core.invoke("create_session", {
      input: {
        agentId: input.agentId,
        interactionMode: input.interactionMode,
        title: "Desktop code review fixture",
        folder: input.projectPath,
        projectPath: input.projectPath,
        remoteWorkspace: null,
        worktree: null,
      },
    }), { ...fixtureAgent, projectPath: repository });
    // Settled before the list is polled. Without this the operation's own failure reason -- the
    // runtime logs "Agent is unavailable: Command 'codex' was not found on PATH" -- never reaches
    // the report, and a refused creation is indistinguishable from a slow one.
    const settledReviewSession = await globalThis.browser.waitUntil(async () => {
      const operation = await globalThis.browser.tauri.execute(
        ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
        reviewSessionOperation.id,
      );
      return ["succeeded", "failed", "cancelled"].includes(operation.status) ? operation : false;
    }, { timeout: 30_000, timeoutMsg: "Desktop review session operation did not finish." });
    assert.equal(
      settledReviewSession.status,
      "succeeded",
      settledReviewSession.error ?? `creating a ${fixtureAgent.agentId} review session failed`,
    );
    const session = await globalThis.browser.waitUntil(async () => {
      const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === "Desktop code review fixture") ?? false;
    }, { timeout: 30_000, timeoutMsg: "Desktop review session was not created." });
    const review = await globalThis.browser.tauri.execute(
      ({ core }, sessionId) => core.invoke("open_code_review", { sessionId }),
      session.id,
    );
    assert.equal(review.files.length, 1);
    const diff = await globalThis.browser.tauri.execute(
      ({ core }, input) => core.invoke("load_code_review_file", input),
      { sessionId: session.id, path: "review.txt", expectedSnapshot: review.fingerprint },
    );
    assert.equal(diff.hunks.length, 1);
    await writeFile(join(repository, "review.txt"), "external edit\n", "utf8");
    await assert.rejects(() => globalThis.browser.tauri.execute(
      ({ core }, input) => core.invoke("revert_code_review_change", { input }),
      { sessionId: session.id, path: "review.txt", expectedSnapshot: review.fingerprint, hunkFingerprint: diff.hunks[0].fingerprint, confirmed: true },
    ));

    const agentRun = await globalThis.browser.tauri.execute(({ core }) => core.invoke("create_agent_run", {
      input: {
        id: randomUUID(),
        owner: { ownerType: "desktop_agent_operation", ownerId: "desktop-smoke" },
        links: [],
        parentRunId: null,
        recoveryPolicy: "not_recoverable",
        maxRetries: 0,
        witness: "desktop-smoke-created",
      },
    }));
    assert.equal(agentRun.state, "created");
    const missionOverview = await globalThis.browser.tauri.execute(({ core }) => core.invoke("get_mission_control_overview", {
      query: { limit: 20, sort: "newest" },
    }));
    assert.ok(missionOverview.active.items.some((run) => run.runId === agentRun.id && run.state === "created"));
    const cancelledRun = await globalThis.browser.tauri.execute(({ core }, input) => core.invoke("cancel_agent_run", input), {
      runId: agentRun.id,
      version: agentRun.version,
    });
    assert.equal(cancelledRun.state, "cancelled");
    const missionDetail = await globalThis.browser.tauri.execute(({ core }, runId) => core.invoke("get_mission_control_run", { runId }), agentRun.id);
    assert.equal(missionDetail.run.state, "cancelled");
    assert.equal(missionDetail.run.endedAt, cancelledRun.updatedAt);
    const runEvents = await globalThis.browser.tauri.execute(({ core }, runId) => core.invoke("list_agent_run_events", {
      runId,
      offset: 0,
      limit: 10,
    }), agentRun.id);
    assert.deepEqual(runEvents.map((event) => event.state), ["created", "cancelled"]);

    const evaluationTasks = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_evaluation_tasks"));
    assert.ok(evaluationTasks.length >= 3);
    const agents = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const installed = agents.find((agent) => agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli"));
    if (!installed) {
      globalThis.console.warn("BLOCKED: native evaluation requires one installed managed CLI Agent");
    } else {
      const evaluation = await globalThis.browser.tauri.execute(({ core }, agentId) => core.invoke("start_evaluation", {
        input: { taskId: "add-parser-test", taskVersion: 1, agentIds: [agentId] },
      }), installed.id);
      assert.equal(evaluation.attempts.length, 1);
      const terminalEvaluation = await globalThis.browser.waitUntil(async () => {
        const arenas = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_evaluation_arenas"));
        const current = arenas.find((arena) => arena.id === evaluation.id);
        return current && !["queued", "running"].includes(current.attempts[0].outcome) ? current : false;
      }, { timeout: 150_000, timeoutMsg: "Installed-Agent evaluation did not reach a terminal state." });
      const exportedEvaluation = await globalThis.browser.tauri.execute(
        ({ core }, arenaId) => core.invoke("export_evaluation", { arenaId }), terminalEvaluation.id,
      );
      assert.equal(exportedEvaluation.schemaVersion, 1);
      assert.equal(exportedEvaluation.arena.attempts[0].agent.agentId, installed.id);
    }

    const settingsButton = await globalThis.$('[data-testid="desktop-smoke-settings"]');
    await settingsButton.waitForClickable();
    await settingsButton.click();
    await globalThis.browser.waitUntil(async () => (await globalThis.browser.getUrl()).includes("/settings"), {
      timeoutMsg: "The native WebView did not navigate to settings.",
    });
    const settingsRoot = await globalThis.$("main");
    await settingsRoot.waitForExist();
    assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);

    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
    await rm(repository, { recursive: true, force: true });
  });
});
