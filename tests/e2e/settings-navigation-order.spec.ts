import { expect, test } from "@playwright/test";

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

test("orders Settings destinations around common setup and customization workflows", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();

  const expected = [
    "基础配置",
    "Agent 配置",
    "Agent 权限策略",
    "CLI 参数",
    "代码智能",
    "MCP 服务器",
    "Skill 管理",
    "AI 个性化",
    "Prompt Hook",
    "专家角色",
    "本地媒体",
    "CLI 管理",
    "扩展能力",
    "插件集成",
    "IM 能力",
    "SSH 连接",
    "执行可观测性",
    "使用统计",
    "使用文档",
    "关于",
  ];
  // Prefix match, not exact: a nav entry can carry a status dot (task 12.16) whose sr-only
  // description text folds into the button's own accessible name and text content (e.g. "基础配置
  // Node.js 环境不可用"), and whether that has loaded by the time either assertion below runs is a
  // race, not a fixed state -- a plain page load in Web preview eventually reports Node.js as
  // unavailable, but not necessarily before this test's own checks run.
  const navigation = page.locator("nav");
  for (const label of expected) {
    await expect(navigation.getByRole("button", { name: new RegExp(`^${escapeRegExp(label)}`) })).toBeAttached();
  }
  const labels = await navigation.getByRole("button").allTextContents();
  const positions = expected.map((label) => labels.findIndex((entry) => entry.trim().startsWith(label)));
  expect(positions.every((position) => position >= 0)).toBe(true);
  expect(positions).toEqual([...positions].sort((left, right) => left - right));
});
