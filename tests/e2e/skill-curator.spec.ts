import { expect, test, type Page } from "@playwright/test";

async function openWorkspace(page: Page, workspace: string) {
  await page.getByLabel("Workspace identifier").fill(workspace);
  await page.getByRole("button", { name: "Open queue" }).click();
  await page.getByRole("listitem").click();
}

async function createDraft(page: Page) {
  await page.getByLabel("Learned guidance").fill("Prefer bounded changes.");
  await page.getByLabel("Evidence-bound rationale").fill("Sanitized evidence supports this guidance.");
  await page.getByLabel("Expected effective change").fill("Adds one guidance block.");
  await page.getByRole("button", { name: "Save draft" }).click();
  await expect(page.getByRole("button", { name: "Create current preview" })).toBeEnabled();
}

async function createPreview(page: Page) {
  await createDraft(page);
  await page.getByRole("button", { name: "Create current preview" }).click();
  await expect(page.getByRole("button", { name: "Final effective change" })).toBeVisible();
}

test.describe("Skill Evolution Curator", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en" }));
    });
    await page.goto("/settings?section=skills");
    await page.getByRole("tab", { name: "Curator" }).click();
  });

  test("reviews one witnessed candidate without bulk or automatic approval", async ({ page }) => {
    await page.getByLabel("Workspace identifier").fill("mock://deterministic");
    await page.getByRole("button", { name: "Open queue" }).click();
    await expect(page.getByText("1 candidates")).toBeVisible();
    await page.getByRole("listitem").click();
    await expect(page.getByText("9 / 9 checks")).toBeVisible();
    await expect(page.getByText(/Target override, base editing/)).toBeVisible();

    await createPreview(page);
    const approval = page.getByRole("button", { name: "Approve and apply Overlay" });
    await expect(approval).toBeDisabled();
    await page.getByLabel(/I reviewed this exact effective diff/).check();
    await approval.click();
    await expect(page.getByRole("link", { name: /Open applied Overlay history/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /approve all/i })).toHaveCount(0);
    await expect(page.getByRole("checkbox", { name: /auto-apply/i })).toHaveCount(0);
  });

  test("keeps the queue responsive and restores focus after a categorized decision", async ({ page }) => {
    await page.setViewportSize({ width: 375, height: 780 });
    await page.getByLabel("Workspace identifier").fill("mock://deterministic");
    await page.getByRole("button", { name: "Open queue" }).click();
    await page.getByRole("listitem").click();
    const defer = page.getByRole("button", { name: "Defer" });
    await defer.click();
    const dialog = page.getByRole("dialog", { name: "Defer candidate" });
    await expect(dialog.getByLabel("Required reason category")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(defer).toBeFocused();
    await defer.click();
    await dialog.getByLabel("Required reason category").selectOption("need_more_evidence");
    await dialog.getByRole("button", { name: "Defer" }).click();
    await page.getByRole("button", { name: "Resume review" }).click();
    await expect(page.getByRole("button", { name: "Defer" })).toBeVisible();
    const hasHorizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(hasHorizontalOverflow).toBe(false);
  });

  test("rejects a candidate with a bounded categorized decision", async ({ page }) => {
    await openWorkspace(page, "mock://deterministic");
    await page.getByRole("button", { name: "Reject" }).click();
    const dialog = page.getByRole("dialog", { name: "Reject candidate" });
    await dialog.getByLabel("Required reason category").selectOption("not_useful");
    await dialog.getByLabel("Optional note").fill("The bounded evidence does not justify this mutation.");
    await dialog.getByRole("button", { name: "Reject" }).click();
    await expect(page.locator('article[aria-labelledby="curator-candidate-title"]').getByText("Rejected", { exact: true })).toBeVisible();
  });

  test("invalidates a stale preview after a policy witness change", async ({ page }) => {
    await openWorkspace(page, "mock://deterministic");
    await createPreview(page);
    await page.getByText("Queue policy and retention").click();
    await page.getByLabel("Immediate notifications").uncheck();
    await page.getByRole("button", { name: "Save policy" }).click();
    await expect(page.getByText(/Current witnesses changed: policy_changed/)).toBeVisible();
    await expect(page.getByRole("button", { name: "Approve and apply Overlay" })).toHaveCount(0);
  });

  test("refuses pinned targets and recovers an apply failure through a new review", async ({ page }) => {
    await openWorkspace(page, "mock://pinned");
    await createDraft(page);
    await page.getByRole("button", { name: "Create current preview" }).click();
    await expect(page.getByRole("alert").filter({ hasText: "target_pinned" })).toBeVisible();

    await page.getByLabel("Workspace identifier").fill("mock://application-failure");
    await page.getByRole("button", { name: "Open queue" }).click();
    await page.getByRole("listitem").click();
    await createPreview(page);
    await page.getByLabel(/I reviewed this exact effective diff/).check();
    await page.getByRole("button", { name: "Approve and apply Overlay" }).click();
    await expect(page.getByText(/Overlay application failed with stable category/)).toBeVisible();
    await page.getByRole("button", { name: "Prepare retry" }).click();
    await expect(page.getByRole("button", { name: "Create current preview" })).toBeEnabled();
  });
});
