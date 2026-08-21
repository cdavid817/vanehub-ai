import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const configDir = path.dirname(fileURLToPath(import.meta.url));

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
    afterTest: async (test, _context, result) => {
      if (!result.passed) {
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
