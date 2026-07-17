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
    const search = page.getByRole("textbox", { name: /搜索扩展能力|Search extensions/ });
    await search.fill("sherpa");
    await expect(page.getByRole("heading", { name: "sherpa-onnx" })).toBeVisible();
    await expect(page.getByRole("heading", { name: "PaddleOCR" })).toBeHidden();
    await search.clear();
    await page.getByRole("button", { name: /安装要求|Requirements/ }).first().click();
    await expect(page.getByRole("dialog")).toBeVisible();
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
