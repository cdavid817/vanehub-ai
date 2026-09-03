import { defineConfig, devices } from "@playwright/test";

const developmentPort = process.env.PLAYWRIGHT_VISUAL_PORT ?? "5191";
const developmentUrl = `http://127.0.0.1:${developmentPort}`;

/**
 * Tasks 21.17-21.19: the first real Playwright `toHaveScreenshot()` baseline-comparison slice
 * (design.md Decision 20's test layer 7, "visual regression: a core matrix, not every random
 * state"). A separate config from `playwright.config.ts`, following this repo's own established
 * precedent (`playwright.docs.config.ts`, `playwright.local-media.config.ts`) of one dedicated
 * config per test slice that must not run inside the default `npx playwright test` invocation:
 *
 * - `tests/e2e/**\/*.spec.ts` (including the pre-existing `*.visual.spec.ts` files there) use
 *   `page.screenshot({ path: testInfo.outputPath(...) })` -- artifact capture into the gitignored,
 *   per-run-wiped `test-results/e2e/` directory, proving a surface renders without crashing but
 *   never diffing against a committed baseline. That is a real, different, already-adequate layer
 *   (design.md's own layer 4) and is not replaced by this file.
 * - This config's tests use real `expect(...).toHaveScreenshot()` against baseline PNGs committed
 *   under `tests/e2e-visual-regression/*-snapshots/` -- Playwright's own default
 *   `snapshotPathTemplate`, left unmodified: no other genuine baseline-comparison convention exists
 *   anywhere in this repo to follow instead (confirmed by grepping the whole tree for
 *   `toHaveScreenshot`/`toMatchSnapshot` before adding this file).
 *
 * Deliberately excluded from `playwright.config.ts`'s own `testDir` (`./tests/e2e`), and therefore
 * from CI's plain `npx playwright test` step in `e2e` job: Playwright's default `toHaveScreenshot`
 * baseline filename embeds `process.platform` (confirmed by reading
 * `playwright/lib/worker/workerProcessEntry.js`'s own `legacyTemplate`), and CI's `e2e` job runs on
 * `runs-on: ubuntu-latest` with no pinned Docker image, while every baseline this increment commits
 * was generated on Windows. Wiring this into that job today would fail every test with "no baseline
 * for linux" rather than skip cleanly. Run explicitly via `npm run visual:test` /
 * `npm run visual:update` -- see `tasks.md` 21.18 evidence for the full disclosure and what real
 * Linux-runner wiring would still need (either a pinned `mcr.microsoft.com/playwright` image step in
 * `ci.yml`, generating its own Linux baselines once, or accepting Windows-only local coverage).
 */
export default defineConfig({
  testDir: "./tests/e2e-visual-regression",
  outputDir: "./test-results/e2e-visual-regression",
  timeout: 90_000,
  workers: 1,
  retries: 0,
  reporter: "list",
  expect: {
    timeout: 10_000,
    toHaveScreenshot: {
      // Playwright's own default already freezes CSS animations/transitions at their end state for
      // this assertion; kept explicit here rather than relying on the default so a future Playwright
      // major version change can't silently loosen it without this file's own diff calling it out.
      animations: "disabled",
      // A small, non-zero tolerance for anti-aliasing/font-hinting drift between otherwise-identical
      // runs on the same machine -- 0 is commonly too strict to be stable, an unbounded value would
      // hide real regressions. 1% is a conventional starting point, not load-bearing evidence itself.
      maxDiffPixelRatio: 0.01,
    },
  },
  use: {
    baseURL: process.env.PLAYWRIGHT_BASE_URL ?? developmentUrl,
    // Pins the browser-level OS colour-scheme preference so no surface can silently pick up an
    // OS-default `prefers-color-scheme` fallback before this app's own `data-theme` attribute (set
    // explicitly per test) applies -- independent of this app's own futuristic/minimal theme system.
    colorScheme: "dark",
    // Task 21.18's "reduced animation": matches this repo's own `useReducedMotion` hook (task
    // 20.12), which reads the real `(prefers-reduced-motion: reduce)` media query.
    reducedMotion: "reduce",
    // Task 21.18's "deterministic dates": pins `Intl`/`Date` formatting to a fixed timezone so a
    // surface that ever renders a wall-clock-derived timestamp does not drift between a run taken at
    // one time of day and a rerun taken at another.
    timezoneId: "UTC",
    trace: "retain-on-failure",
    video: "off",
    screenshot: "off",
  },
  webServer: {
    command: `npm run dev -- --port ${developmentPort} --strictPort`,
    url: developmentUrl,
    reuseExistingServer: false,
  },
  projects: [
    {
      name: "visual-chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
