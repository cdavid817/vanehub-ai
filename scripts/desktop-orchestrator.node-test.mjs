import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { resolveDesktopArtifact } from "./desktop/artifact.mjs";
import { collectUnifiedLogs, writeRunSummary } from "./desktop/evidence.mjs";
import { detectHost } from "./desktop/platform.mjs";
import { ensureOwnedProcessesStopped, ownedProcessIds } from "./desktop/process-ownership.mjs";
import { createLayerResult, verificationExitCode } from "./desktop/result.mjs";
import { createRunContext, validateIsolatedDataPath } from "./desktop/run-context.mjs";

test("maps every supported host and blocks unsupported hosts", () => {
  assert.equal(detectHost("win32", "x64").targetTriple, "x86_64-pc-windows-msvc");
  assert.equal(detectHost("darwin", "arm64").targetTriple, "aarch64-apple-darwin");
  assert.equal(detectHost("linux", "x64").platform, "linux");
  assert.throws(() => detectHost("freebsd", "x64"), (error) => error.status === "BLOCKED");
});

test("resolves only a fresh metadata-derived test artifact", () => {
  const host = detectHost("win32", "x64");
  const metadata = { targetDirectory: "D:/target", binaryName: "vanehub-ai", productName: "VaneHub AI" };
  const artifact = resolveDesktopArtifact({
    metadata,
    host,
    buildStartedAt: 1_000,
    stat: (candidate) => ({ isFile: () => candidate.endsWith("vanehub-ai.exe"), mtimeMs: 1_500 }),
  });
  assert.match(artifact.executablePath, /x86_64-pc-windows-msvc[\\/]debug[\\/]vanehub-ai\.exe$/);
  assert.equal(artifact.testBuild, true);
  assert.throws(
    () => resolveDesktopArtifact({ metadata, host, buildStartedAt: 2_000, stat: () => ({ isFile: () => true, mtimeMs: 1 }) }),
    /predates the current build/,
  );
  assert.throws(() => resolveDesktopArtifact({ metadata, host, stat: () => { throw new Error("missing"); } }), /not produced/);
});

test("creates isolated run paths and rejects unsafe aliases", async () => {
  const tempRoot = await mkdtemp(path.join(os.tmpdir(), "vanehub-context-test-"));
  const repoRoot = path.join(tempRoot, "repo");
  const context = await createRunContext(repoRoot, { tempRoot, runId: "run-1", normalDataDir: path.join(tempRoot, "normal") });
  assert.equal(path.isAbsolute(context.dataDir), true);
  assert.equal(context.environment.VANEHUB_TEST_RUN_ID, "run-1");
  assert.equal(context.resultDir, path.join(repoRoot, "test-results", "desktop", "run-1"));
  assert.throws(
    () => validateIsolatedDataPath({ runRoot: context.runRoot, dataDir: tempRoot, normalDataDir: null }),
    (error) => error.status === "BLOCKED",
  );
  assert.throws(
    () => validateIsolatedDataPath({ runRoot: context.runRoot, dataDir: context.dataDir, normalDataDir: context.dataDir }),
    /aliases normal application data/,
  );
  await rm(tempRoot, { recursive: true, force: true });
});

test("uses explicit result states and failure exit codes", () => {
  assert.equal(verificationExitCode("PASSED"), 0);
  assert.equal(verificationExitCode("FAILED"), 1);
  assert.equal(verificationExitCode("BLOCKED"), 1);
  assert.throws(() => createLayerResult({ layer: "desktop", status: "NOT REQUIRED" }), /include a reason/);
  assert.equal(createLayerResult({ layer: "desktop", status: "NOT REQUIRED", reason: "Docs only" }).reason, "Docs only");
});

test("derives only descendants from the owned root and rejects uncertain ownership", async () => {
  const processes = [
    { pid: 10, parentPid: 1 },
    { pid: 11, parentPid: 10 },
    { pid: 12, parentPid: 11 },
    { pid: 99, parentPid: 1 },
  ];
  assert.deepEqual(ownedProcessIds(10, processes).sort((left, right) => left - right), [10, 11, 12]);
  await assert.rejects(
    ensureOwnedProcessesStopped({ marker: { pid: 10, runId: "another-run" }, runId: "run-1" }),
    /ownership could not be proven/,
  );
});

test("collects only bounded unified logs and indexes unavailable evidence", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "vanehub-evidence-test-"));
  const dataDir = path.join(root, "data");
  const resultDir = path.join(root, "results");
  await mkdir(path.join(dataDir, "logs"), { recursive: true });
  const content = `redacted=true\n${"x".repeat(1024 * 1024 + 128)}`;
  await writeFile(path.join(dataDir, "logs", "vanehub.log"), content);
  await writeFile(path.join(dataDir, "database.sqlite"), "must-not-copy");

  const collected = await collectUnifiedLogs(dataDir, resultDir);
  assert.equal(collected.files.length, 1);
  assert.equal(collected.files[0].truncated, true);
  assert.equal((await readFile(collected.files[0].destination)).length, 1024 * 1024);
  const summaryPath = await writeRunSummary(resultDir, { nativeLogs: collected });
  assert.match(await readFile(summaryPath, "utf8"), /vanehub\.log/);

  const unavailable = await collectUnifiedLogs(path.join(root, "missing"), resultDir);
  assert.equal(unavailable.unavailable.length, 1);
  await rm(root, { recursive: true, force: true });
});
