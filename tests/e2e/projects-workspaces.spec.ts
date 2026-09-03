import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

/**
 * Task 13.14: Playwright, accessibility, and route coverage for the Projects and Workspaces
 * destination (`src/projects/`) -- the "unit coverage exists... but no Playwright/accessibility/
 * privacy suite yet" gap this task's own tasks.md evidence already named. "Privacy" itself is
 * covered separately, at the unit level (`workspace-detail.privacy.test.tsx`), where asserting
 * "this substring never appears in rendered text" is precise; a full Playwright round trip
 * through SSH connection setup would only re-prove the same claim through a slower, more brittle
 * path. "Service contract" is 13.14's own already-satisfied piece (`workspace-aggregation.test.ts`).
 *
 * The Web mock's known-project/remote-workspace/SSH-connection state all start genuinely empty
 * (`web-known-workspace-client.ts`/`web-ssh-connection-client.ts`, confirmed by reading both) --
 * there is no seed data at all until a session is actually created against a path. Every test
 * below that needs a real row creates one first via `createSession`, the same helper every other
 * spec in this suite already uses, rather than assuming Projects ships with demo content.
 */

async function navigateToProjects(page: Page) {
  await page.getByRole("button", { name: "项目与工作区", exact: true }).click();
  await expect(page).toHaveURL(/\/workspace\/projects$/);
}

