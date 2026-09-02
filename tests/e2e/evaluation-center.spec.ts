import { expect, test, type Page } from "@playwright/test";

type Theme = "futuristic" | "minimal";

async function openEvaluation(page: Page, theme: Theme, width: number) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme }));
  }, theme);
  await page.goto("/");
  // The nine-entry activity bar (with a direct "Evaluations" primary entry) was replaced by five
  // stable domains before this task (design.md Decision 1, workspace-activity-bar.tsx) -- Evaluation
  // is reached through Quality's own single section, not a dedicated top-level button anymore. This
  // spec's own locator was never updated for that rename, a pre-existing break unrelated to §18.
  await page.getByRole("button", { name: "Quality" }).click();
  await expect(page.getByTestId("evaluation-center")).toBeVisible();
}

test("runs, compares, filters, inspects, and exports the complete mock benchmark", async ({ page }) => {
  await openEvaluation(page, "futuristic", 1440);
  for (const agentId of ["claude-code", "opencode", "codex-cli", "gemini-cli", "antigravity-cli", "onepiece"]) {
    await page.getByTestId(`evaluation-agent-${agentId}`).uncheck();
  }
  await page.getByTestId("evaluation-agent-onepiece").check();
  await page.getByTestId("evaluation-agent-codex-cli").check();
  await page.getByRole("button", { name: "Run arena" }).click();
  await expect(page.getByText("Passed", { exact: true })).toBeVisible();
  await expect(page.getByText("Task failed", { exact: true })).toBeVisible();
  await page.getByLabel("Filter results").fill("codex-cli");
  await page.getByText("Task failed", { exact: true }).click();
  await expect(page.getByText("Metrics and provenance")).toBeVisible();
  await expect(page.getByText(/unavailable · provider/)).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export JSON" }).click();
  expect((await download).suggestedFilename()).toMatch(/^web-eval-\d+\.json$/);
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`evaluation workspace visual ${variant.name}`, async ({ page }, testInfo) => {
    await openEvaluation(page, variant.theme, variant.width);
    await page.getByRole("button", { name: "Run arena" }).click();
    await page.getByText("Passed", { exact: true }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.getByTestId("evaluation-center").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
