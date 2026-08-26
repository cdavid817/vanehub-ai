import { expect, test, type Page } from "@playwright/test";

/** `onCommit` in `custom-instructions-section.tsx` fires a fire-and-forget `saveSetting` on
 * blur — reloading immediately after `blur()` races the localStorage write, so wait for the
 * persisted value directly rather than guessing at a UI-visible confirmation signal. */
async function waitForPersistedSetting(page: Page, needle: string) {
  await page.waitForFunction(
    (value) => window.localStorage.getItem("vanehub.appSettings")?.includes(value) ?? false,
    needle,
  );
}

/** The page opens on Overview, so every test names the destination it is about. */
async function openView(page: Page, view: "overview" | "instructions" | "memory" | "runtimePreview") {
  await page.getByTestId(`personalization-view-tab-${view}`).click();
}

test.describe("Personalization settings", () => {
  test.beforeEach(async ({ page }) => {
    // Merge rather than overwrite: `addInitScript` re-runs on every navigation, including
    // `page.reload()` mid-test — a plain `setItem` would wipe out settings a test just saved
    // before the app's own JS gets a chance to read them back.
    await page.addInitScript(() => {
      const raw = window.localStorage.getItem("vanehub.appSettings");
      const existing = raw ? JSON.parse(raw) : {};
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ ...existing, applicationLanguage: "en" }));
    });
    await page.goto("/settings");
    await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
  });

  test("renders the custom instructions and memory sections", async ({ page }) => {
    await expect(page.getByRole("heading", { name: "AI Personalization", exact: true, level: 2 })).toBeVisible();

    await openView(page, "instructions");
    await expect(page.getByRole("heading", { name: "Custom Instructions", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "Response style" })).toBeVisible();
    await expect(page.getByRole("textbox", { name: "About you" })).toBeVisible();
    await expect(page.getByRole("switch", { name: "Enable custom instructions" })).toHaveAttribute("aria-checked", "true");

    await openView(page, "memory");
    await expect(page.getByRole("heading", { name: "Memory", exact: true, level: 3 })).toBeVisible();
    await expect(page.getByRole("switch", { name: "Enable memory" })).toHaveAttribute("aria-checked", "true");
    await expect(page.getByRole("switch", { name: "Remember from tool-assisted chats" })).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("No memories saved yet.")).toBeVisible();
  });

  test("saves custom instructions on blur, persists across reload, and rejects an oversized value", async ({ page }) => {
    await openView(page, "instructions");
    const styleField = page.getByRole("textbox", { name: "Response style" });
    await styleField.fill("Always answer in Chinese.");
    await styleField.blur();
    await expect(page.getByText("25 / 3000")).toBeVisible();
    await waitForPersistedSetting(page, "Always answer in Chinese.");

    await page.reload();
    await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
    await openView(page, "instructions");
    await expect(page.getByRole("textbox", { name: "Response style" })).toHaveValue("Always answer in Chinese.");

    const overLimitField = page.getByRole("textbox", { name: "About you" });
    await overLimitField.fill("a".repeat(3001));
    await expect(page.getByText("3001 / 3000")).toBeVisible();
    await overLimitField.blur();

    await page.reload();
    await page.getByRole("button", { name: "AI Personalization", exact: true }).click();
    await openView(page, "instructions");
    await expect(page.getByRole("textbox", { name: "About you" })).toHaveValue("");
  });

  test("disables both custom instructions fields when the enable toggle is off", async ({ page }) => {
    await openView(page, "instructions");
    const toggle = page.getByRole("switch", { name: "Enable custom instructions" });
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-checked", "false");
    await expect(page.getByRole("textbox", { name: "Response style" })).toBeDisabled();
    await expect(page.getByRole("textbox", { name: "About you" })).toBeDisabled();
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
