import { mkdir, mkdtemp, realpath, rm } from "node:fs/promises";
import net from "node:net";
import os from "node:os";
import path from "node:path";
import { randomUUID } from "node:crypto";
import { DesktopVerificationError } from "./verification-error.mjs";

/**
 * Reserves a free loopback port for this run's WebDriver endpoint.
 *
 * The run root, data directory, CLI home and result directory are already per-run; the driver port
 * was the one thing left shared, and a fixed one means two desktop runs on the same machine — a
 * second worktree, or a rerun started before the previous driver had exited — answer on the same
 * address. The loser sees `ECONNRESET` then `ECONNREFUSED` while creating a session, which is
 * reported as whichever spec happened to start at that moment rather than as a collision.
 *
 * Binding to port 0 and releasing it leaves a short window before the driver claims it. That is the
 * standard trade and far narrower than a constant.
 */
export async function reserveLoopbackPort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.on("error", reject);
    server.listen(0, "127.0.0.1", () => {
      const address = server.address();
      server.close(() => {
        if (address && typeof address === "object") resolve(address.port);
        else reject(new DesktopVerificationError("BLOCKED", "No loopback port could be reserved."));
      });
    });
  });
}

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
  const resultDir = path.join(repoRoot, "test-results", "desktop", runId);
  await Promise.all([
    mkdir(dataDir),
    mkdir(fixtureDir),
    mkdir(cliConfigHome),
    mkdir(resultDir, { recursive: true }),
  ]);
  const webdriverPort = options.webdriverPort ?? (await reserveLoopbackPort());
  const canonicalRoot = await realpath(runRoot);
  const canonicalData = await realpath(dataDir);
  validateIsolatedDataPath({
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    normalDataDir: options.normalDataDir ?? defaultApplicationDataDir(),
  });
  return {
    runId,
    runRoot: canonicalRoot,
    dataDir: canonicalData,
    fixtureDir,
    resultDir,
    webdriverPort,
    environment: {
      VANEHUB_APP_DATA_DIR: canonicalData,
      VANEHUB_CLI_CONFIG_HOME: cliConfigHome,
      VANEHUB_TEST_RUN_ID: runId,
      VANEHUB_DESKTOP_RESULT_DIR: resultDir,
      VANEHUB_WEBDRIVER_PORT: String(webdriverPort),
      // The same port under the name the service's direct-eval channel reads. `browser.tauri
      // .execute` does not go through the WebDriver session: it POSTs to `/wdio/eval` on a port it
      // takes from `TAURI_WEBDRIVER_PORT`, falling back to a hard-coded 4445 that has nothing to do
      // with the service's `embeddedPort` option. Moving the driver without moving this leaves
      // sessions creating cleanly while every `execute` fails with a bare `fetch failed`.
      TAURI_WEBDRIVER_PORT: String(webdriverPort),
    },
  };
}
