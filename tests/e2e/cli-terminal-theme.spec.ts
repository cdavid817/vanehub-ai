import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

// The values declared in `src/styles.css` for the two CLI terminal palettes, as the browser
// reports them. Read from the live xterm DOM, not from a mock, so this proves what the user sees.
const darkBackground = "rgb(13, 17, 23)";
const lightBackground = "rgb(255, 255, 255)";
const marker = "theme probe marker";
const ansiSample = "[31mansi-red[0m [38;2;255;0;128mtruecolor[0m [7minverse[0m [33myellow[0m [34mblue[0m";

// xterm paints the terminal background onto its scrollable element's inline style whenever the
// theme changes, so the first descendant carrying an inline background is the real canvas color.
async function readTerminalColors(page: Page) {
  return page.evaluate(() => {
    const host = document.querySelector<HTMLElement>(".ucd-agent-terminal");
    if (!host) throw new Error("agent terminal host missing");
    const painted = [...host.querySelectorAll<HTMLElement>("*")].find((element) => element.style.backgroundColor);
    return {
      hostBackground: getComputedStyle(host).backgroundColor,
      xtermBackground: painted ? getComputedStyle(painted).backgroundColor : null,
      xtermCount: host.querySelectorAll(".xterm").length,
      xtermProbe: host.querySelector<HTMLElement>(".xterm")?.dataset.probe ?? null,
      // A CLI-emitted truecolor cell keeps its own color under either palette.
      truecolorCell: [...host.querySelectorAll<HTMLElement>(".xterm-rows span")].some((span) => getComputedStyle(span).color === "rgb(255, 0, 128)"),
      appTheme: document.documentElement.dataset.theme ?? null,
      cliTheme: document.documentElement.dataset.cliTerminalTheme ?? null,
    };
  });
}

async function openBasicSettings(page: Page) {
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await expect(page.getByRole("heading", { name: "基础配置" })).toBeVisible();
}

async function returnToWorkspace(page: Page) {
  await page.getByRole("button", { name: "返回", exact: true }).click();
  await expect(page.getByLabel("Agent CLI 工作区")).toBeVisible();
}

