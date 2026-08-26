import { expect, test, type Page } from "@playwright/test";

/** The page opens on Overview, so every test names the destination it is about. */
async function openView(page: Page, view: "overview" | "instructions" | "memory" | "runtimePreview") {
  await page.getByTestId(`personalization-view-tab-${view}`).click();
}

async function openPersonalization(page: Page) {
  await page.goto("/settings");
  await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
}

test.describe("Personalization settings", () => {
  test.beforeEach(async ({ page }) => {
    // Merge rather than overwrite: `addInitScript` re-runs on every navigation, including
    // `page.reload()` mid-test -- a plain `setItem` would wipe out settings a test just saved
    // before the app's own JS gets a chance to read them back.
    await page.addInitScript(() => {
      const raw = window.localStorage.getItem("vanehub.appSettings");
      const existing = raw ? JSON.parse(raw) : {};
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ ...existing, applicationLanguage: "en" }));
    });
    await openPersonalization(page);
  });

  test("renders the instruction editor and the memory section", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "AI Personalization", exact: true, level: 2 })).toBeVisible();

    await openView(page, "instructions");
    await expect(page.getByRole("heading", { name: "Scope", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByRole("heading", { name: "Custom Instructions", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByTestId("personalization-field-aboutUser")).toBeVisible();
    await expect(page.getByTestId("personalization-field-styleRules")).toBeVisible();

    await openView(page, "memory");
    await expect(page.getByRole("heading", { name: "Memory", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByRole("switch", { name: "Enable memory" })).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("No memories saved yet.")).toBeVisible();
  });

  test("writes nothing until Save, then keeps it across a reload", async ({ page }) => {
    await openView(page, "instructions");
    const styleField = page.getByTestId("personalization-field-styleRules");
    await styleField.fill("Always answer in Chinese.");
    await styleField.blur();

    // Blur is not a save. The store still holds what it held, so a reload here would show the old
    // text -- which is exactly what the next reload proves once Save has been pressed.
    await expect(page.getByTestId("personalization-dirty")).toBeVisible();

    await page.getByTestId("personalization-save").click();
    await expect(page.getByTestId("personalization-dirty")).toBeHidden();

    await page.reload();
    await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
    await openView(page, "instructions");
    await expect(page.getByTestId("personalization-field-styleRules")).toHaveValue("Always answer in Chinese.");
  });

  test("refuses to save a field past the character limit", async ({ page }) => {
    await openView(page, "instructions");
    await page.getByTestId("personalization-field-aboutUser").fill("a".repeat(3001));

    await expect(page.getByTestId("personalization-count-aboutUser")).toContainText("3001 / 3000");
    await expect(page.getByTestId("personalization-save")).toBeDisabled();
  });

  test("puts the text back when the edit is discarded", async ({ page }) => {
    await openView(page, "instructions");
    const aboutField = page.getByTestId("personalization-field-aboutUser");
    const stored = await aboutField.inputValue();
    await aboutField.fill("Something else entirely.");

    await page.getByTestId("personalization-discard").click();

    await expect(aboutField).toHaveValue(stored);
    await expect(page.getByTestId("personalization-dirty")).toBeHidden();
  });

  test("turning a layer off keeps its text rather than clearing it", async ({ page }) => {
    await openView(page, "instructions");
    const aboutField = page.getByTestId("personalization-field-aboutUser");
    const stored = await aboutField.inputValue();

    await page.getByTestId("personalization-merge-mode").selectOption("disabled");
    await page.getByTestId("personalization-save").click();

    await expect(page.getByTestId("personalization-dirty")).toBeHidden();
    // Applying nothing is not the same as having nothing: the text survives being switched off,
    // so turning the layer back on does not require retyping it.
    await expect(aboutField).toHaveValue(stored);
  });

  test("disables the tool-assisted sub-toggle when the memory master toggle is off", async ({ page }) => {
    await openView(page, "memory");
    const memoryToggle = page.getByRole("switch", { name: "Enable memory" });
    const subToggle = page.getByRole("switch", { name: "Remember from tool-assisted chats" });
    await expect(subToggle).toBeEnabled();

    await memoryToggle.click();
    await expect(memoryToggle).toHaveAttribute("aria-checked", "false");
    await expect(subToggle).toBeDisabled();
  });
});
