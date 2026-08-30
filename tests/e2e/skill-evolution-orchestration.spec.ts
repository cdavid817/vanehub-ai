import { expect, test } from "@playwright/test";

test.beforeEach(async ({ page }) => {
  await page.addInitScript(() => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({
      applicationLanguage: "en",
      theme: "futuristic",
    }));
  });
});

test("opens the orchestration deep link and records an observe-only Web decision", async ({ page }) => {
  await page.goto("/settings?section=skills&skillWorkspace=orchestration&workspace=e2e-workspace");
  await expect(page.getByRole("heading", { name: "Evolution orchestration" })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Evolution orchestration" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByLabel("Workspace identifier")).toHaveValue("e2e-workspace");
  await expect(page.getByText("Web simulation")).toBeVisible();

  await page.getByRole("tab", { name: "Policy" }).click();
  await page.getByRole("radio", { name: /Observe/ }).click();
  await page.getByLabel("Allowed Skill IDs").fill("code-review");
  await page.getByLabel(/locally consent/).check();
  await page.getByRole("button", { name: "Save policy" }).click();
  await page.getByRole("button", { name: "Request manual run" }).click();
  await page.getByRole("tab", { name: "Decisions & history" }).click();
  await expect(page.getByText("Would apply").first()).toBeVisible();
  await expect(page.getByText("No automatic applications have been committed.")).toBeVisible();
});

test("keeps orchestration controls bounded at narrow width", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/settings?section=skills&skillWorkspace=orchestration&workspace=narrow-workspace");
  await expect(page.getByRole("heading", { name: "Evolution orchestration" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Request manual run" })).toBeVisible();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
});
