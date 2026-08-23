import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";

/**
 * Helpers for the native CLI Management layer.
 *
 * Everything here goes through real Tauri IPC. Where a spec asserts on state, it reads the command
 * the frontend reads -- not a mock, not `localStorage`, and not the fixture's own files, except
 * where the point is that the fixture actually changed on disk.
 */

const resultDir = () => process.env.VANEHUB_DESKTOP_RESULT_DIR ?? process.cwd();

export async function readFixture() {
  return JSON.parse(await readFile(path.join(resultDir(), "cli-fixture.json"), "utf8"));
}

/** Every line the fixture's fakes recorded, so a run can prove which binaries answered. */
export async function readInvocations(fixture) {
  const raw = await readFile(fixture.logPath, "utf8");
  return raw
    .split(/\r?\n/)
    .filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line));
}

/**
 * One Tauri command, with a refusal reported as a refusal.
 *
 * The bridge resolves whatever the executed function returns, and a rejected `core.invoke` came
 * back as `undefined` rather than as an error -- which read as "the backend accepted it", the one
 * reading that is never true of a refusal. Catching inside the page and tagging the result is what
 * makes a refused command assertable.
 */
export async function invoke(command, args) {
  const result = await globalThis.browser.tauri.execute(
    async ({ core }, name, payload) => {
      try {
        return { ok: true, value: await core.invoke(name, payload) };
      } catch (error) {
        return { ok: false, error: error instanceof Error ? error.message : error };
      }
    },
    command,
    args ?? {},
  );
  if (!result?.ok) {
    const detail = typeof result?.error === "string" ? result.error : JSON.stringify(result?.error);
    throw new Error(`${command} refused: ${detail}`);
  }
  return result.value;
}

/** The refusal a command produced, or `null` when it was accepted. */
export async function invokeExpectingRefusal(command, args) {
  try {
    await invoke(command, args);
    return null;
  } catch (error) {
    return error instanceof Error ? error.message : String(error);
  }
}

/** Polls one operation to a terminal status and returns it. */
export async function awaitOperation(operationId, { timeout = 120_000 } = {}) {
  let last = null;
  await globalThis.browser.waitUntil(async () => {
    last = await invoke("get_operation_status", { operationId });
    return !["queued", "running"].includes(last.status);
  }, { timeout, timeoutMsg: `Operation ${operationId} never finished; last seen ${JSON.stringify(last)}` });
  return last;
}

/** Runs a full detection pass and returns the snapshots the frontend would read. */
export async function refreshEnvironments({ agentIds = [], forceCatalog = false } = {}) {
  const handle = await invoke("refresh_cli_environment", { agentIds, forceCatalog });
  const operation = await awaitOperation(handle.operationId);
  assert.equal(operation.status, "succeeded", `refresh failed: ${operation.error ?? "no error recorded"}`);
  return invoke("list_cli_environments");
}

export function snapshotOf(snapshots, agentId) {
  const snapshot = snapshots.find((item) => item.agentId === agentId);
  assert.ok(snapshot, `no snapshot for ${agentId}`);
  return snapshot;
}

export function installationOf(snapshot, installationId) {
  const installation = snapshot.installations.find((item) => item.id === installationId);
  assert.ok(installation, `no installation ${installationId} on ${snapshot.agentId}`);
  return installation;
}

/** Asserts every path a snapshot reports came from the fixture, never from the host. */
export function assertPathsAreFixtureOwned(snapshot, fixtureRoot) {
  for (const installation of snapshot.installations) {
    assert.ok(
      installation.executablePath.startsWith(fixtureRoot),
      `${snapshot.agentId} reports a path outside the fixture: ${installation.executablePath}`,
    );
  }
}
