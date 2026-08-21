import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const artifactPath = process.env.VANEHUB_DESKTOP_ARTIFACT;
const resultDir = process.env.VANEHUB_DESKTOP_RESULT_DIR;
if (!artifactPath || !resultDir) throw new Error("Desktop artifact and result directory are required.");

const configDir = path.dirname(fileURLToPath(import.meta.url));
const logDir = path.join(resultDir, "logs");
await mkdir(logDir, { recursive: true });

function proxyEnvironment() {
  const proxy = process.env.HTTPS_PROXY ?? process.env.https_proxy;
  if (!proxy?.startsWith("http")) {
    return {};
  }
  const bypass = process.env.NO_PROXY ?? process.env.no_proxy;
  return {
    HTTPS_PROXY: proxy,
    HTTP_PROXY: process.env.HTTP_PROXY ?? process.env.http_proxy ?? proxy,
    // Loopback must never go through the proxy: local model discovery probes 127.0.0.1, and a
    // proxy that answers for it turns a real endpoint check into a bogus result.
    NO_PROXY: bypass ? `127.0.0.1,localhost,${bypass}` : "127.0.0.1,localhost",
  };
}

// CI runs `smoke.e2e.mjs` only, which is the coverage it had before these specs existed. The other
// thirteen stay in the repository and run locally; they are not ready to gate a pull request.
//
// Three things have to be true before one joins the gate, and none is yet:
//
// - It has to hold on all three runners. These were written against the Windows runtime in one
//   pass, and their first contact with macOS and Linux failed for three unrelated reasons: Linux
//   CI has no Secret Service, so anything touching the credential store returns
//   `communications-credential-read-failed` where it should report BLOCKED; the driver repeatedly
//   failed to create a session at all, four specs into the run; and WebKit does not present the
//   Settings shell the way WebView2 does, so the page probes miss.
// - It has to be stable. A full Windows run still turns up `database is locked` in the OnePiece
//   send, with every spec sharing one data directory and one app instance following another.
// - It has to be worth its wall clock. The full suite is roughly fifteen minutes on the Windows
//   runner, against a gate that is otherwise a couple of minutes for this job.
//
// A flaky fifteen-minute job in front of every pull request teaches people to ignore the Desktop
// Smoke result, which costs more than the coverage buys. Run them with
// `npm run test:desktop` locally, or set `VANEHUB_DESKTOP_FULL_SUITE=1` in a workflow that wants
// them, and promote them into the gate one at a time as each is made to hold.
const FULL_SUITE = path.join(configDir, "specs", "**", "*.e2e.mjs");
const GATE_SUITE = path.join(configDir, "specs", "smoke.e2e.mjs");
// Opt in, and off by default wherever CI is set. `VANEHUB_DESKTOP_FULL_SUITE=1` forces it back on.
const runFullSuite = process.env.VANEHUB_DESKTOP_FULL_SUITE === "1" || !process.env.CI;
const specs = runFullSuite ? [FULL_SUITE] : [GATE_SUITE];
if (!runFullSuite) {
  // Said out loud rather than left to whoever compares spec counts between runs: a run that
  // covers one spec instead of thirteen still prints PASSED.
  process.stdout.write(
    "Desktop specs: gate run (smoke only). Set VANEHUB_DESKTOP_FULL_SUITE=1 for all 14.\n",
  );
}

export const config = {
  runner: "local",
  specs,
  maxInstances: 1,
  services: [["tauri", {
    appBinaryPath: artifactPath,
    driverProvider: "embedded",
    embeddedPort: Number(process.env.VANEHUB_WEBDRIVER_PORT ?? 4445),
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
      // Isolates the app's CLI global-config reads and writes (permission hook projection
      // included) the same way the data dir isolates SQLite — without it a spec that assigns
      // claude-code a policy template edits the user's real ~/.claude/settings.json.
      VANEHUB_CLI_CONFIG_HOME: process.env.VANEHUB_CLI_CONFIG_HOME,
      VANEHUB_TEST_RUN_ID: process.env.VANEHUB_TEST_RUN_ID,
      VANEHUB_DESKTOP_RESULT_DIR: resultDir,
      // CLI Agents launched by the app inherit this environment, and on a network that requires
      // an egress proxy they cannot reach their provider without it. Forwarded only when the host
      // set it. `socks5://` is deliberately not accepted here: Node's undici -- which
      // `claude` uses -- ignores it and retries until the turn times out, which reads as an
      // authentication failure rather than a proxy misconfiguration.
      ...proxyEnvironment(),
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
  // Above the longest in-spec wait. A session case waits on a cold CLI start plus a provider round
  // trip through an egress proxy; when mocha's ceiling was the lower of the two it killed the test
  // first, which reads as "the Agent never answered" rather than "the harness stopped waiting".
  mochaOpts: { ui: "bdd", timeout: 300_000 },
  // Named per test. A single fixed filename meant the last failure in a run overwrote every
  // earlier one, so the run with the most to diagnose kept the least evidence -- one screen-sweep
  // failure was already lost this way, to screenshots taken by a later spec.
  afterTest: async (test, _context, result) => {
    if (result.passed) {
      return;
    }
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
  },
  // Every session is asked to shut down gracefully, not just whichever spec happens to sort last.
  //
  // The harness's clean-shutdown verdict reads a marker the runtime writes on exit, and all specs
  // in a run share one marker file, so the verdict reflects only the final app instance. While
  // `smoke.e2e.mjs` sorted last that worked by accident; adding the `ui-*` specs moved a file that
  // deliberately does not exit itself into last place, and a run with all 14 specs green reported
  // "The desktop runtime did not record a clean shutdown." Doing it here makes the verdict a
  // property of the runtime rather than of filename ordering, and turns every spec into evidence
  // that the app can exit on request instead of only one.
  //
  // Best effort: a spec that already exited, or one whose window is gone, must not turn a passing
  // file red on the way out.
  after: async () => {
    try {
      await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
    } catch {
      // Already gone, or refusing to answer -- either way the marker keeps whatever it holds and
      // the verdict below reports it.
    }
  },
  onPrepare: async () => {
    await mkdir(path.join(resultDir, "screenshots"), { recursive: true });
  },
  // The skipped count is recorded, not just the exit code. A host without a proxy, provider key,
  // SSH target or installed CLI silently skips whole capabilities and the run still exits 0 --
  // reporting only PASSED there presents reduced coverage as a clean bill of health.
  onComplete: async (exitCode, _config, _capabilities, results) => {
    await writeFile(
      path.join(resultDir, "wdio-result.json"),
      `${JSON.stringify({
        exitCode,
        passed: results?.passed ?? null,
        failed: results?.failed ?? null,
        skipped: results?.skipped ?? null,
      }, null, 2)}\n`,
    );
  },
};
