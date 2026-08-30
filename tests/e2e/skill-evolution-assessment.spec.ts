import { expect, type Page, test } from "@playwright/test";

async function openAssessment(page: Page, scenario: string) {
  await page.addInitScript((fixture) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en" }));
    window.localStorage.setItem("vanehub.skillAssessmentScenario", fixture);
  }, scenario);
  await page.goto("/settings");
  await page.getByRole("button", { name: "Skills", exact: true }).click();
  await page.locator('article[data-skill-id="readme-generation"]').getByRole("button", { name: /View details for/ }).click();
  const inspector = page.getByRole("complementary", { name: /details/ });
  await expect(inspector.getByRole("heading", { name: "Skill assessment" })).toBeVisible();
  return inspector;
}

test.describe("Skill evolution assessment", () => {
  test("shows a verified low-risk advance result without mutation authority", async ({ page }) => {
    const inspector = await openAssessment(page, "deterministic");
    await expect(inspector).toContainText("Selected by relevance");
    await expect(inspector).toContainText("9 of 9 passed");
    await expect(inspector).toContainText("Ready for later governance");
    await expect(inspector).toContainText("Verified attribution");
    await expect(inspector.getByRole("button", { name: /Approve|Reject|Apply|Override target|Write memory|Unpin|Archive|Automatic evolution/i })).toHaveCount(0);
  });

  for (const fixture of [
    { scenario: "ambiguous", expected: "Ambiguous alternatives", provenance: "Deterministic" },
    { scenario: "model-assisted", expected: "Selected by relevance", provenance: "mock-judge" },
    { scenario: "model-invalid", expected: "invalid_schema", provenance: "Deterministic fallback" },
    { scenario: "fallback", expected: "provider_unavailable", provenance: "Deterministic fallback" },
  ]) {
    test(`keeps ambiguous selection safe for ${fixture.scenario}`, async ({ page }) => {
      const inspector = await openAssessment(page, fixture.scenario);
      await expect(inspector).toContainText(fixture.expected);
      await expect(inspector).toContainText(fixture.provenance);
      await expect(inspector).toContainText("Needs human review");
    });
  }

  for (const fixture of [
    { scenario: "privacy", check: "Privacy residue", reason: "privacy_residue_detected", route: "Drop" },
    { scenario: "duplicate", check: "Duplicate knowledge", reason: "canonical_duplicate", route: "Merge duplicate later" },
    { scenario: "transient", check: "Transient incident", reason: "workspace_local_fact", route: "Record as memory only" },
    { scenario: "contradiction", check: "Evidence consistency", reason: "material_contradiction", route: "Needs human review" },
    { scenario: "executable", check: "Executable-content risk", reason: "executable_expansion", route: "Needs human review" },
    { scenario: "pinned", check: "Target lifecycle mutability", reason: "target_pinned", route: "Record as memory only" },
    { scenario: "archived", check: "Target lifecycle mutability", reason: "target_archived", route: "Drop" },
  ]) {
    test(`explains the ${fixture.scenario} quality route`, async ({ page }) => {
      const inspector = await openAssessment(page, fixture.scenario);
      await expect(inspector).toContainText(fixture.route);
      const check = inspector.locator("details").filter({ hasText: fixture.check }).first();
      await check.locator("summary").click();
      await expect(check).toContainText(fixture.reason);
    });
  }

  for (const fixture of ["changed-evidence", "changed-revision", "policy-upgrade"]) {
    test(`preserves superseded ${fixture} witnesses`, async ({ page }) => {
      const inspector = await openAssessment(page, fixture);
      await expect(inspector.getByRole("button", { name: "Inspect", exact: true })).toHaveCount(2);
      if (fixture === "changed-revision") await expect(inspector).toContainText("revision-hash-2");
      if (fixture !== "changed-revision") {
        await inspector.locator("details").filter({ hasText: "Version and witness audit" }).locator("summary").click();
        await expect(inspector).toContainText(fixture === "changed-evidence" ? "revision-2" : "selector-v2");
      }
    });
  }

  test("shows revoked consent and coalesces duplicate reassessment", async ({ page }) => {
    let inspector = await openAssessment(page, "consent-revocation");
    await expect(inspector.getByRole("button", { name: "Enable evaluation" })).toBeDisabled();
    await page.evaluate(() => window.localStorage.setItem("vanehub.skillAssessmentScenario", "duplicate-request"));
    await page.reload();
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    await page.locator('article[data-skill-id="readme-generation"]').getByRole("button", { name: /View details for/ }).click();
    inspector = page.getByRole("complementary", { name: /details/ });
    const reassess = inspector.getByRole("button", { name: "Request reassessment" });
    await reassess.click();
    await expect(inspector).toContainText("Reassessment queued");
    await reassess.click();
    await expect(inspector).toContainText("already current or queued");
  });

  for (const fixture of [
    { scenario: "attribution-native", label: "Verified attribution" },
    { scenario: "attribution-utility", label: "Verified attribution" },
    { scenario: "attribution-managed", label: "Correlated attribution" },
    { scenario: "attribution-interactive", label: "Weak attribution" },
  ]) {
    test(`retains ${fixture.scenario} attribution fidelity`, async ({ page }) => {
      const inspector = await openAssessment(page, fixture.scenario);
      await expect(inspector).toContainText(fixture.label);
    });
  }
});