test.describe("CLI session terminal theme", () => {
  test("defaults to dark, switches to light in place, and keeps the app theme and the terminal intact", async ({ page }, testInfo) => {
    // Seeds once: the init script also runs on reload, and overwriting the store there would
    // erase exactly the persistence the reload step is meant to prove.
    await page.addInitScript(() => {
      if (window.localStorage.getItem("vanehub.appSettings")) return;
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme: "minimal" }));
      window.localStorage.setItem("vanehub.uiStyle", "minimal");
    });
    await page.goto("/");
    await createSession(page, "CLI 主题会话");

    const terminal = page.getByLabel("Agent CLI 工作区");
    // The Web mock echoes input verbatim (ending in a bare `\r`, so consecutive sends overwrite one
    // row), which makes one line carrying the marker plus ANSI 16-color, truecolor, and inverse
    // sequences render through the real xterm exactly as a CLI would emit it. The palette test is
    // that those sequences survive both themes untouched and stay readable.
    await page.getByRole("textbox", { name: "工作区命令输入" }).fill(`${marker} ${ansiSample}`);
    await page.getByRole("button", { name: "发送命令" }).click();
    await expect(terminal).toContainText(marker);
    await expect(terminal).toContainText("ansi-red truecolor inverse");
    await expect(terminal.locator(".xterm-fg-1").first()).toBeVisible();
    await expect.poll(async () => (await readTerminalColors(page)).truecolorCell).toBe(true);

    // Tag the live xterm root so a recreated terminal would be caught later.
    await page.evaluate(() => {
      const root = document.querySelector<HTMLElement>(".ucd-agent-terminal .xterm");
      if (root) root.dataset.probe = "original";
    });
    const dark = await readTerminalColors(page);
    expect(dark.cliTheme).toBe("dark");
    expect(dark.appTheme).toBe("minimal");
    expect(dark.hostBackground).toBe(darkBackground);
    expect(dark.xtermBackground).toBe(darkBackground);
    expect(dark.xtermCount).toBe(1);
    await terminal.screenshot({ path: testInfo.outputPath("cli-terminal-dark.png") });

    // The settings page is its own route and unmounts the workspace, so the in-place proof has to
    // change the setting while the terminal stays mounted. Saving through the Web settings adapter
    // is the same path a settings event from another surface takes: the provider re-reads, applies
    // the root attribute, and the mounted terminal repaints. No React tree is touched.
    await page.evaluate(async () => {
      const { webSettingsClient } = await import("/src/services/web-settings-client.ts");
      await webSettingsClient.saveSetting({ key: "cliTerminalTheme", value: "light" });
    });
    await expect(page.locator("html")).toHaveAttribute("data-cli-terminal-theme", "light");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    await expect(terminal).toContainText(marker);
    await expect(terminal).toContainText("ansi-red truecolor inverse");
    await expect.poll(async () => (await readTerminalColors(page)).xtermBackground).toBe(lightBackground);
    // CLI-owned truecolor is rendered as emitted under the light palette too.
    expect((await readTerminalColors(page)).truecolorCell).toBe(true);
    const light = await readTerminalColors(page);
    expect(light.cliTheme).toBe("light");
    expect(light.appTheme).toBe("minimal");
    expect(light.hostBackground).toBe(lightBackground);
    expect(light.xtermCount).toBe(1);
    // The same xterm root as before: repainted, not recreated.
    expect(light.xtermProbe).toBe("original");
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toBeEnabled();
    await expect(page.getByRole("button", { name: "停止", exact: true })).toBeEnabled();
    await terminal.screenshot({ path: testInfo.outputPath("cli-terminal-light.png") });

    // The settings page shows the persisted choice, and the UI control drives the reverse switch.
    await openBasicSettings(page);
    const select = page.getByRole("combobox", { name: "CLI 会话主题" });
    await expect(select).toHaveValue("light");
    await expect(page.getByRole("combobox", { name: "主题", exact: true })).toHaveValue("minimal");
    await expect(page.getByText("部分 CLI 使用自身配色")).toBeVisible();
    await select.selectOption("dark");
    await expect(page.locator("html")).toHaveAttribute("data-cli-terminal-theme", "dark");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    await select.selectOption("light");
    await expect(page.locator("html")).toHaveAttribute("data-cli-terminal-theme", "light");

    // Survives a reload through the Web adapter's persistence, and the app theme is untouched.
    // A terminal created fresh under a persisted light setting is covered by the next test.
    await page.reload();
    await expect(page.locator("html")).toHaveAttribute("data-cli-terminal-theme", "light");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
  });

  test("changing the application theme does not change the CLI terminal palette", async ({ page }) => {
    await page.addInitScript(() => {
      if (window.localStorage.getItem("vanehub.appSettings")) return;
      window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme: "minimal", cliTerminalTheme: "light" }));
      window.localStorage.setItem("vanehub.uiStyle", "minimal");
    });
    await page.goto("/");
    await createSession(page, "应用主题独立性");
    const terminal = page.getByLabel("Agent CLI 工作区");
    await expect(terminal).toBeVisible();
    expect((await readTerminalColors(page)).xtermBackground).toBe(lightBackground);

    await openBasicSettings(page);
    await page.getByRole("combobox", { name: "主题", exact: true }).selectOption("futuristic");
    await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");
    await expect(page.getByRole("combobox", { name: "CLI 会话主题" })).toHaveValue("light");

    await returnToWorkspace(page);
    const after = await readTerminalColors(page);
    expect(after.appTheme).toBe("futuristic");
    expect(after.cliTheme).toBe("light");
    expect(after.xtermBackground).toBe(lightBackground);
  });

  test("folds the composer away and gives the terminal the height", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "折叠输入框");
    const terminal = page.getByLabel("Agent CLI 工作区");
    const before = await terminal.boundingBox();
    await page.getByRole("button", { name: "收起输入框" }).click();
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toHaveCount(0);
    await expect(page.getByText("可直接在终端中输入")).toBeVisible();
    await expect.poll(async () => (await terminal.boundingBox())?.height ?? 0).toBeGreaterThan((before?.height ?? 0) + 40);

    await page.getByRole("button", { name: "展开输入框" }).click();
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toBeVisible();
  });
});
