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

    await page.getByRole("button", { name: /Restore Built-in/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.locator("select")).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
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
    await page.getByRole("tab", { name: "Skill", exact: true }).click();
    await expect(page.getByRole("tab", { name: "Effective" })).toBeVisible();
    await expect(page.getByRole("tab", { name: "Global" })).toBeVisible();
    await page.getByRole("tab", { name: "Project" }).click();
    await expect(page.getByText("Project Skill context")).toBeVisible();
    await expect(page.getByRole("button", { name: "Create Skill" })).toBeVisible();
    await expect(page.getByRole("button", { name: "Import Skill" })).toBeVisible();
  });
});
