import assert from "node:assert/strict";
import { access, mkdir, mkdtemp, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import process from "node:process";
import { clearTimeout, setTimeout } from "node:timers";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import { resolveDesktopArtifact } from "./desktop/artifact.mjs";
import { collectUnifiedLogs, writeRunSummary } from "./desktop/evidence.mjs";
import { detectHost } from "./desktop/platform.mjs";
import { ensureOwnedProcessesStopped, ownedProcessIds } from "./desktop/process-ownership.mjs";
import { createLayerResult, verificationExitCode } from "./desktop/result.mjs";
import { createRunContext, disposeRunContext, validateIsolatedDataPath } from "./desktop/run-context.mjs";

const exists = async (target) => access(target).then(() => true, () => false);

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

test("removes temporary application data after a successful run and keeps only evidence", async () => {
  const tempRoot = await mkdtemp(path.join(os.tmpdir(), "vanehub-cleanup-test-"));
  const repoRoot = path.join(tempRoot, "repo");
  const context = await createRunContext(repoRoot, { tempRoot, runId: "run-clean" });
  await mkdir(path.join(context.dataDir, "logs"), { recursive: true });
  await writeFile(path.join(context.dataDir, "logs", "vanehub.log"), "redacted=true\n");
  await writeFile(path.join(context.dataDir, "vanehub.sqlite"), "isolated-test-database");
  await writeFile(path.join(context.fixtureDir, "workspace.json"), "{}");

  const nativeLogs = await collectUnifiedLogs(context.dataDir, context.resultDir);
  await writeRunSummary(context.resultDir, { layers: [createLayerResult({ layer: "desktop-smoke", status: "PASSED" })], nativeLogs });
  const disposal = await disposeRunContext(context);

  assert.equal(disposal.removed, context.runRoot);
  assert.equal(await exists(context.runRoot), false);
  assert.equal(await exists(context.dataDir), false);
  assert.equal(await exists(context.fixtureDir), false);
  assert.equal(await exists(path.join(context.resultDir, "summary.json")), true);
  assert.deepEqual(await readdir(path.join(context.resultDir, "logs", "native")), ["vanehub.log"]);
  assert.equal((await collectEvidenceNames(context.resultDir)).some((name) => name.endsWith(".sqlite")), false);
  await rm(tempRoot, { recursive: true, force: true });
});

// A spec that opened a terminal or shell tab leaves a PTY child whose working directory is a
// fixture directory, and Windows keeps that directory undeletable until the process is gone --
// and for a short while after. A live child reproduces that exactly; an open file handle does
// not, because Node opens files with FILE_SHARE_DELETE. Without retries the removal loses the
// race and reports EBUSY, turning a run whose every test passed into FAILED.
test("disposal outlasts a child process still rooted in a fixture directory", async () => {
  const tempRoot = await mkdtemp(path.join(os.tmpdir(), "vanehub-cleanup-busy-"));
  const context = await createRunContext(path.join(tempRoot, "repo"), { tempRoot, runId: "run-busy" });
  const rooted = path.join(context.fixtureDir, "rooted");
  await mkdir(rooted, { recursive: true });
  const child = process.platform === "win32"
    ? spawn("cmd.exe", ["/c", "pause"], { cwd: rooted, stdio: "ignore", windowsHide: true })
    : spawn("sh", ["-c", "read line"], { cwd: rooted, stdio: "ignore" });
  const release = setTimeout(() => child.kill(), 500);

  const disposal = await disposeRunContext(context);

  clearTimeout(release);
  child.kill();
  assert.equal(disposal.removed, context.runRoot);
  assert.equal(await exists(context.runRoot), false);
  await rm(tempRoot, { recursive: true, force: true });
});

async function collectEvidenceNames(directory) {
  const entries = await readdir(directory, { withFileTypes: true, recursive: true });
  return entries.map((entry) => entry.name);
}

test("each desktop layer owns a disjoint spec directory and its own wdio configuration", async () => {
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  const smoke = await readFile("tests/desktop/wdio.conf.mjs", "utf8");
  const cliTerminal = await readFile("tests/desktop/wdio.cli-terminal.conf.mjs", "utf8");

  // `all` must run both layers, and a layer that reused another's spec directory would silently
  // run the same specs twice under two different environments.
  assert.match(orchestrator, /mode === "cli-terminal"/);
  assert.match(orchestrator, /cliTerminalDesktop\(artifact\)/);
  assert.match(smoke, /specDirectory: "specs"/);
  assert.match(cliTerminal, /specDirectory: "specs-cli-terminal"/);
});

test("the CLI terminal layer resolves its fixture Agent instead of an installed one", async () => {
  const config = await readFile("tests/desktop/wdio.cli-terminal.conf.mjs", "utf8");
  const fixture = "tests/desktop/fixtures/cli/opencode";
  const windowsLauncher = `${fixture}.cmd`;

  // Ahead of the inherited PATH, or a host with the real Agent installed would test that Agent.
  assert.match(config, /PATH: `\$\{fixtureDir\}\$\{path\.delimiter\}/);
  assert.ok(await exists(fixture), "the fixture CLI executable is missing");
  assert.ok(await exists(windowsLauncher), "the Windows fixture CLI launcher is missing");
  const source = await readFile(fixture, "utf8");
  const windowsSource = await readFile(windowsLauncher, "utf8");
  assert.match(source, /VANEHUB-FIXTURE-CLI READY/);
  assert.match(windowsSource, /VANEHUB-FIXTURE-CLI READY/);
  assert.match(windowsSource, /VANEHUB-FIXTURE-ECHO/);
  // The fixture is what makes this layer runnable without credentials on any CI host.
  assert.doesNotMatch(source, /require\(|fetch\(|https?:\/\//);
});

test("the fixture CLI stays executable through a fresh clone", async () => {
  const { execFile } = await import("node:child_process");
  const { promisify } = await import("node:util");
  const { stdout } = await promisify(execFile)("git", ["ls-files", "--stage", "tests/desktop/fixtures/cli/opencode"]);

  assert.match(stdout, /^100755 /, "the fixture must be committed with its executable bit");
});

test("every native UI layer is wired, disjoint, and reachable on its own", async () => {
  const orchestrator = await readFile("scripts/test-desktop.mjs", "utf8");
  const { scripts } = JSON.parse(await readFile("package.json", "utf8"));
  const layers = [
    ["session-workspace", "sessionWorkspaceDesktop"],
    ["dialogs", "dialogsDesktop"],
    ["settings-persistence", "settingsPersistenceDesktop"],
  ];

  for (const [mode, factory] of layers) {
    assert.match(orchestrator, new RegExp(`mode === "${mode}"`), `${mode} has no mode`);
    assert.match(orchestrator, new RegExp(`${factory}[,\\]]`), `${mode} is missing from the all-layer run`);
    assert.ok(`test:desktop:${mode}` in scripts, `${mode} has no npm script`);
    const config = await readFile(`tests/desktop/wdio.${mode}.conf.mjs`, "utf8");
    assert.match(config, new RegExp(`specDirectory: "specs-${mode}"`), `${mode} does not own its spec directory`);
  }
});

test("the settings persistence layer orders its specs so the relaunch is real", async () => {
  const config = await readFile("tests/desktop/wdio.settings-persistence.conf.mjs", "utf8");
  const relaunch = await readFile("tests/desktop/specs-settings-persistence/verify-after-relaunch.e2e.mjs", "utf8");

  // A glob would not promise that the change runs before the check, and the check is only
  // meaningful against an application that started after the change was written.
  assert.match(config, /specFiles: \["change-setting\.e2e\.mjs", "verify-after-relaunch\.e2e\.mjs"\]/);
  assert.match(config, /specFileRetries: 2/);
  assert.match(config, /specFileRetriesDelay: 5/);
  // Importing the sibling spec would register its describe block twice in the same worker.
  assert.doesNotMatch(relaunch, /from "\.\/change-setting\.e2e\.mjs"/);
  // Browser storage is the Web adapter's persistence, not the desktop client's.
  assert.doesNotMatch(relaunch, /localStorage/);
  assert.match(relaunch, /core\.invoke\("get_settings"\)/);
});
