import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

test.describe("frontend rendering performance", () => {
  test("windows a 500+ Prompt Hook inventory and preserves lazy page state", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.addInitScript(() => {
      const hooks = Object.fromEntries(Array.from({ length: 501 }, (_, index) => {
        const id = `user-windowing-${String(index).padStart(3, "0")}`;
        return [id, {
          id,
          name: `Hook ${index}`,
          description: `Virtualized Prompt Hook ${index}`,
          category: "static",
          stage: "session-init",
          order: 1_000 + index,
          version: 1,
          source: "user",
          enabled: true,
          disableable: true,
          cliBindings: ["codex-cli"],
          governance: {
            safetyTier: "editable",
            transparencyTier: "opt-in-view",
            governanceTier: "human-gated",
          },
          templateBody: `Prompt body ${index}`,
          createdAt: "2026-07-23T00:00:00.000Z",
          updatedAt: "2026-07-23T00:00:00.000Z",
        }];
      }));
      window.localStorage.setItem("vanehub.prompt-hooks.v1", JSON.stringify(hooks));
    });
    await page.goto("/settings");
    // Below `lg` the sidebar is hidden in favor of a searchable sheet (task 12.9): open it first.
    // No `applicationLanguage` override in this test, so the app's real zh-CN default applies --
    // the trigger's own label is localized even though "Prompt Hook" itself renders untranslated.
    await page.getByRole("button", { name: /^(切换设置页面|Switch settings page)/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();

    const list = page.getByTestId("prompt-hook-virtual-list");
    await expect(list).toBeVisible();
    await expect(list).toHaveAttribute("data-virtual-count", "515");
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(30);

    await list.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    const lastRow = list.getByRole("listitem").filter({ hasText: "Hook 500" });
    await expect(lastRow).toBeVisible();
    await lastRow.locator("summary").click();
    await lastRow.getByRole("button", { name: "预览 Hook 内容" }).click();
    await expect(page.getByText("Prompt body 500", { exact: true })).toBeVisible();
    await page.getByRole("button", { name: "关闭" }).click();

    const filter = page.getByPlaceholder("按 ID、名称、描述、分类或来源搜索");
    await filter.fill("Hook 500");
    // The compact-nav Sheet closes on every selection (task 12.9), so each further page switch
    // below `lg` needs its own trigger click, not just the first one.
    await page.getByRole("button", { name: /^(切换设置页面|Switch settings page)/ }).click();
    await page.getByRole("button", { name: "Agent 配置" }).click();
    await expect(page.getByRole("heading", { name: "Agent 配置", level: 2 })).toBeVisible();
    await page.getByRole("button", { name: /^(切换设置页面|Switch settings page)/ }).click();
    await page.getByRole("button", { name: "Prompt Hook" }).click();
    // Disclosed, not fixed here: this now reaches a real, separate, pre-existing issue -- per
    // `settings-page-lifecycle.ts`, `prompt-hooks` has no entry of its own and so defaults to
    // `keepAlive: "never"` (design.md Decision 6's default), meaning the page genuinely unmounts
    // on navigate-away regardless of how you got there -- contradicting this assertion's own
    // "preserves lazy page state" intent. Not caused by this file's reachability fix above (which
    // only gets the test far enough to reach this assertion for the first time); fixing it means
    // picking a lifecycle policy for a different page, not propagating an already-established
    // test fix, so it is left disclosed rather than guessed at here.
    await expect(filter).toHaveValue("Hook 500");
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });

  test("virtualizes Agent logs and locates a timestamp across bounded pages", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await createSession(page, "日志虚拟化测试");
    // Logs lives in the closed-by-default Runtime Panel (§8's split), not a top-level tab -- open
    // it first, the same staleness task 11.7 already found and fixed once in
    // session-workspace-console.visual.spec.ts.
    await page.getByRole("button", { name: "运行时面板" }).click();
    await page.getByRole("tab", { name: "日志" }).click();

    const list = page.getByTestId("session-log-virtual-list");
    await expect(list).toBeVisible();
    await expect(list).toHaveAttribute("data-virtual-count", "201");
    // Disclosed, not fixed here: this now reaches a real, separate, pre-existing issue --
    // `data-rendered-count` stays at the full 201 instead of windowing down, reproduced
    // consistently across 3 independent runs including a single-worker isolated rerun (rules out
    // a load flake). Not caused by opening the Runtime Panel above (that only gets the test far
    // enough to reach this assertion for the first time); the virtual list's own windowing logic
    // is unrelated to both this task's SplitPane and compact-nav fixes, so it is left disclosed
    // rather than guessed at here.
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(50);

    await page.getByRole("button", { name: "定位", exact: true }).click();
    await expect(page.getByText("请输入有效时间。")).toBeVisible();

    await list.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    await page.getByRole("button", { name: "加载更多" }).click();
    await expect(list).toHaveAttribute("data-virtual-count", "401");

    const localTimestamp = await page.evaluate(() => {
      const value = new Date("2026-07-17T00:30:00.000Z");
      const pad = (part: number) => String(part).padStart(2, "0");
      return `${value.getFullYear()}-${pad(value.getMonth() + 1)}-${pad(value.getDate())}T${pad(value.getHours())}:${pad(value.getMinutes())}`;
    });
    await page.getByLabel("日志时间").fill(localTimestamp);
    await page.getByRole("button", { name: "定位", exact: true }).click();

    const located = page.locator('[data-log-id="web-log-150-history"]');
    await expect(located).toBeVisible();
    await expect(located).toBeFocused();
    await expect(list).toHaveAttribute("data-virtual-count", "600");
    await expect.poll(async () => Number(await list.getAttribute("data-rendered-count"))).toBeLessThan(50);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
  });
});
