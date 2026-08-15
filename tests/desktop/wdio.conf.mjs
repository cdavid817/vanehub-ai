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

export const config = {
  runner: "local",
  specs: [path.join(configDir, "specs", "**", "*.e2e.mjs")],
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
      VANEHUB_TEST_RUN_ID: process.env.VANEHUB_TEST_RUN_ID,
      VANEHUB_DESKTOP_RESULT_DIR: resultDir,
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
  mochaOpts: { ui: "bdd", timeout: 120_000 },
  afterTest: async (_test, _context, result) => {
    if (!result.passed) {
      try {
        await globalThis.browser.saveScreenshot(path.join(resultDir, "screenshots", "desktop-smoke-failure.png"));
      } catch (error) {
        await writeFile(
          path.join(resultDir, "screenshots", "unavailable.txt"),
          `Failure screenshot unavailable: ${error instanceof Error ? error.message : String(error)}\n`,
        );
      }
    }
  },
  onPrepare: async () => {
    await mkdir(path.join(resultDir, "screenshots"), { recursive: true });
  },
  onComplete: async (exitCode) => {
    await writeFile(path.join(resultDir, "wdio-result.json"), `${JSON.stringify({ exitCode }, null, 2)}\n`);
  },
};
