import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

async function openReview(page: Parameters<typeof createSession>[0], theme: "futuristic" | "minimal", width: number) {
  await page.setViewportSize({ width, height: width < 600 ? 844 : 900 });
  await page.addInitScript((selectedTheme) => {
    window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme: selectedTheme }));
  }, theme);
  await page.goto("/");
  await createSession(page, `Review ${theme} ${width}`);
  await page.getByRole("tab", { name: "变更" }).click();
  await expect(page.getByTestId("review-center")).toBeVisible();
}

test("reviews three files, adds inline feedback, and reports simulated revert", async ({ page }) => {
  await openReview(page, "futuristic", 1440);
  await expect(page.getByText("审查文件（3）")).toBeVisible();
  await page.getByRole("button", { name: /export const runtime = "web-mock"/ }).click();
  await page.getByLabel("审查评论").fill("Please add a regression test.");
  await page.getByRole("button", { name: "添加评论" }).click();
  await page.getByRole("button", { name: "回退文件" }).click();
  await page.getByRole("button", { name: "确认" }).click();
  await expect(page.getByText("已完成模拟回退，未修改本地文件。")).toBeVisible();
  await page.getByRole("button", { name: "发送反馈" }).click();
});

for (const variant of [
  { theme: "futuristic" as const, width: 1440 },
  { theme: "minimal" as const, width: 1440 },
  { theme: "futuristic" as const, width: 390 },
  { theme: "minimal" as const, width: 390 },
]) {
  test(`review center visual ${variant.theme} ${variant.width}`, async ({ page }, testInfo) => {
    await openReview(page, variant.theme, variant.width);
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
    await expect(page.getByRole("button", { name: "接受", exact: true })).toBeVisible();
    await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${variant.theme}-${variant.width}.png`) });
  });
}
