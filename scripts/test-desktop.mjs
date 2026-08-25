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
import { withDirectoryRestored } from "./desktop/schema-snapshot.mjs";
import { DesktopVerificationError } from "./desktop/verification-error.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const npmCli = process.env.npm_execpath;
const latestArtifactPath = path.join(repoRoot, "test-results", "desktop", "latest-artifact.json");
// The `desktop-e2e` feature pulls in the WDIO plugin, whose ACL entries Tauri regenerates into
// these tracked files. A normal build does not produce them, so leaving them behind breaks the
// documentation job's read-only check for whoever commits next.
const generatedSchemas = path.join(repoRoot, "src-tauri", "gen", "schemas");
// The expanded native layers are valuable release coverage, but they do not yet hold on every
// hosted runner. Keep the PR gate on the cross-platform smoke contract; developers and release
// workflows can opt into the complete suite explicitly.
const runFullSuite = process.env.VANEHUB_DESKTOP_FULL_SUITE === "1" || !process.env.CI;

/** Without all of these there is no real integration for the external suite to verify. */
const EXTERNAL_PREREQUISITES = ["VANEHUB_DESKTOP_MUTATE_HOST", "VANEHUB_SSH_HOST", "VANEHUB_SSH_USER", "VANEHUB_SSH_PASSWORD"];

/**
 * Records a BLOCKED external run as evidence in its own right.
 *
 * A job that uploads nothing looks the same as a job that never ran, and "never ran" is the reading
 * that quietly becomes "passed" when someone summarises a matrix later.
 */
async function writeExternalBlockedEvidence(reason, missing) {
  const resultDir = path.join(repoRoot, "test-results", "desktop", "external-provider-blocked");
  await mkdir(resultDir, { recursive: true });
  await writeFile(path.join(resultDir, "summary.json"), `${JSON.stringify({
    layers: [{ layer: "desktop-external-provider", status: "BLOCKED", reason, missingPrerequisites: missing }],
    coverage: null,
  }, null, 2)}\n`);
}

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
  // Snapshot by bytes rather than by Git: the directory may already carry edits the developer made
  // before running this, and a `git restore` would delete work nobody asked to discard.
  const artifact = await withDirectoryRestored(generatedSchemas, async () => {
    runNpm(["run", "sidecar:prepare", "--", `--target=${host.targetTriple}`]);
    runNpm([
      "exec", "--", "tauri", "build", "--debug", "--no-bundle", "--ci",
      "--target", host.targetTriple,
      "--features", "desktop-e2e",
      "--config", "src-tauri/tauri.desktop-e2e.conf.json",
    ]);
    return resolveDesktopArtifact({ metadata, host, profile: "debug", buildStartedAt });
  });
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
async function runDesktopLayer({ layer, config, label, artifact, environment = {} }) {
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
    const env = { ...process.env, ...context.environment, ...environment, VANEHUB_DESKTOP_ARTIFACT: artifact.executablePath };
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

function coreSmokeDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-core-smoke",
    config: "tests/desktop/wdio.conf.mjs",
    label: "Desktop core smoke",
    artifact,
    environment: { VANEHUB_DESKTOP_CORE_SMOKE: "1" },
  });
}

/**
 * The suite that needs a real package manager, host environment, or SSH server.
 *
 * Reports `BLOCKED` rather than running when its prerequisites are absent. A suite that verifies
 * real integrations has nothing to say on a runner that has none, and saying `PASSED` there would
 * be the most misleading result available.
 */
function missingExternalPrerequisites() {
  return EXTERNAL_PREREQUISITES.filter((variable) => !process.env[variable]);
}

async function externalProviderDesktop(artifact) {
  const missing = missingExternalPrerequisites();
  if (missing.length > 0) {
    const reason = `External provider suite: no real prerequisites on this host (${missing.join(", ")}).`;
    process.stdout.write(`Desktop external provider: BLOCKED\n${reason}\n`);
    await writeExternalBlockedEvidence(reason, missing);
    // Deliberately not an error exit. This suite never gates, so an unconfigured runner reports
    // what it is rather than failing a pipeline that was never asking it for a verdict.
    return { status: "BLOCKED", resultDir: null };
  }
  return runDesktopLayer({
    layer: "desktop-external-provider",
    config: "tests/desktop/wdio.external-provider.conf.mjs",
    label: "Desktop external provider",
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

// Opt-in only, never part of `all`: it drives the REAL codex-cli and claude-code against a real
// requirement, so it spends model tokens and needs both CLIs authenticated on the host.
//
// External-provider layers in the sense the desktop spec now defines: real Agent, real login, real
// model output. They stay out of the required gate for the same reason `native-flows` does.
function multiAgentRequirementDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-multi-agent-requirement",
    config: "tests/desktop/wdio.multi-agent-requirement.conf.mjs",
    label: "Desktop multi-agent requirement",
    artifact,
  });
}

