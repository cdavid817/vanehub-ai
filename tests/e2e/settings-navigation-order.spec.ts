import { expect, test } from "@playwright/test";

test("orders Settings destinations around common setup and customization workflows", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();

  const labels = await page.locator("nav button").allTextContents();
  const expected = [
    "基础配置",
    "Agent 配置",
    "Agent 权限策略",
    "CLI 参数",
    "MCP 服务器",
    "Skill 管理",
    "个性化",
    "Prompt Hook",
    "专家角色",
    "CLI 管理",
    "扩展能力",
    "插件集成",
    "IM 能力",
    "SSH 连接",
    "执行可观测性",
    "使用统计",
    "关于",
  ];
  const positions = expected.map((label) => labels.findIndex((entry) => entry.trim() === label));
  expect(positions.every((position) => position >= 0)).toBe(true);
  expect(positions).toEqual([...positions].sort((left, right) => left - right));
});
