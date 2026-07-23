import { expect, test, type Locator, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

test("moves a Session by drag and by the accessible category menu", async ({ page }) => {
  const sessionTitle = `分类交互-${Date.now()}`;
  const sourceName = `来源-${Date.now()}`;
  const targetName = `目标-${Date.now()}`;

  await page.goto("/");
  await createSession(page, sessionTitle);

  const sessionCard = page.locator("[data-session-id]").filter({ hasText: sessionTitle });
  await createAndAssignCategory(page, sessionCard, sourceName);
  await createAndAssignCategory(page, sessionCard, targetName);

  await page.getByRole("button", { name: "分类", exact: true }).click();
  const sourceSection = page.locator("[data-session-category-id]").filter({ hasText: sourceName });
  const targetSection = page.locator("[data-session-category-id]").filter({ hasText: targetName });
  await targetSection.getByRole("button").first().click();
  await expect(targetSection.locator("[data-session-id]").filter({ hasText: sessionTitle })).toBeVisible();

  await targetSection.locator("[data-session-id]").filter({ hasText: sessionTitle }).dragTo(sourceSection);
  await expect(sourceSection.getByRole("button").first()).toContainText("1");
  await sourceSection.getByRole("button").first().click();
  const movedCard = sourceSection.locator("[data-session-id]").filter({ hasText: sessionTitle });
  await expect(movedCard).toBeVisible();

  await movedCard.click({ button: "right" });
  await page.getByRole("button", { name: "未分类", exact: true }).click();
  const uncategorized = page.locator('[data-session-category-id="uncategorized"]');
  await expect(uncategorized.getByRole("button").first()).toContainText("1");
  await uncategorized.getByRole("button").first().click();
  await expect(uncategorized.locator("[data-session-id]").filter({ hasText: sessionTitle })).toBeVisible();
});

async function createAndAssignCategory(
  page: Page,
  sessionCard: Locator,
  name: string,
) {
  await sessionCard.click({ button: "right" });
  page.once("dialog", (dialog) => dialog.accept(name));
  await page.getByRole("button", { name: "新建分类", exact: true }).click();
  await sessionCard.click({ button: "right" });
  await expect(page.getByRole("button", { name, exact: true })).toBeVisible();
  await page.mouse.click(2, 2);
}
