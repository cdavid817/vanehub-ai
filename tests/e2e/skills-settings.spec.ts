import { expect, test } from "@playwright/test";

test.describe("Skills management", () => {
  test.beforeEach(async ({ page }) => {
    await page.addInitScript(() => {
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en" }));
    });
  });

  test("manages global Skills by dynamic CLI Agent without scope controls", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();

    await expect(page.getByRole("heading", { name: "Skills", exact: true, level: 2 })).toBeVisible();
    await expect(page.getByRole("button", { name: /^All Skills/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Unassigned/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /^Global$/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /^Workspace$/ })).toHaveCount(0);

    const systemSkill = page.locator('article[data-skill-id="tdd-discipline"]');
    await expect(systemSkill).toContainText("System");
    await expect(systemSkill).toContainText("Role");
    await expect(systemSkill).toContainText("Read-only");
    await expect(systemSkill).not.toContainText("On demand");
    await expect(systemSkill.getByRole("button", { name: "Edit Skill" })).toHaveCount(0);
    await expect(systemSkill.getByRole("button", { name: "Delete Skill" })).toHaveCount(0);

    const migratedOverride = page.locator('article[data-skill-id="readme-generation"]');
    await expect(migratedOverride).toHaveCount(1);
    const detailsToggle = migratedOverride.getByRole("button", { name: /View details for/ });
    await expect(detailsToggle).toHaveAttribute("aria-expanded", "false");
    await detailsToggle.click();
    await expect(detailsToggle).toHaveAttribute("aria-expanded", "true");
    const inspector = page.getByRole("complementary", { name: /details/ });
    await expect(inspector).toContainText("Project > User > Registry > System");
    await expect(inspector.getByRole("heading", { name: "Definition precedence" })).toBeVisible();
    await expect(inspector.locator('[data-definition-state="effective"]')).toHaveCount(1);
    await expect(inspector.locator('[data-definition-state="shadowed"]')).toHaveCount(1);
    const inspectorBox = await inspector.boundingBox();
    const selectedRowBox = await migratedOverride.boundingBox();
    expect(inspectorBox).not.toBeNull();
    expect(selectedRowBox).not.toBeNull();
    expect(inspectorBox!.x).toBeGreaterThan(selectedRowBox!.x);
    await inspector.getByRole("button", { name: "Close Skill details" }).click();
    await expect(detailsToggle).toBeFocused();

    const previewButton = systemSkill.getByRole("button", { name: "Preview Skill" });
    await previewButton.click();
    const previewDialog = page.getByRole("dialog");
    await expect(previewDialog).toContainText("indexed resources");
    await expect(previewDialog).toContainText("read-only");
    await page.keyboard.press("Escape");
    await expect(previewDialog).toBeHidden();
    await expect(previewButton).toBeFocused();

    await page.getByRole("button", { name: /Codex CLI/ }).click();
    const board = page.getByTestId("skill-selection-board");
    const assigned = board.locator('[data-skill-group="assigned"]');
    const available = board.locator('[data-skill-group="available"]');
    await expect(assigned.getByRole("heading", { name: "Assigned" })).toBeVisible();
    await expect(available.getByRole("heading", { name: "Available" })).toBeVisible();
    const assignedBox = await assigned.boundingBox();
    const availableBox = await available.boundingBox();
    expect(assignedBox).not.toBeNull();
    expect(availableBox).not.toBeNull();
    expect(availableBox!.x).toBeGreaterThan(assignedBox!.x + 100);
    expect(Math.abs(availableBox!.y - assignedBox!.y)).toBeLessThan(8);
    await expect(page.getByRole("checkbox", { name: "Enabled" })).toHaveCount(0);
    await expect(page.getByRole("checkbox", { name: /Assign to|Unassign from/ })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /Edit Skill|Delete Skill/ })).toHaveCount(0);
    await expect(page.getByText("Globally enabled").first()).toBeVisible();

    const assignmentAction = page.getByRole("button", { name: /Assign to Codex CLI|Unassign from Codex CLI/ }).first();
    const assignmentRow = assignmentAction.locator("xpath=ancestor::article");
    await expect(assignmentRow.getByRole("button", { name: /View details for/ })).toBeVisible();
    await expect(assignmentRow.getByRole("button", { name: "Preview Skill" })).toBeVisible();
    const actionName = await assignmentAction.getAttribute("aria-label");
    const skillId = await assignmentAction.locator("xpath=ancestor::article").getAttribute("data-skill-id");
    await assignmentAction.focus();
    await expect(assignmentAction).toBeFocused();
    await page.keyboard.press("Enter");
    await expect(page.locator(`article[data-skill-id="${skillId}"] button[aria-label="${actionName}"]`)).toHaveCount(0);

    const advanced = page.locator("details").filter({ hasText: "Agent mount paths" });
    await expect(advanced).not.toHaveAttribute("open", "");
    await advanced.locator("summary").click();
    await expect(advanced.locator("input")).toHaveValue(/\.codex[/\\]skills/);
  });

  test("supports inventory filters and focus-safe application dialogs", async ({ page }) => {
    await page.setViewportSize({ width: 760, height: 820 });
    await page.goto("/settings");
    await page.getByRole("button", { name: "Skills", exact: true }).click();
    await page.getByPlaceholder("Search Skills...").fill("readme");
    await expect(page.getByText(/items$/).first()).toBeVisible();
    await page.getByPlaceholder("Search Skills...").fill("");

    const localSearch = page.getByPlaceholder("Search by id, name, category, trigger, or source");
    await localSearch.fill("readme");
    await expect(page.getByText("1 items")).toBeVisible();
    await localSearch.fill("");

    const codexAgent = page.getByRole("button", { name: /Codex CLI/ });
    await codexAgent.locator("span.truncate").evaluate((label) => {
      label.textContent = "OpenAI Codex CLI with an exceptionally long registered Agent display name";
    });
    await expect(codexAgent.locator("span.truncate")).toHaveCSS("text-overflow", "ellipsis");
    await codexAgent.click();
    const board = page.getByTestId("skill-selection-board");
    const assigned = board.locator('[data-skill-group="assigned"]');
    const available = board.locator('[data-skill-group="available"]');
    const assignedBox = await assigned.boundingBox();
    const availableBox = await available.boundingBox();
    expect(assignedBox).not.toBeNull();
    expect(availableBox).not.toBeNull();
    expect(availableBox!.y).toBeGreaterThan(assignedBox!.y + assignedBox!.height - 2);
    expect(Math.abs(availableBox!.x - assignedBox!.x)).toBeLessThan(8);

    await page.getByRole("button", { name: /^All Skills/ }).click();
    await page.setViewportSize({ width: 375, height: 780 });
    const narrowDetailsButton = page.locator("article[data-skill-id]").first().getByRole("button", { name: /View details for/ });
    await narrowDetailsButton.click();
    const detailsDialog = page.getByRole("dialog", { name: /details/ });
    await expect(detailsDialog).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(detailsDialog).toBeHidden();
    await expect(narrowDetailsButton).toBeFocused();
    const hasHorizontalOverflow = await page.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
    expect(hasHorizontalOverflow).toBe(false);

    const restoreButton = page.getByRole("button", { name: /Restore Built-in/ });
    await restoreButton.click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("select")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
    await expect(restoreButton).toBeFocused();
  });

  test("exposes Effective, Global, and Project Skill views in session information", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: "New", exact: true }).click();
    const project = page.getByPlaceholder(/code.*project/);
    await project.fill("D:\\example-workspace");
    await project.press("Tab");
    const create = page.getByRole("button", { name: "Create", exact: true });
    await expect(create).toBeEnabled();
    await create.click();
    await page.getByRole("button", { name: "Skill", exact: true }).click();
    await expect(page.getByRole("tab", { name: "Effective" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Global" })).toBeVisible();
    await page.getByRole("tab", { name: "Project" }).click();
    await expect(page.getByText("Project Skill context")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Skill" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Import Skill" })).toBeVisible();
  });
});
