import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { readOnePieceApiKey } from "../helpers/onepiece-credential.mjs";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const mode = process.env.VANEHUB_AGENT_EVALUATION_MODE ?? "fixture-opencode";
const agentId = mode === "live-onepiece" ? "onepiece" : "opencode";
const fixture = mode === "fixture-opencode";
const TERMINAL = new Set([
  "succeeded",
  "task_failed",
  "agent_failed",
  "timed_out",
  "stuck",
  "cancelled",
  "benchmark_error",
]);

let profileId = null;
let cliEnvironmentEvidence = null;

async function bootstrapReady() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
  assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);
}

async function configureOnePiece() {
  const apiKey = readOnePieceApiKey();
  assert.ok(apiKey, "live-onepiece preflight admitted a run without an API key");
  const profiles = await invoke(({ core }, input) => core.invoke(
    "save_onepiece_provider_profile",
    { input },
  ), {
    id: null,
    name: "Agent evaluation DeepSeek",
    providerId: "deepseek",
    endpointType: "openai-chat-completions",
    modelId: "deepseek-v4-flash",
    apiKey,
  });
  const profile = profiles.profiles.find((entry) => entry.name === "Agent evaluation DeepSeek");
  assert.equal(profile?.active, true, "the isolated OnePiece profile did not become active");
  profileId = profile.id;
}

async function openEvaluationCenter() {
  await globalThis.browser.execute(() => {
    globalThis.history.pushState({}, "", "/workspace/quality/evaluations");
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  });
  const center = await globalThis.$('[data-testid="evaluation-center"]');
  await center.waitForExist({ timeout: 60_000 });
  return center;
}

async function selectOnly(center, selectedAgentId) {
  const toggles = await center.$$('fieldset input[type="checkbox"]');
  const ids = [];
  for (const toggle of toggles) {
    const id = (await toggle.getAttribute("data-testid")).replace("evaluation-agent-", "");
    ids.push(id);
    const selected = await toggle.isSelected();
    if (selected !== (id === selectedAgentId)) await toggle.click();
  }
  assert.ok(ids.includes(selectedAgentId), `evaluation picker did not offer ${selectedAgentId}`);
}

async function waitForCreatedArena(knownIds) {
  return globalThis.browser.waitUntil(async () => {
    const arenas = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    return arenas.find((arena) => !knownIds.has(arena.id)) ?? false;
  }, { timeout: 60_000, interval: 1_000, timeoutMsg: "evaluation run created no arena" });
}

async function waitForTerminalArena(arenaId) {
  return globalThis.browser.waitUntil(async () => {
    const arena = await invoke(({ core }, id) => core.invoke("get_evaluation_arena", {
      arenaId: id,
    }), arenaId);
    return arena.attempts.every((attempt) => TERMINAL.has(attempt.outcome)) ? arena : false;
  }, { timeout: 8 * 60_000, interval: 2_000, timeoutMsg: `${agentId} evaluation did not settle` });
}

async function waitForCliEnvironment(selectedAgentId) {
  const handle = await invoke(({ core }, id) => core.invoke("refresh_cli_environment", {
    agentIds: [id],
    forceCatalog: false,
  }), selectedAgentId);
  const operation = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(({ core }, operationId) => core.invoke("get_operation_status", {
      operationId,
    }), handle.operationId);
    return ["queued", "running"].includes(status.status) ? false : status;
  }, { timeout: 120_000, interval: 1_000, timeoutMsg: `${selectedAgentId} CLI refresh did not settle` });
  assert.equal(operation.status, "succeeded", operation.error ?? `${selectedAgentId} CLI refresh failed`);
  const snapshots = await invoke(({ core }) => core.invoke("list_cli_environments"));
  const snapshot = snapshots.find((entry) => entry.agentId === selectedAgentId);
  assert.equal(snapshot?.executable, "healthy", `${selectedAgentId} CLI environment is not healthy`);
  cliEnvironmentEvidence = {
    installationCount: snapshot.installations.length,
    origins: snapshot.installations.map((installation) => installation.environmentOrigin),
    pathPriorities: snapshot.installations.map((installation) => installation.pathPriority),
    conflicts: snapshot.conflicts.map((conflict) => ({
      kind: conflict.kind,
      reasonCode: conflict.reasonCode,
      blocksLaunch: conflict.blocksLaunch,
    })),
  };
}

