import { expect, test, type Page } from "@playwright/test";

async function openHybridRuntime(page: Page, theme: "futuristic" | "minimal", width: number) {
  await page.setViewportSize({ width, height: width < 600 ? 1_500 : 900 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "en", theme: selectedTheme }));
  }, theme);
  await page.goto("/");
  await page.getByRole("button", { name: /Settings/ }).click();
  await page.getByRole("button", { name: "Agent Configurations" }).click();
  await page.getByRole("tab", { name: "OnePiece" }).click();
  const section = page.getByRole("region", { name: "Hybrid local model runtime" });
  await expect(section).toBeVisible();
  return section;
}

test("discovers, verifies, saves and previews a local-only route without network access", async ({ page }) => {
  const section = await openHybridRuntime(page, "futuristic", 1440);
  await section.getByRole("button", { name: "Discover localhost" }).click();
  await expect(section.getByLabel("Model ID")).toHaveValue("web-simulated-local-model");
  await section.getByRole("button", { name: "Verify metadata" }).click();
  await section.getByLabel("Tool calling").selectOption("unsupported");
  await section.getByRole("button", { name: "Save profile" }).click();
  await expect(section.getByLabel("Preferred profile")).toContainText("web-simulated-local-model");
  await section.getByLabel("Preferred profile").selectOption({ label: "web-simulated-local-model" });
  await section.getByLabel("Data policy").selectOption("local-only");
  await section.getByRole("button", { name: "Save rule" }).click();
  await section.getByRole("button", { name: "Preview route" }).click();
  await expect(section.getByText("rule-preferred", { exact: true })).toBeVisible();
});

for (const variant of [
  { theme: "futuristic" as const, width: 1440 },
  { theme: "minimal" as const, width: 1440 },
  { theme: "futuristic" as const, width: 390 },
  { theme: "minimal" as const, width: 390 },
]) {
  test(`hybrid runtime visual ${variant.theme} ${variant.width}`, async ({ page }, testInfo) => {
    const section = await openHybridRuntime(page, variant.theme, variant.width);
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await expect(section.getByLabel("Reasoning field")).toBeVisible();
    await section.screenshot({ path: testInfo.outputPath(`${variant.theme}-${variant.width}.png`) });
  });
}
