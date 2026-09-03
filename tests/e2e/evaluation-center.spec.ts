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

/**
 * 18.4 moved task/Agent configuration out of the header and into `EvaluationRunWizard`'s own
 * guided Sheet (task -> Agents -> Review) -- opens it, optionally adjusts the Agent step's own
 * selection (18.5), then advances to Review and clicks the (relocated, but same-labeled) Run
 * action.
 */
async function openWizardAndRun(page: Page, customizeAgentStep?: () => Promise<void>) {
  await page.getByRole("button", { name: "Configure evaluation" }).click();
  await page.getByRole("button", { name: "Next" }).click();
  if (customizeAgentStep) await customizeAgentStep();
  await page.getByRole("button", { name: "Next" }).click();
  await page.getByRole("button", { name: "Run arena" }).click();
}

test("runs, compares, filters, inspects, and exports the complete mock benchmark", async ({ page }) => {
  await openEvaluation(page, "futuristic", 1440);
  await openWizardAndRun(page, async () => {
    for (const agentId of ["claude-code", "opencode", "codex-cli", "gemini-cli", "antigravity-cli", "onepiece"]) {
      await page.getByTestId(`evaluation-agent-${agentId}`).uncheck();
    }
    await page.getByTestId("evaluation-agent-onepiece").check();
    await page.getByTestId("evaluation-agent-codex-cli").check();
  });
  // Scoped to result rows via the shared "evaluation-row" testid (present on both the desktop
  // <table> and the narrow-viewport compact-card fallback -- `DataTableBody.tsx` spreads
  // `getRowMeta`'s attributes onto whichever element it renders): 18.12 added an outcome badge to
  // the detail pane too, and `start()` auto-selects the arena's first attempt, so an unscoped text
  // locator can resolve to two elements (the row and the now-rendered detail pane) once that first
  // attempt succeeded. Each mock outcome is unique per row (`web-evaluation-client.ts`: only index 0
  // ever succeeds), so filtering rows by outcome text stays unambiguous.
  const passedRow = page.getByTestId("evaluation-row").filter({ hasText: "Passed" });
  const taskFailedRow = page.getByTestId("evaluation-row").filter({ hasText: "Task failed" });
  await expect(passedRow).toBeVisible();
  await expect(taskFailedRow).toBeVisible();
  await page.getByLabel("Filter results").fill("codex-cli");
  await taskFailedRow.click();
  await expect(page.getByText("Metrics and provenance")).toBeVisible();
  await expect(page.getByText(/unavailable · provider/)).toBeVisible();
  const download = page.waitForEvent("download");
  await page.getByRole("button", { name: "Export JSON" }).click();
  expect((await download).suggestedFilename()).toMatch(/^web-eval-\d+\.json$/);
});

// 20.19: picks a baseline + candidate through `EvaluationComparisonPanel` (evaluation-comparison-
// panel.tsx, tasks 18.8-18.11) with no pointer at all. Both pickers are plain native `<select>`s --
// once focused (not expanded), ArrowDown/ArrowUp change the selected option and fire a real `change`
// event directly, standard cross-browser `<select>` keyboard behavior, so no click is needed to open
// either dropdown first. `openWizardAndRun(page)` with no `customizeAgentStep` keeps every default
// Agent checked (the pre-existing functional test above explicitly unchecks that same six-agent set
// after `openWizardAndRun`, proving it is the default), so the arena has six attempts -- comfortably
// enough for ArrowDown-once and ArrowDown-twice from the empty placeholder to land on two distinct
// attempts deterministically.
test("keyboard-only: picks a baseline and candidate and reads the comparison result", async ({ page }) => {
  await openEvaluation(page, "futuristic", 1440);
  await openWizardAndRun(page);

  const comparison = page.getByTestId("evaluation-comparison");
  const baselineSelect = comparison.getByTestId("evaluation-comparison-baseline");
  const candidateSelect = comparison.getByTestId("evaluation-comparison-candidate");
  await expect(baselineSelect).toHaveValue("");
  await expect(candidateSelect).toHaveValue("");

  await baselineSelect.focus();
  await page.keyboard.press("ArrowDown");
  await candidateSelect.focus();
  await page.keyboard.press("ArrowDown");
  await page.keyboard.press("ArrowDown");

  const baselineValue = await baselineSelect.inputValue();
  const candidateValue = await candidateSelect.inputValue();
  expect(baselineValue).not.toBe("");
  expect(candidateValue).not.toBe("");
  expect(baselineValue).not.toBe(candidateValue);

  await expect(comparison.getByTestId("evaluation-comparison-result")).toBeVisible();
});

for (const variant of [
  { name: "futuristic-desktop", theme: "futuristic" as const, width: 1440 },
  { name: "minimal-desktop", theme: "minimal" as const, width: 1440 },
  { name: "futuristic-narrow", theme: "futuristic" as const, width: 390 },
  { name: "minimal-narrow", theme: "minimal" as const, width: 390 },
]) {
  test(`evaluation workspace visual ${variant.name}`, async ({ page }, testInfo) => {
    await openEvaluation(page, variant.theme, variant.width);
    await openWizardAndRun(page);
    // Scoped to the result row -- see the same-named comment in the functional test above. This
    // must work in the 390px-narrow compact-card fallback too, where there is no <table> at all.
    await page.getByTestId("evaluation-row").filter({ hasText: "Passed" }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await page.getByTestId("evaluation-center").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
  });
}
