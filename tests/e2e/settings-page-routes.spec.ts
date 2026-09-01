import { expect, test } from "@playwright/test";

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Task 12.20's "all page routes" gap: `settings-navigation-order.spec.ts` proves all 20 nav
 * buttons are attached and correctly ordered, but never clicks through any of them to prove each
 * one actually renders its own distinct content. This sweeps all 20, in the same order as that
 * file's own `expected` array (the product order defined in `settings-pages.ts`), clicking each
 * nav entry and asserting a page-specific heading becomes visible before moving to the next.
 *
 * `level` defaults to 2 because 17 of the 20 pages render their title through the local
 * `page-parts.tsx` `PageHeader`, an `<h2>`. Three pages -- `extensions`, `plugins`, and
 * `ssh-connections` -- instead import the newer shared `ui/page-header/PageHeader`
 * (`src/ui/page-header/PageHeader.tsx`, task 12.18's own primitive), which renders an `<h1>`, not
 * an `<h2>`. Confirmed by reading each of those 3 pages' own import line and both `PageHeader`
 * components' source directly, not assumed. Their title text still starts with this page's own
 * nav label the same as every other page, only the heading level differs -- so `level: 1` here is
 * the "actual page-specific marker" the task calls for, not a forced workaround.
 *
 * Prefix match, not exact, for two independent reasons already established elsewhere this session:
 * a nav entry's own accessible name can carry a folded-in status-dot description
 * (`settings-navigation-order.spec.ts`), and a couple of pages' own heading strings carry extra
 * trailing text beyond the nav label itself -- About's `<h2>` reads "关于 VaneHub AI" (`about.title`)
 * and CLI Parameters' reads "CLI 参数管理" (`cliParameters.title`), both confirmed directly against
 * `zh-CN.json` rather than guessed.
 */
const pages: ReadonlyArray<{ label: string; level: 1 | 2 }> = [
  { label: "基础配置", level: 2 },
  { label: "Agent 配置", level: 2 },
  { label: "Agent 权限策略", level: 2 },
  { label: "CLI 参数", level: 2 },
  { label: "代码智能", level: 2 },
  { label: "MCP 服务器", level: 2 },
  { label: "Skill 管理", level: 2 },
  { label: "AI 个性化", level: 2 },
  { label: "Prompt Hook", level: 2 },
  { label: "专家角色", level: 2 },
  { label: "本地媒体", level: 2 },
  { label: "CLI 管理", level: 2 },
  { label: "扩展能力", level: 1 },
  { label: "插件集成", level: 1 },
  { label: "IM 能力", level: 2 },
  { label: "SSH 连接", level: 1 },
  { label: "执行可观测性", level: 2 },
  { label: "使用统计", level: 2 },
  { label: "使用文档", level: 2 },
  { label: "关于", level: 2 },
];

test.describe("settings page routes (task 12.20)", () => {
  test("every registered settings page renders its own distinct heading when opened from nav", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();

    const navigation = page.locator("nav");
    for (const { label, level } of pages) {
      const namePrefix = new RegExp(`^${escapeRegExp(label)}`);
      await navigation.getByRole("button", { name: namePrefix }).click();
      await expect(page.getByRole("heading", { level, name: namePrefix })).toBeVisible();
    }
  });

  test("navigating to another page unmounts the previous page's own heading", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();

    // "basic" is `defaultSettingsPageId` (settings-pages.ts) -- already the active page on first
    // entry, no click needed to arrive here.
    const basicHeading = page.getByRole("heading", { level: 2, name: /^基础配置/ });
    await expect(basicHeading).toBeVisible();

    await page.locator("nav").getByRole("button", { name: /^关于/ }).click();
    await expect(page.getByRole("heading", { level: 2, name: /^关于/ })).toBeVisible();
    // Task 12.17 established most pages unmount when not the active page (draft-only pages being
    // the deliberate exception) -- this is a wholesale-regression sanity check on one arbitrary
    // pair, not exhaustive per-page lifecycle coverage.
    await expect(basicHeading).toHaveCount(0);
  });
});
