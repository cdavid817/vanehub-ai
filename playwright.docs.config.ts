import { defineConfig, devices } from "@playwright/test";

const screenshotPort = Number(process.env.DOCS_SCREENSHOT_PORT);
if (!Number.isSafeInteger(screenshotPort) || screenshotPort < 1 || screenshotPort > 65_535) {
  throw new Error("DOCS_SCREENSHOT_PORT must contain an allocated loopback port.");
}
const baseURL = `http://127.0.0.1:${screenshotPort}`;

export default defineConfig({
  testDir: "./tests/docs",
  outputDir: ".docs-screenshots/test-results",
  timeout: 60_000,
  workers: 1,
  retries: 0,
  reporter: "list",
  expect: {
    timeout: 10_000,
  },
  use: {
    ...devices["Desktop Chrome"],
    baseURL,
    colorScheme: "dark",
    reducedMotion: "reduce",
    trace: "retain-on-failure",
    video: "off",
    screenshot: "off",
  },
  webServer: {
    command: `npm run dev -- --host 127.0.0.1 --port ${screenshotPort}`,
    url: baseURL,
    reuseExistingServer: false,
  },
  projects: [
    {
      name: "docs-chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