globalThis.describe(`VaneHub AI focused ${mode} evaluation`, () => {
  globalThis.before(async () => {
    await bootstrapReady();
    if (mode === "live-onepiece") await configureOnePiece();
  });

  globalThis.it(`evaluates only stable Agent id ${agentId} and projects the result`, async () => {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const selected = agents.find((agent) => agent.id === agentId);
    assert.ok(selected, `${agentId} is missing from the native registry`);
    assert.equal(selected.availabilityState, "available", selected.unavailableReason ?? `${agentId} unavailable`);
    if (agentId === "opencode") await waitForCliEnvironment(agentId);

    const center = await openEvaluationCenter();
    await selectOnly(center, agentId);
    const before = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    const knownIds = new Set(before.map((arena) => arena.id));
    const run = await center.$('[data-testid="evaluation-run"]');
    await run.waitForClickable({ timeout: 30_000 });
    await run.click();

    const created = await waitForCreatedArena(knownIds);
    assert.deepEqual(created.attempts.map((attempt) => attempt.agent.agentId), [agentId]);
    const settled = await waitForTerminalArena(created.id);
    assert.equal(settled.attempts.length, 1);
    const attempt = settled.attempts[0];

    await globalThis.browser.waitUntil(async () => {
      const row = await globalThis.$(`[data-testid="evaluation-row"][data-attempt-id="${attempt.id}"]`);
      return (await row.isExisting()) && (await row.getAttribute("data-outcome")) === attempt.outcome;
    }, { timeout: 60_000, interval: 1_000, timeoutMsg: "terminal result never reached the evaluation table" });

    const row = await globalThis.$(`[data-testid="evaluation-row"][data-attempt-id="${attempt.id}"]`);
    await row.click();
    const detail = await center.$('[data-testid="evaluation-detail"]');
    await globalThis.browser.waitUntil(
      async () => (await detail.getAttribute("data-selected-attempt")) === attempt.id,
      { timeout: 30_000, timeoutMsg: "result row did not open its detail" },
    );
    assert.equal(await detail.getAttribute("data-selected-outcome"), attempt.outcome);

    if (attempt.outcome === "benchmark_error") {
      const operation = await invoke(({ core }, operationId) => core.invoke(
        "get_operation_status",
        { operationId },
      ), settled.operationId);
      assert.fail(`evaluation orchestration failed: ${operation.error ?? "reason unavailable"}`);
    }
    if (fixture) {
      assert.notEqual(attempt.outcome, "agent_failed", "OpenCode fixture never reached execution");
    }

    if (attempt.outcome === "agent_failed") {
      const diagnostic = attempt.checks.find((check) => check.checkId === "agent-dispatch");
      assert.ok(diagnostic?.summary, "Agent dispatch failed without a bounded diagnostic");
      assert.match(await detail.getText(), new RegExp(diagnostic.summary.replaceAll(/[.*+?^${}()|[\]\\]/g, "\\$&")));
    } else {
      assert.ok(attempt.checks.length > 0, `${attempt.outcome} carried no deterministic verification`);
    }

    const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
    assert.ok(resultDir, "desktop result directory is unavailable");
    await writeFile(path.join(resultDir, "agent-evaluation-result.json"), `${JSON.stringify({
      schemaVersion: 1,
      mode,
      fixture,
      taskId: settled.taskId,
      taskVersion: settled.taskVersion,
      agentId,
      arenaId: settled.id,
      attemptId: attempt.id,
      outcome: attempt.outcome,
      diagnosticChecks: attempt.checks.filter((check) => check.checkId === "agent-dispatch").length,
      cliEnvironment: cliEnvironmentEvidence,
    }, null, 2)}\n`);
  });

  globalThis.after(async () => {
    if (profileId) {
      await invoke(({ core }, id) => core.invoke("delete_onepiece_provider_profile", {
        profileId: id,
      }), profileId);
    }
  });
});
