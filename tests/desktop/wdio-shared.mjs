import { mkdir, writeFile } from "node:fs/promises";
import { createConnection } from "node:net";
import path from "node:path";
import process from "node:process";
import { setTimeout as delay } from "node:timers/promises";
import { fileURLToPath } from "node:url";

const configDir = path.dirname(fileURLToPath(import.meta.url));
const EMBEDDED_DRIVER_SHUTDOWN_POLL_MS = 100;
const EMBEDDED_DRIVER_SHUTDOWN_TIMEOUT_MS = 10_000;
const EMBEDDED_DRIVER_PROCESS_REAP_MS = 2_000;

function isTcpPortOpen(port) {
  return new Promise((resolve) => {
    const socket = createConnection({ host: "127.0.0.1", port });
    let settled = false;
    const finish = (open) => {
      if (settled) return;
      settled = true;
      socket.destroy();
      resolve(open);
    };
    socket.setTimeout(500, () => finish(false));
    socket.once("connect", () => finish(true));
    socket.once("error", () => finish(false));
  });
}

function createEmbeddedDriverShutdownWaiter(port) {
  let hasStartedWorker = false;
  return async () => {
    if (!hasStartedWorker) {
      hasStartedWorker = true;
      return;
    }
    const deadline = Date.now() + EMBEDDED_DRIVER_SHUTDOWN_TIMEOUT_MS;
    while (Date.now() < deadline && await isTcpPortOpen(port)) {
      await delay(EMBEDDED_DRIVER_SHUTDOWN_POLL_MS);
    }
    // The native port closes before Tauri has fully reaped the old application process. Starting
    // its replacement immediately can hit the single-instance process while it is shutting down.
    await delay(EMBEDDED_DRIVER_PROCESS_REAP_MS);
  };
}

function isFailedTest(result) {
  return !result.passed && result.skipped !== true;
}

/**
 * The isolated OS home the orchestrator created for this run, mapped onto the real variable names
 * for the application under test.
 *
 * `run-context.mjs` passes these under `VANEHUB_DESKTOP_*` precisely so they do not apply to this
 * process; it owns the run root and has already validated that it does not alias real application
 * data. Absent when wdio was invoked directly rather than through `test-desktop.mjs`, which keeps
 * a bare `wdio run` working against the developer's own profile.
 */
function homeEnvironment() {
  const home = process.env.VANEHUB_DESKTOP_HOME;
  if (!home) return {};
  return {
    // Both names for one directory: `HOME` is what POSIX APIs read and `USERPROFILE` is what
    // Windows APIs read, so setting both keeps one environment shape across the three runners.
    HOME: home,
    USERPROFILE: home,
    APPDATA: process.env.VANEHUB_DESKTOP_APPDATA,
    LOCALAPPDATA: process.env.VANEHUB_DESKTOP_LOCALAPPDATA,
  };
}

function proxyEnvironment() {
  const proxy = process.env.HTTPS_PROXY ?? process.env.https_proxy;
  if (!proxy?.startsWith("http")) return {};
  const bypass = process.env.NO_PROXY ?? process.env.no_proxy;
  return {
    HTTPS_PROXY: proxy,
    HTTP_PROXY: process.env.HTTP_PROXY ?? process.env.http_proxy ?? proxy,
    NO_PROXY: bypass ? `127.0.0.1,localhost,${bypass}` : "127.0.0.1,localhost",
  };
}

/**
 * Shared wdio configuration for every desktop verification layer.
 *
 * Each layer owns a disjoint spec directory and its own result directory, so one layer's
 * environment — notably the CLI-terminal layer's fixture `PATH` — can never leak into another
 * and silently change what that layer tests.
 */
