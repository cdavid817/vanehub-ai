import { expect, test } from "@playwright/test";

async function openAbout(page: import("@playwright/test").Page, theme: "futuristic" | "minimal", width: number) {
  await page.setViewportSize({ width, height: 820 });
  await page.addInitScript((selectedTheme) => localStorage.setItem("vanehub.appSettings", JSON.stringify({ theme: selectedTheme })), theme);
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings|設定|설정/ }).click();
  await page.getByRole("navigation", { name: /系统设置|Settings/ }).getByRole("button", { name: /关于|About/, exact: true }).click();
}

for (const theme of ["futuristic", "minimal"] as const) {
  for (const width of [1440, 390]) {
    test(`${theme} update surface at ${width}px`, async ({ page }, testInfo) => {
      await openAbout(page, theme, width);
      await expect(page.getByRole("button", { name: /检查更新|Check Updates/ })).toBeVisible();
      await expect(page.getByText(/更新通道|Update channel/)).toBeVisible();
      await page.getByRole("button", { name: /检查更新|Check Updates/ }).click();
      await expect(page.getByRole("button", { name: /下载并安装|Download and install/ })).toBeVisible();
      await page.getByRole("button", { name: /下载并安装|Download and install/ }).click();
      await expect(page.getByRole("button", { name: /立即重启|Restart now/ })).toBeVisible();
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= document.documentElement.clientWidth)).toBe(true);
      await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${theme}-${width}-ready.png`) });
    });
  }
}
