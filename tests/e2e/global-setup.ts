import { chromium, type FullConfig } from "@playwright/test";

/**
 * Warms the dev server's module graph before any worker navigates.
 *
 * Playwright's `webServer.url` probe only proves the server answers HTTP — it serves
 * `index.html` immediately, long before Vite has transformed the app's module graph. The
 * workers then start together and race that first transform, and the one that loses sits on the
 * static `Starting...` shell until the navigation timeout. It reads as a deterministic UI
 * regression in whichever spec happens to sort first, which is how it was found: the first spec
 * of a run failed while every later spec, including others in the same file using the same
 * helper, passed.
 *
 * One navigation here pays that cost once, serially, outside any test's timeout.
 */
export default async function globalSetup(config: FullConfig) {
  const baseURL = config.projects[0]?.use.baseURL;
  if (!baseURL) {
    return;
  }
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    await page.goto(baseURL, { waitUntil: "load", timeout: 180_000 });
    // `index.html` ships a static `Starting...` overlay, so `load` fires long before the app
    // mounts. The overlay leaves only after React commits, which proves the module graph is warm.
    await page.locator("#bootstrap-shell").waitFor({ state: "detached", timeout: 180_000 });
  } finally {
    await browser.close();
  }
}
