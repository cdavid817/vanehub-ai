import { mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { DesktopVerificationError } from "./verification-error.mjs";

const normalize = (value) => process.platform === "win32" ? value.toLowerCase() : value;
const isWithin = (parent, child) => {
  const relative = path.relative(parent, child);
  return relative !== "" && !relative.startsWith(`..${path.sep}`) && relative !== ".." && !path.isAbsolute(relative);
};

export function defaultApplicationDataDir(identifier = "ai.vanehub.app", env = process.env, platform = process.platform) {
  if (platform === "win32") return env.APPDATA ? path.join(env.APPDATA, identifier) : null;
  if (platform === "darwin") return env.HOME ? path.join(env.HOME, "Library", "Application Support", identifier) : null;
  const root = env.XDG_DATA_HOME ?? (env.HOME ? path.join(env.HOME, ".local", "share") : null);
  return root ? path.join(root, identifier) : null;
}

/**
 * A port the OS has just confirmed is free, for this run's embedded WebDriver.
 *
 * The default used to be a hard-coded 4445. That port is not owned by anything: the layers run
 * one after another in a single job and the previous layer's driver can still be releasing it,
 * and on a developer machine a second checkout running its own suite competes for the same
 * number. Both show up as `Embedded WebDriver server did not become ready on port 4445 within
 * 120000ms` -- a whole layer lost to an address, with nothing wrong with the code under test.
 *
 * Binding to port 0 and reading back the assignment leaves a small window before the driver
 * claims it, which is the standard trade for not having to guess. It is a far smaller window than
 * sharing one number across every run on the machine.
 */
async function reserveWebdriverPort() {
  return await new Promise((resolve, reject) => {
    const probe = net.createServer();
    probe.unref();
    probe.on("error", reject);
    probe.listen(0, "127.0.0.1", () => {
      const { port } = probe.address();
      probe.close((error) => (error ? reject(error) : resolve(port)));
    });
  });
}

export function validateIsolatedDataPath({ runRoot, dataDir, normalDataDir }) {
  if (!path.isAbsolute(runRoot) || !path.isAbsolute(dataDir)) {
    throw new DesktopVerificationError("BLOCKED", "Desktop test paths must be absolute.");
  }
  const resolvedRoot = path.resolve(runRoot);
  const resolvedData = path.resolve(dataDir);
  if (!isWithin(resolvedRoot, resolvedData)) {
    throw new DesktopVerificationError("BLOCKED", "The desktop test data directory escapes its run root.");
  }
  if (normalDataDir && normalize(resolvedData) === normalize(path.resolve(normalDataDir))) {
    throw new DesktopVerificationError("BLOCKED", "The desktop test data directory aliases normal application data.");
  }
}

// Cleanup lives beside the code that created the run root so a passing run cannot leave the
// isolated SQLite database, configuration, or fixtures behind — CI would otherwise upload them
// as evidence from a job that had nothing to diagnose.
//
// The retries are for Windows: a spec that opened a terminal or shell tab rooted a PTY child in a
// fixture directory, and the OS releases that directory handle a moment after the child dies, not
// synchronously with it. Without them the removal loses the race and reports `EBUSY`, which turns
// a run whose every test passed into `FAILED` on a cleanup detail.
export async function disposeRunContext(context) {
  await rm(context.runRoot, { recursive: true, force: true, maxRetries: 20, retryDelay: 100 });
  return { removed: context.runRoot, retainedEvidence: context.resultDir };
}

export async function createRunContext(repoRoot, options = {}) {
  const runId = options.runId ?? `${new Date().toISOString().replaceAll(/[:.]/g, "-")}-${randomUUID().slice(0, 8)}`;
  const runRoot = await mkdtemp(path.join(options.tempRoot ?? os.tmpdir(), "vanehub-desktop-e2e-"));
  const dataDir = path.join(runRoot, "data");
  const fixtureDir = path.join(runRoot, "fixtures");
  // CLI global configuration (`~/.claude/settings.json` and friends) is normal application state
  // too: without its own isolated home, a spec that assigns claude-code a policy template writes
  // the permission hook into the user's real settings, where it outlives the test app and blocks
  // every later tool call against a dead approval server.
  const cliConfigHome = path.join(runRoot, "cli-home");
  // The OS home, distinct from the two above. `VANEHUB_APP_DATA_DIR` covers what the application
  // stores and `VANEHUB_CLI_CONFIG_HOME` covers what the CLI agents store, but anything either one
  // resolves through the *platform's* home -- a cache, a keyring path, a tool's dotfile -- still
  // landed in the developer's real profile and outlived the run.
  const resultScope = options.resultScope ?? "desktop";
  const resultDir = path.join(repoRoot, "test-results", resultScope, runId);
  await Promise.all([
    mkdir(dataDir),
    mkdir(fixtureDir),
    mkdir(cliConfigHome),
    mkdir(resultDir, { recursive: true }),
  ]);
  const canonicalRoot = await realpath(runRoot);
  const canonicalData = await realpath(dataDir);
  // Derived from the canonical root rather than the one `mkdtemp` returned. On macOS those differ
  // -- `/var/folders/...` against `/private/var/folders/...` -- and a home that is inside the run
  // root by one spelling and outside it by the other cannot be checked against anything.
  const home = path.join(canonicalRoot, "home");
  const localAppData = path.join(home, "AppData", "Local");
  const roamingAppData = path.join(home, "AppData", "Roaming");
  await Promise.all([
    mkdir(localAppData, { recursive: true }),
    mkdir(roamingAppData, { recursive: true }),
  ]);
  validateIsolatedDataPath({
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    normalDataDir: options.normalDataDir ?? defaultApplicationDataDir(),
  });
  // An explicit port stays honoured so a constrained environment can pin one it has opened.
  const pinnedPort = Number(process.env.VANEHUB_WEBDRIVER_PORT);
  const requestedPort = options.webdriverPort ?? (Number.isInteger(pinnedPort) && pinnedPort > 0 ? pinnedPort : null);
  const webdriverPort = requestedPort ?? (await reserveWebdriverPort());
  return {
    runId,
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    fixtureDir,
    home,
    webdriverPort,
    resultDir,
    environment: {
      VANEHUB_APP_DATA_DIR: canonicalData,
      VANEHUB_CLI_CONFIG_HOME: cliConfigHome,
      VANEHUB_TEST_RUN_ID: runId,
      VANEHUB_DESKTOP_RESULT_DIR: resultDir,
      VANEHUB_WEBDRIVER_PORT: String(webdriverPort),
      // The same port, under the name the wdio worker itself reads.
      //
      // `browser.tauri.execute` does not go through the WebDriver session -- it opens its own
      // connection, and it resolves that port from `TAURI_WEBDRIVER_PORT` in the worker process,
      // falling back to 4445. The service only sets that variable for the application it spawns,
      // so on any port but 4445 the session would connect correctly while every `tauri.execute`
      // -- which is every `core.invoke` a spec makes -- talked to an address nothing was serving
      // and failed as `TypeError: fetch failed`.
      TAURI_WEBDRIVER_PORT: String(webdriverPort),
      // Carried under their own names, not as `HOME`/`APPDATA` directly. These have to reach the
      // application under test, and only it: repointing them for this process would also repoint
      // the npm and node that are running the harness, which read `APPDATA` for their own config.
      // `wdio-shared.mjs` maps them onto the real variable names for the child it launches.
      VANEHUB_DESKTOP_HOME: home,
      VANEHUB_DESKTOP_APPDATA: roamingAppData,
      VANEHUB_DESKTOP_LOCALAPPDATA: localAppData,
    },
  };
}