export async function createDesktopConfig({ specDirectory, specFiles, environment = {} }) {
  const artifactPath = process.env.VANEHUB_DESKTOP_ARTIFACT;
  const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
  if (!artifactPath || !resultDir) throw new Error("Desktop artifact and result directory are required.");

  const logDir = path.join(resultDir, "logs");
  const embeddedDriverPort = Number(process.env.VANEHUB_WEBDRIVER_PORT ?? 4445);
  const waitForEmbeddedDriverShutdown = createEmbeddedDriverShutdownWaiter(embeddedDriverPort);
  await mkdir(logDir, { recursive: true });

  return {
    runner: "local",
    // An explicit list, when given, fixes the order. A layer that verifies persistence across a
    // relaunch depends on one spec running after another, which a glob does not promise.
    specs: specFiles
      ? specFiles.map((file) => path.join(configDir, specDirectory, file))
      : [path.join(configDir, specDirectory, "**", "*.e2e.mjs")],
    maxInstances: 1,
    services: [["tauri", {
      appBinaryPath: artifactPath,
      driverProvider: "embedded",
      embeddedPort: embeddedDriverPort,
      startTimeout: 120_000,
      statusPollTimeout: 5_000,
      commandTimeout: 30_000,
      captureBackendLogs: true,
      captureFrontendLogs: true,
      backendLogLevel: "debug",
      frontendLogLevel: "debug",
      logDir,
      env: {
        VANEHUB_APP_DATA_DIR: process.env.VANEHUB_APP_DATA_DIR,
        VANEHUB_CLI_CONFIG_HOME: process.env.VANEHUB_CLI_CONFIG_HOME,
        VANEHUB_TEST_RUN_ID: process.env.VANEHUB_TEST_RUN_ID,
        VANEHUB_DESKTOP_RESULT_DIR: resultDir,
        // The run context's isolated OS home, so anything the application resolves through the
        // platform home lands inside the run root instead of the developer's profile. A layer that
        // owns a fixture home -- CLI management -- overrides these below.
        ...homeEnvironment(),
        ...proxyEnvironment(),
        ...environment,
      },
    }]],
    capabilities: [{
      browserName: "tauri",
      "tauri:options": { application: artifactPath },
    }],
    logLevel: "info",
    outputDir: logDir,
    bail: 0,
    waitforTimeout: 20_000,
    connectionRetryTimeout: 120_000,
    connectionRetryCount: 1,
    framework: "mocha",
    reporters: ["spec"],
    mochaOpts: { ui: "bdd", timeout: 300_000 },
    // WDIO runs this launcher hook before the Tauri service hook. Wait until the prior worker's
    // clean app exit has really closed the port, so the service observes the stopped driver and
    // restarts it before creating the next session.
    onWorkerStart: waitForEmbeddedDriverShutdown,
    afterTest: async (test, _context, result) => {
      if (isFailedTest(result)) {
        const slug = `${test.parent ?? "spec"}-${test.title ?? "test"}`
          .replaceAll(/[^\p{L}\p{N}]+/gu, "-")
          .replaceAll(/^-|-$/g, "")
          .slice(0, 120);
        try {
          await globalThis.browser.saveScreenshot(path.join(resultDir, "screenshots", `${slug}.png`));
        } catch (error) {
          await writeFile(
            path.join(resultDir, "screenshots", `${slug}-unavailable.txt`),
            `Failure screenshot unavailable: ${error instanceof Error ? error.message : String(error)}\n`,
          );
        }
      }
    },
    onPrepare: async () => {
      await mkdir(path.join(resultDir, "screenshots"), { recursive: true });
    },
    after: async () => {
      try {
        await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
      } catch {
        // A layer may already have exited explicitly; the process marker remains authoritative.
      }
    },
    onComplete: async (exitCode, _config, _capabilities, results) => {
      await writeFile(path.join(resultDir, "wdio-result.json"), `${JSON.stringify({
        exitCode,
        passed: results?.passed ?? null,
        failed: results?.failed ?? null,
        skipped: results?.skipped ?? null,
      }, null, 2)}\n`);
    },
  };
}
