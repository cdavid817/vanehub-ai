import { defineConfig, devices } from "@playwright/test";

const managedPort = process.env.PLAYWRIGHT_TEST_PORT;
const baseURL = managedPort
  ? `http://127.0.0.1:${managedPort}`
  : process.env.PLAYWRIGHT_BASE_URL ?? "http://127.0.0.1:1420";

export default defineConfig({
  testDir: "./tests/e2e",
  workers: 1,
  timeout: 30_000,
  expect: {
    timeout: 5_000,
  },
  use: {
    baseURL,
    trace: "on-first-retry",
  },
  webServer: managedPort
    ? {
        command: `npm run dev -- --host 127.0.0.1 --port ${managedPort} --strictPort`,
        reuseExistingServer: false,
        timeout: 120_000,
        url: baseURL,
      }
    : undefined,
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
