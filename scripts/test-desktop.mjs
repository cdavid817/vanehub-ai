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

/** Pass/fail/skip counts the WDIO run recorded, or null when it never got that far. */
async function readWdioCoverage(resultDir) {
  try {
    const parsed = JSON.parse(await readFile(path.join(resultDir, "wdio-result.json"), "utf8"));
    return typeof parsed.skipped === "number" ? parsed : null;
  } catch {
    return null;
  }
}

async function loadArtifact() {
  const requested = process.env.VANEHUB_DESKTOP_ARTIFACT;
  if (requested) return { ...JSON.parse(await readFile(latestArtifactPath, "utf8")), executablePath: path.resolve(requested) };
  return JSON.parse(await readFile(latestArtifactPath, "utf8"));
}

async function smokeDesktop(artifact) {
  artifact ??= await loadArtifact();
  if (!artifact.testBuild || !path.isAbsolute(artifact.executablePath)) {
    throw new DesktopVerificationError("BLOCKED", "Desktop smoke requires an absolute test-build artifact path.");
  }
  const context = await createRunContext(repoRoot);
  const startedAt = new Date().toISOString();
  let status = "FAILED";
  let errorDetails;
  let processCleanup;
  let processState;
  try {
    const env = { ...process.env, ...context.environment, VANEHUB_DESKTOP_ARTIFACT: artifact.executablePath };
    const result = spawnSync(process.execPath, [npmCli, "exec", "--", "wdio", "run", "tests/desktop/wdio.conf.mjs"], {
      cwd: repoRoot,
      env,
      stdio: "inherit",
    });
    if (result.error || result.status !== 0) {
      throw new DesktopVerificationError("FAILED", "Native desktop smoke failed.", {
        exitCode: result.status,
        error: result.error?.message,
      });
    }
    processState = await readProcessMarker(context.dataDir);
    processCleanup = await ensureOwnedProcessesStopped({ marker: processState, runId: context.runId });
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
  const layer = createLayerResult({
    layer: "desktop-smoke",
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
  const summaryPath = await writeRunSummary(context.resultDir, { layers: [layer], coverage });
  await disposeRunContext(context);
  // Skips are named in the verdict rather than buried in the reporter: a host without a proxy,
  // provider key, SSH target or installed CLI skips whole capabilities and still exits 0, and
  // "PASSED" alone presents that reduced coverage as a clean bill of health.
  const skipped = coverage?.skipped ? ` (${coverage.skipped} skipped — see BLOCKED above)` : "";
  process.stdout.write(`Desktop smoke: ${status}${skipped}\nEvidence: ${context.resultDir}\n`);
  process.exitCode = verificationExitCode(status);
  return { status, summaryPath, resultDir: context.resultDir };
}

async function main() {
  const mode = process.argv[2] ?? "all";
  if (mode === "build") await buildDesktop();
  else if (mode === "smoke") await smokeDesktop();
  else if (mode === "all") await smokeDesktop(await buildDesktop());
  else throw new DesktopVerificationError("BLOCKED", `Unknown desktop test mode: ${mode}`);
}

main().catch((error) => {
  const status = error.status ?? "FAILED";
  process.stderr.write(`${status}: ${error.message}\n`);
  if (error.details) process.stderr.write(`${JSON.stringify(error.details, null, 2)}\n`);
  process.exitCode = verificationExitCode(status);
});