test.describe("projects and workspaces destination", () => {
  test("lists a real known project reached via the activity bar (route + content)", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "工作区来源会话");

    await navigateToProjects(page);

    await expect(page.getByRole("heading", { name: "项目与工作区" })).toBeVisible();
    const list = page.getByRole("list", { name: "工作区列表" });
    // `{ exact: true }`: the card's own displayPath paragraph ("D:\example-workspace") contains
    // "example-workspace" as a substring too -- a non-exact match resolves both and violates
    // strict mode.
    await expect(list.getByText("example-workspace", { exact: true })).toBeVisible();
  });

  test("opens directly from its URL, rendering the real empty state (route)", async ({ page }) => {
    // No session created first -- a fresh load has genuinely nothing known yet, which is exactly
    // what proves this renders from the route itself rather than from prior click-driven state
    // (mirrors workspace-routing.spec.ts's own "opens a destination directly from its URL").
    await page.goto("/workspace/projects");
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    await expect(page.getByRole("heading", { name: "项目与工作区" })).toBeVisible();
    await expect(page.getByText("暂无项目或工作区")).toBeVisible();
  });

  test("switches the workspace view with accessible tab semantics", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "工作区筛选会话");
    await navigateToProjects(page);

    const recentTab = page.getByRole("tab", { name: "最近" });
    const unavailableTab = page.getByRole("tab", { name: "不可用" });
    await expect(recentTab).toHaveAttribute("aria-selected", "true");
    await expect(unavailableTab).toHaveAttribute("aria-selected", "false");

    await unavailableTab.click();
    await expect(unavailableTab).toHaveAttribute("aria-selected", "true");
    await expect(recentTab).toHaveAttribute("aria-selected", "false");
    // The just-created local row is available, so the unavailable view is genuinely, correctly empty.
    await expect(page.getByText("没有不可用项")).toBeVisible();

    await page.getByRole("tab", { name: "全部" }).click();
    await expect(page.getByRole("list", { name: "工作区列表" }).getByText("example-workspace", { exact: true })).toBeVisible();
  });

  test("selecting a workspace shows its accessible detail panel and primary action", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "工作区详情会话");
    await navigateToProjects(page);

    await page.getByRole("list", { name: "工作区列表" }).getByText("example-workspace", { exact: true }).click();

    const detail = page.getByTestId("workspace-detail");
    await expect(detail).toBeVisible();
    await expect(detail.getByRole("heading", { name: "example-workspace" })).toBeVisible();
    // A session was just created against this workspace, so Continue (not New) is primary --
    // proves the detail panel is reading real session state, not a static placeholder.
    await expect(detail.getByRole("button", { name: "继续会话" })).toBeVisible();
  });

  test("13.12: restores the active view across a destination switch and back", async ({ page }) => {
    await page.goto("/");
    await createSession(page, "视图持久化会话");
    await navigateToProjects(page);

    await page.getByRole("tab", { name: "不可用" }).click();
    await expect(page.getByText("没有不可用项")).toBeVisible();

    // The Sessions activity-bar entry's own accessible name toggles between "折叠会话栏"/
    // "展开会话栏" (it doubles as the session-sidebar collapse/expand control, `workspace-
    // activity-bar.tsx`), never a fixed "会话" -- targeted by its stable `aria-controls`
    // attribute instead of guessing which of the two dynamic labels currently applies.
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await expect(page).toHaveURL(/\/workspace\/sessions/);

    await navigateToProjects(page);
    await expect(page.getByRole("tab", { name: "不可用" })).toHaveAttribute("aria-selected", "true");
    await expect(page.getByText("没有不可用项")).toBeVisible();
  });

  test("13.12: below md width, selecting replaces the list with detail, and Back restores the list with focus", async ({ page }) => {
    await page.setViewportSize({ width: 700, height: 900 });
    await page.goto("/");
    await createSession(page, "紧凑布局会话");
    await navigateToProjects(page);

    const list = page.getByRole("list", { name: "工作区列表" });
    await expect(list).toBeVisible();
    await expect(page.getByTestId("workspace-detail")).toHaveCount(0);

    await list.getByText("example-workspace", { exact: true }).click();
    await expect(page.getByTestId("workspace-detail")).toBeVisible();
    // Compact never renders both at once -- the list must be gone, not merely off-screen below a
    // long detail panel.
    await expect(list).toHaveCount(0);

    const backButton = page.getByRole("button", { name: "返回工作区列表" });
    await expect(backButton).toBeVisible();
    await backButton.click();

    await expect(list).toBeVisible();
    await expect(page.getByTestId("workspace-detail")).toHaveCount(0);
    // Back must not strand keyboard focus off-page.
    await expect(page.getByTestId("projects-scroll-region")).toBeFocused();
  });

  /**
   * 20.2/20.17: extends this file's own existing 13.12 compact-mode test above (width 700, no
   * theme dimension) with a real theme-paired pair at two of task 20.2's own named widths,
   * bracketing the same `md:` (767px, `projects.tsx`'s own `COMPACT_QUERY`) breakpoint that test
   * already proves functionally: 768 keeps list+detail side by side, 640 is the next named width
   * down and switches to the compact "detail replaces list, with Back" mode. This destination had
   * no theme-paired visual coverage at all before this pass.
   */
  for (const variant of [
    { compact: false, name: "futuristic-wide", theme: "futuristic" as const, width: 768 },
    { compact: false, name: "minimal-wide", theme: "minimal" as const, width: 768 },
    { compact: true, name: "futuristic-narrow", theme: "futuristic" as const, width: 640 },
    { compact: true, name: "minimal-narrow", theme: "minimal" as const, width: 640 },
  ]) {
    test(`Projects visual ${variant.name}`, async ({ page }, testInfo) => {
      await page.setViewportSize({ width: variant.width, height: 900 });
      await page.addInitScript((theme) => window.localStorage.setItem("vanehub.appSettings", JSON.stringify({ applicationLanguage: "zh-CN", theme })), variant.theme);
      await page.goto("/");
      await createSession(page, "响应式矩阵会话");
      await navigateToProjects(page);
      await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

      const list = page.getByRole("list", { name: "工作区列表" });
      await expect(list).toBeVisible();
      await list.getByText("example-workspace", { exact: true }).click();

      const detail = page.getByTestId("workspace-detail");
      await expect(detail).toBeVisible();
      const backButton = page.getByRole("button", { name: "返回工作区列表", exact: true });
      if (variant.compact) {
        await expect(list).toHaveCount(0);
        await expect(backButton).toBeVisible();
      } else {
        await expect(list).toBeVisible();
        await expect(backButton).toHaveCount(0);
      }
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
      await page.getByTestId("workspace-frame").screenshot({ path: testInfo.outputPath(`${variant.name}.png`) });
    });
  }
});
