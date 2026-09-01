import { expect, test } from "@playwright/test";

test.describe("Extension Capabilities settings page", () => {
  test("renders localized Web mock capabilities and installation preview", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /扩展能力|Extension Capabilities/ }).click();

    await expect(page.getByText(/Tauri 桌面端|Tauri desktop runtime/).first()).toBeVisible();
    await expect(page.getByRole("heading", { name: "PaddleOCR" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "faster-whisper" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "sherpa-onnx" })).toBeVisible();
    // Pre-existing, unrelated to task 12.18: commit ebfde179 ("add cross-page search UI") gave
    // this input role="combobox" (aria-autocomplete listbox), but this spec was never updated off
    // its old role="textbox" locator.
    const search = page.getByRole("combobox", { name: /搜索扩展能力|Search extensions/ });
    await search.fill("sherpa");
    await expect(page.getByRole("heading", { name: "sherpa-onnx" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "PaddleOCR" })).toBeHidden();
    await search.clear();
    // Task 12.18: Requirements moved from a directly-visible button into the card's own
    // ActionMenu -- open the paddleocr card's "..." trigger before reaching the menu item. Named
    // explicitly (not a bare role locator) because task 12.19 added a second, always-visible
    // button to this card (Copy Diagnostics), so "the card's only button" is no longer true.
    await page.getByTestId("extension-card-paddleocr").getByRole("button", { name: /PaddleOCR的操作|Actions for PaddleOCR/ }).click();
    await page.getByRole("menuitem", { name: /安装要求|Requirements/ }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(dialog).toBeHidden();
  });

  test("shows a non-transparent, semantically-toned status badge for each framework", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /扩展能力|Extension Capabilities/ }).click();

    const card = page.getByTestId("extension-card-paddleocr");
    const badge = card.getByText(/当前环境不支持|not supported/);
    await expect(badge).toBeVisible();
    await expect(badge).not.toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
  });

  test("preserves the page while switching both registered themes", async ({ page }) => {
    await page.goto("/", { waitUntil: "domcontentloaded" });
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /基础配置|Basic Settings/ }).click();
    await page.getByLabel(/主题|Theme/).selectOption("minimal");
    await page.getByRole("button", { name: /扩展能力|Extension Capabilities/ }).click();

    await expect(page.getByTestId("extension-card-paddleocr")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");

    await page.getByRole("button", { name: /基础配置|Basic Settings/ }).click();
    await page.getByLabel(/主题|Theme/).selectOption("futuristic");
    await page.getByRole("button", { name: /扩展能力|Extension Capabilities/ }).click();
    await expect(page.getByTestId("extension-card-paddleocr")).toBeVisible();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");
  });
});