// Opt-in only: a long-running three-seat feature build with real model calls.
function multiAgentLongrunDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-multi-agent-longrun",
    config: "tests/desktop/wdio.multi-agent-longrun.conf.mjs",
    label: "Desktop multi-agent longrun",
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

function agentMcpDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-agent-mcp",
    config: "tests/desktop/wdio.agent-mcp.conf.mjs",
    label: "Desktop Agent MCP",
    artifact,
  });
}

/**
 * Deterministic local-media coverage, kept out of `all` on purpose.
 *
 * It needs a Python interpreter and a prepared fixture tree that none of the other layers want, and
 * it runs the same artifact under a different assembly. Folding it into the default suite would
 * make every other layer's result depend on whether this machine has Python.
 */
function localMediaDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-local-media-fixture",
    config: "tests/desktop/wdio.local-media.conf.mjs",
    label: "Desktop local media fixture",
    artifact,
  });
}

function skillsDesktop(artifact) {
  return runDesktopLayer({
    layer: "desktop-skills",
    config: "tests/desktop/wdio.skills.conf.mjs",
    label: "Desktop Skills effectiveness",
    artifact,
  });
}

/** The layers the required hermetic gate runs. Every one of them must pass. */
const fullSuiteLayers = [
  smokeDesktop,
  cliTerminalDesktop,
  cliManagementDesktop,
  sessionWorkspaceDesktop,
  dialogsDesktop,
  settingsPersistenceDesktop,
  agentMcpDesktop,
];

async function runLayers(layers, artifact) {
  const results = [];
  // Sequential rather than concurrent: the layers share one desktop artifact, and a layer's
  // evidence is only attributable if it owned the machine while it ran.
  for (const layer of layers) {
    results.push(await layer(artifact));
  }
  return results;
}

async function main() {
  const mode = process.argv[2] ?? "all";
  if (mode === "build") await buildDesktop();
  else if (mode === "smoke") await smokeDesktop();
  else if (mode === "core-smoke") await coreSmokeDesktop();
  else if (mode === "cli-terminal") await cliTerminalDesktop();
  else if (mode === "session-workspace") await sessionWorkspaceDesktop();
  else if (mode === "dialogs") await dialogsDesktop();
  else if (mode === "settings-persistence") await settingsPersistenceDesktop();
  else if (mode === "cli-management") await cliManagementDesktop();
  else if (mode === "agent-mcp") await agentMcpDesktop();
  else if (mode === "local-media") await localMediaDesktop();
  else if (mode === "skills") await skillsDesktop();
  else if (mode === "multi-agent-requirement") await multiAgentRequirementDesktop();
  else if (mode === "multi-agent-longrun") await multiAgentLongrunDesktop();
  else if (mode === "external-provider") {
    // Prerequisites before the build. A runner with no real Agent has nothing for this suite to
    // verify, and spending ten minutes compiling the application to say so buys nothing.
    const blocked = missingExternalPrerequisites().length > 0;
    const result = await externalProviderDesktop(blocked ? null : await buildDesktop());
    // BLOCKED is a reportable outcome here, not a failure: this suite never gates.
    process.exitCode = result.status === "FAILED" ? 1 : 0;
  } else if (mode === "all" || mode === "everything") {
    const artifact = await buildDesktop();
    const layers = runFullSuite ? fullSuiteLayers : [coreSmokeDesktop];
    if (!runFullSuite) {
      process.stdout.write("Desktop verification: CI gate runs the core smoke contract; set VANEHUB_DESKTOP_FULL_SUITE=1 for every required layer.\n");
    }
    const results = await runLayers(layers, artifact);
    // Each layer sets its own exit code as it finishes; the run as a whole is only green when
    // every layer is, so the worst result has to win rather than the last one.
    process.exitCode = Math.max(...results.map((result) => verificationExitCode(result.status)));
    if (mode === "everything") {
      // Appended, never folded in: a BLOCKED external result must not turn the required verdict
      // red, and a passing external result must not make a failed required layer look green.
      const external = await externalProviderDesktop(artifact);
      if (external.status === "FAILED") process.exitCode = 1;
    }
  } else throw new DesktopVerificationError("BLOCKED", `Unknown desktop test mode: ${mode}`);
}

main().catch((error) => {
  const status = error.status ?? "FAILED";
  process.stderr.write(`${status}: ${error.message}\n`);
  if (error.details) process.stderr.write(`${JSON.stringify(error.details, null, 2)}\n`);
  process.exitCode = verificationExitCode(status);
});
