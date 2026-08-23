import { spawnSync } from "node:child_process";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { loadDesktopMetadata, resolveDesktopArtifact } from "./desktop/artifact.mjs";
import { collectUnifiedLogs, writeRunSummary } from "./desktop/evidence.mjs";
import { detectHost } from "./desktop/platform.mjs";
import { ensureOwnedProcessesStopped, readProcessMarker } from "./desktop/process-ownership.mjs";
import { createLayerResult, verificationExitCode } from "./desktop/result.mjs";
import { createRunContext, disposeRunContext } from "./desktop/run-context.mjs";
import { DesktopVerificationError } from "./desktop/verification-error.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const npmCli = process.env.npm_execpath;
const latestArtifactPath = path.join(repoRoot, "test-results", "desktop", "latest-artifact.json");
// The expanded native layers are valuable release coverage, but they do not yet hold on every
// hosted runner. Keep the PR gate on the cross-platform smoke contract; developers and release
// workflows can opt into the complete suite explicitly.
const runFullSuite = process.env.VANEHUB_DESKTOP_FULL_SUITE === "1" || !process.env.CI;

function run(command, args, options = {}) {
  const result = spawnSync(command, args, { cwd: repoRoot, stdio: "inherit", ...options });
  if (result.error || result.status !== 0) {
    throw new DesktopVerificationError("FAILED", `${command} ${args.join(" ")} failed.`, {
      exitCode: result.status,
      error: result.error?.message,
    });
  }
}

function runNpm(args, options = {}) {
  if (!npmCli) throw new DesktopVerificationError("BLOCKED", "npm_execpath is required to run desktop verification.");
  return run(process.execPath, [npmCli, ...args], options);
}

async function buildDesktop() {
  const host = detectHost();
  const metadata = await loadDesktopMetadata(repoRoot);
  const expectedPath = path.join(metadata.targetDirectory, host.targetTriple, "debug", `${metadata.binaryName}${host.extension}`);
  // Deleting the previous artifact and pinning the build start both bind resolution to this
  // invocation; the timestamp is what fails loudly if a build silently reuses an older binary.
  const buildStartedAt = Date.now();
  await rm(expectedPath, { force: true });
  runNpm(["run", "sidecar:prepare", "--", `--target=${host.targetTriple}`]);
  runNpm([
    "exec", "--", "tauri", "build", "--debug", "--no-bundle", "--ci",
    "--target", host.targetTriple,
    "--features", "desktop-e2e",
    "--config", "src-tauri/tauri.desktop-e2e.conf.json",
  ]);
  const artifact = resolveDesktopArtifact({ metadata, host, profile: "debug", buildStartedAt });
  await mkdir(path.dirname(latestArtifactPath), { recursive: true });
  await writeFile(latestArtifactPath, `${JSON.stringify(artifact, null, 2)}\n`);
  process.stdout.write(`Desktop artifact: ${artifact.executablePath}\n`);
  return artifact;
}

async function loadArtifact() {
  const requested = process.env.VANEHUB_DESKTOP_ARTIFACT;
  if (requested) return { ...JSON.parse(await readFile(latestArtifactPath, "utf8")), executablePath: path.resolve(requested) };
  return JSON.parse(await readFile(latestArtifactPath, "utf8"));
}

/** Pass/fail/skip counts the WDIO run recorded, or null when it never got that far. */
async function readWdioCoverage(resultDir) {
  try {
    const parsed = JSON.parse(await readFile(path.join(resultDir, "wdio-result.json"), "utf8"));
    return typeof parsed.skipped === "number" ? parsed : null;
  } catch {
    return null;
  }
}

/**
 * Runs one wdio-driven desktop layer end to end: isolated run context, artifact launch, owned
 * process cleanup, evidence collection, and a layer result. Every layer gets its own run context
 * and its own wdio configuration, so one layer's environment cannot change what another tests.
 */
