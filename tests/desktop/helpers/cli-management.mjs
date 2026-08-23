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

export function invoke(command, args) {
  return globalThis.browser.tauri.execute(
    ({ core }, name, payload) => core.invoke(name, payload),
    command,
    args ?? {},
  );
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
