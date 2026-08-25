import { defineConfig, devices } from "@playwright/test";

/**
 * Browser E2E for local media, driven by the deterministic fake.
 *
 * A separate config rather than a project inside `playwright.config.ts`, because the main browser
 * suite deliberately runs against an **un-instrumented** dev server -- `desktop-instrumentation-
 * boundary.test.ts` asserts that its command carries no build flag, and its specs may not even
 * mention one. Putting an instrumented server in the same config would either break that assertion
 * or quietly instrument the honest suite.
 *
 * The flag is passed through `webServer.env` rather than baked into a package script, so no
 * production entry point gains a way to request the fake.
 */
const developmentPort = process.env.PLAYWRIGHT_LOCAL_MEDIA_PORT ?? "5183";
const developmentUrl = `http://127.0.0.1:${developmentPort}`;

export default defineConfig({
  testDir: "./tests/e2e-local-media",
  timeout: 60_000,
  workers: 1,
  retries: process.env.CI ? 1 : 0,
  reporter: process.env.CI ? [["line"], ["html", { open: "never" }]] : "list",
  expect: { timeout: 10_000 },
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
    env: { VITE_LOCAL_MEDIA_FAKE: "1" },
  },
  projects: [{ name: "local-media-chromium", use: { ...devices["Desktop Chrome"] } }],
});
