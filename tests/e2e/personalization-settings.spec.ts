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

    // The list is its own panel, paged and filtered, and carries names rather than bodies.
    await expect(page.getByRole("heading", { name: "Saved memories", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByTestId("personalization-memory-filters")).toBeVisible();
    await expect(page.getByTestId("personalization-memory-list")).toBeVisible();
    await expect(page.getByText("prefers-metric-units")).toBeVisible();
    await expect(page.getByText("The user prefers metric units and 24-hour time.")).toHaveCount(0);
  });

  test("filters the memory list without reloading the page", async ({ page }) => {
    await openView(page, "memory");
    await expect(page.getByText("vanehub-uses-npm")).toBeVisible();

    await page.getByTestId("personalization-memory-type").selectOption("user");

    await expect(page.getByText("prefers-metric-units")).toBeVisible();
    await expect(page.getByText("vanehub-uses-npm")).toHaveCount(0);
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
  for (const viewport of [
    { name: "900px", width: 900, height: 720 },
    { name: "640px", width: 640, height: 720 },
  ]) {
    test(`keeps the instruction workflow usable at ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await openPersonalization(page);
      await openView(page, "instructions");

      // The controls stack rather than overflow. A field the user cannot reach without a
      // horizontal scrollbar is a field they will not fill in.
      for (const testId of [
        "personalization-scope-kind",
        "personalization-merge-mode",
        "personalization-field-aboutUser",
        "personalization-field-styleRules",
        "personalization-save",
      ]) {
        const box = await page.getByTestId(testId).boundingBox();
        expect(box, testId).not.toBeNull();
        expect(box!.x + box!.width, testId).toBeLessThanOrEqual(viewport.width);
      }

      await expect(page.locator("body")).toHaveJSProperty("scrollWidth", viewport.width);
    });
  }

  test("moves between destinations with the arrow keys", async ({ page }) => {
    await page.getByTestId("personalization-view-tab-overview").focus();
    await page.keyboard.press("ArrowRight");

    await expect(page.getByTestId("personalization-view-tab-instructions")).toHaveAttribute(
      "aria-selected",
      "true",
    );
    // Selection follows focus, so a second arrow press has to continue from the new tab rather
    // than from wherever the keyboard was left behind.
    await page.keyboard.press("ArrowRight");
    await expect(page.getByTestId("personalization-view-tab-memory")).toHaveAttribute(
      "aria-selected",
      "true",
    );
  });

  test("saves with the keyboard alone", async ({ page }) => {
    await openView(page, "instructions");
    const styleField = page.getByTestId("personalization-field-styleRules");
    await styleField.fill("Saved from the keyboard.");

    await page.keyboard.press("ControlOrMeta+s");

    await expect(page.getByTestId("personalization-dirty")).toBeHidden();
    await page.reload();
    await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
    await openView(page, "instructions");
    await expect(page.getByTestId("personalization-field-styleRules")).toHaveValue(
      "Saved from the keyboard.",
    );
  });
});