async function runDesktopLayer({ layer, config, label, artifact }) {
  artifact ??= await loadArtifact();
  if (!artifact.testBuild || !path.isAbsolute(artifact.executablePath)) {
    throw new DesktopVerificationError("BLOCKED", `${label} requires an absolute test-build artifact path.`);
  }
  const context = await createRunContext(repoRoot);
  const startedAt = new Date().toISOString();
  let status = "FAILED";
  let errorDetails;
  let processCleanup;
  let processState;
  try {
    const env = { ...process.env, ...context.environment, VANEHUB_DESKTOP_ARTIFACT: artifact.executablePath };
    const result = spawnSync(process.execPath, [npmCli, "exec", "--", "wdio", "run", config], {
      cwd: repoRoot,
      env,
      stdio: "inherit",
    });
    if (result.error || result.status !== 0) {
      throw new DesktopVerificationError("FAILED", `${label} failed.`, {
        exitCode: result.status,
        error: result.error?.message,
      });
    }
    processState = await readProcessMarker(context.dataDir);
    processCleanup = await ensureOwnedProcessesStopped({ marker: processState, runId: context.runId });
    processState = await readProcessMarker(context.dataDir);
    if (processState.state !== "exited") {
      throw new DesktopVerificationError("FAILED", "The desktop runtime did not record a clean shutdown.", {
        marker: processState,
      });
    }
    status = "PASSED";
  } catch (error) {
    errorDetails = { message: error.message, status: error.status ?? "FAILED", details: error.details ?? {} };
    try {
      processState = await readProcessMarker(context.dataDir);
      processCleanup = await ensureOwnedProcessesStopped({
        marker: processState,
        runId: context.runId,
        timeoutMs: 2_000,
      });
    } catch (cleanupError) {
      errorDetails.cleanup = cleanupError.message;
    }
  }
  const nativeLogs = await collectUnifiedLogs(context.dataDir, context.resultDir);
  const layerResult = createLayerResult({
    layer,
    status,
    platform: artifact.platform,
    architecture: artifact.architecture,
    artifact: artifact.executablePath,
    runId: context.runId,
    startedAt,
    completedAt: new Date().toISOString(),
    processState,
    processCleanup,
    nativeLogs,
    ...(errorDetails ? { error: errorDetails } : {}),
  });
  const coverage = await readWdioCoverage(context.resultDir);
  const summaryPath = await writeRunSummary(context.resultDir, { layers: [layerResult], coverage });
  await disposeRunContext(context);
  const skipped = coverage?.skipped ? ` (${coverage.skipped} skipped — see BLOCKED above)` : "";
  process.stdout.write(`${label}: ${status}${skipped}\nEvidence: ${context.resultDir}\n`);
  process.exitCode = verificationExitCode(status);
  return { status, summaryPath, resultDir: context.resultDir };
}

function smokeDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-smoke",
    config: "tests/desktop/wdio.conf.mjs",
    label: "Desktop smoke",
    artifact,
  });
}

function cliTerminalDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-cli-terminal",
    config: "tests/desktop/wdio.cli-terminal.conf.mjs",
    label: "Desktop CLI terminal",
    artifact,
  });
}

function sessionWorkspaceDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-session-workspace",
    config: "tests/desktop/wdio.session-workspace.conf.mjs",
    label: "Desktop session workspace",
    artifact,
  });
}

function dialogsDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-dialogs",
    config: "tests/desktop/wdio.dialogs.conf.mjs",
    label: "Desktop dialogs",
    artifact,
  });
}

function cliManagementDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-cli-management",
    config: "tests/desktop/wdio.cli-management.conf.mjs",
    label: "Desktop CLI management",
    artifact,
  });
}

function settingsPersistenceDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-settings-persistence",
    config: "tests/desktop/wdio.settings-persistence.conf.mjs",
    label: "Desktop settings persistence",
    artifact,
  });
}

async function main() {
  const mode = process.argv[2] ?? "all";
  if (mode === "build") await buildDesktop();
  else if (mode === "smoke") await smokeDesktop();
  else if (mode === "cli-terminal") await cliTerminalDesktop();
  else if (mode === "session-workspace") await sessionWorkspaceDesktop();
  else if (mode === "dialogs") await dialogsDesktop();
  else if (mode === "settings-persistence") await settingsPersistenceDesktop();
  else if (mode === "cli-management") await cliManagementDesktop();
  else if (mode === "all") {
    const artifact = await buildDesktop();
    const fullSuiteLayers = [smokeDesktop, cliTerminalDesktop, cliManagementDesktop, sessionWorkspaceDesktop, dialogsDesktop, settingsPersistenceDesktop];
    const layers = runFullSuite ? fullSuiteLayers : [smokeDesktop];
    if (!runFullSuite) {
      process.stdout.write("Desktop verification: CI gate runs smoke only; set VANEHUB_DESKTOP_FULL_SUITE=1 for all layers.\n");
    }
    const results = [];
    // Sequential rather than concurrent: the layers share one webdriver port and one desktop
    // artifact, and a layer's evidence is only attributable if it owned the machine while it ran.
    for (const layer of layers) {
      results.push(await layer(artifact));
    }
    // Each layer sets its own exit code as it finishes; the run as a whole is only green when
    // every layer is, so the worst result has to win rather than the last one.
    process.exitCode = Math.max(...results.map((result) => verificationExitCode(result.status)));
  } else throw new DesktopVerificationError("BLOCKED", `Unknown desktop test mode: ${mode}`);
}

main().catch((error) => {
  const status = error.status ?? "FAILED";
  process.stderr.write(`${status}: ${error.message}\n`);
  if (error.details) process.stderr.write(`${JSON.stringify(error.details, null, 2)}\n`);
  process.exitCode = verificationExitCode(status);
});
