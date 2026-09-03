import { defineConfig, devices } from "@playwright/test";

const developmentPort = process.env.PLAYWRIGHT_PORT ?? "5174";
const developmentUrl = `http://127.0.0.1:${developmentPort}`;

export default defineConfig({
  testDir: "./tests/e2e",
  // Not the default `test-results/`: Playwright empties its output directory on every run, and the
  // desktop layers keep their per-run evidence -- and the artifact marker the next desktop run
  // needs -- under `test-results/desktop/`. Sharing the directory means running the browser suite
  // silently destroys the evidence a desktop failure has to be explained from.
  outputDir: "./test-results/e2e",
  globalSetup: "./tests/e2e/global-setup.ts",
  timeout: 60_000,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["line"], ["html", { open: "never" }]] : "list",
  expect: {
    timeout: 10_000,
  },
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? developmentUrl,
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "retain-on-failure",
  },
  webServer: {
    command: `npm run dev -- --port ${developmentPort} --strictPort`,
    url: developmentUrl,
    reuseExistingServer: false,
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
