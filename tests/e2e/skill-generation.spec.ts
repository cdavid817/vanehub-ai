import { expect, test } from "@playwright/test";

test.describe("Skill Evolution generation", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en" }));
    });
    await page.goto("/settings?section=skills");
    await page.getByRole("tab", { name: "Generation lab" }).click();
    await page.getByLabel("Workspace ID").fill("mock://generation");
    await page.getByRole("button", { name: "Open workspace" }).click();
  });

  test("reviews a bounded draft and thirteen-section dossier without mutation controls", async ({ page }) => {
    await page.getByText("seed-mock-generation-2", { exact: true }).click();
    await expect(page.getByText("Seven-stage execution")).toBeVisible();
    await expect(page.getByText("Locally rendered draft")).toBeVisible();
    await expect(page.getByText("Validation matrix")).toBeVisible();
    await expect(page.getByLabel("Evidence citations")).toBeVisible();
    await page.getByRole("tab", { name: "Evidence dossier" }).click();
    await expect(page.getByRole("navigation", { name: "Evidence dossier sections" }).getByRole("button")).toHaveCount(13);
    await expect(page.getByRole("button", { name: "Next page" })).toBeVisible();
    await page.getByRole("button", { name: "JSON" }).click();
    await expect(page.getByText(/sanitized export was saved/i)).toBeVisible();
    await expect(page.getByRole("button", { name: /apply|install|approve/i })).toHaveCount(0);
    await page.getByRole("button", { name: "Send to Curator" }).click();
    await page.getByRole("button", { name: "Open Curator" }).click();
    await expect(page.getByRole("heading", { name: "Curator" })).toBeVisible();
    await expect(page.getByLabel("Workspace identifier")).toHaveValue("mock://generation");
  });

  test("cancels and regenerates as a linked immutable attempt on a narrow screen", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 820 });
    await page.getByText("seed-mock-generation-1", { exact: true }).click();
    await page.getByRole("button", { name: "Cancel" }).click();
    await expect(page.getByRole("button", { name: "Regenerate" })).toBeEnabled();
    await page.getByRole("button", { name: "Regenerate" }).click();
    await expect(page.getByText(/Supersedes mock-generation-1/)).toBeVisible();
    const overflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(overflow).toBe(false);
  });
});
