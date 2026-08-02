import { expect, test, type Page } from "@playwright/test";

async function openAgentConfigurations(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.getByRole("button", { name: /^(Agent 管理|Agent Management)$/ }).click();
  const claudeCard = page.locator("section.ucd-interactive").filter({ has: page.getByRole("heading", { name: "Claude Code", exact: true }) });
  await claudeCard.getByRole("button", { name: /管理全局配置|Manage global configurations/ }).click();
  await expect(page.getByRole("heading", { name: /^(Agent 配置|Agent Configurations)$/, level: 2 })).toBeVisible();
  await expect(page.getByRole("tab", { name: "Claude Code" })).toHaveAttribute("aria-selected", "true");
  await expect(page.getByText(/Web 模式不会同步本地 CLI 配置|Web mode does not synchronize local CLI configuration/)).toBeVisible();
}

async function createAndApplyProfile(page: Page, agentName: string, presetName: string, profileName: string, credential?: string) {
  await page.getByRole("tab", { name: agentName, exact: true }).click();
  await expect(page.getByText(/Web 模拟|Web simulation/)).toBeVisible();
  await page.getByRole("button", { name: /新增配置|Add configuration/ }).click();

  const editor = page.getByRole("dialog");
  await editor.getByRole("button", { name: new RegExp(`^${presetName}`) }).click();
  await editor.getByRole("textbox", { name: /配置名称|Profile name/ }).fill(profileName);
  if (credential) await editor.getByLabel(/API Key 或 Token|API key or token/).fill(credential);
  await editor.getByRole("button", { name: /保存修改|Save changes/ }).click();

  const profile = page.locator("article").filter({ hasText: profileName });
  await expect(profile).toBeVisible();
  await profile.getByRole("button", { name: /全局应用|Apply globally/ }).click();
  const confirmation = page.getByRole("dialog");
  await confirmation.getByRole("button", { name: /全局应用|Apply globally/ }).click();
  await expect(page.getByText(/Web 模式已模拟全局切换|Web mode simulated the global switch/)).toBeVisible();
  await expect(profile.getByText(/当前已应用|Currently applied/)).toBeVisible();
}

test.describe("Agent global CLI configuration", () => {
  test("navigates from Agent management and applies independent profiles", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 1000 });
    await openAgentConfigurations(page);

    await createAndApplyProfile(page, "Claude Code", "Anthropic", "Claude Official");
    await createAndApplyProfile(page, "OpenCode", "OpenAI", "OpenCode OpenAI", "not-persisted-secret");
    await createAndApplyProfile(page, "Codex CLI", "OpenAI", "Codex Official");
  });

  test("filters providers and keeps dialogs usable at narrow width", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openAgentConfigurations(page);
    await expect(page.getByText(/Web 模式不会同步本地 CLI 配置|Web mode does not synchronize local CLI configuration/)).toBeVisible();
    await expect(page.getByText("DeepSeek · Claude Code")).toHaveCount(0);
    await page.getByRole("button", { name: /新增配置|Add configuration/ }).click();

    const dialog = page.getByRole("dialog");
    await dialog.getByRole("textbox", { name: /搜索厂商预设|Search provider presets/ }).fill("DeepSeek");
    await expect(dialog.getByText("DeepSeek · Claude Code")).toBeVisible();
    await expect(dialog.getByText("Anthropic · Claude Code")).toHaveCount(0);
    await dialog.getByRole("button", { name: /^DeepSeek/ }).click();
    await expect(dialog.getByRole("button", { name: /保存修改|Save changes/ })).toBeVisible();
    const bodyOverflows = await page.evaluate(() => document.body.scrollWidth > window.innerWidth);
    expect(bodyOverflows).toBe(false);
  });
});
