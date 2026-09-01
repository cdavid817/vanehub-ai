import { expect, test } from "@playwright/test";

test.describe("settings compact navigation (task 12.9, 12.20)", () => {
  test("below the lg breakpoint the full sidebar is gone from the accessibility tree; the compact nav trigger is the way in", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();

    // `settings-sidebar.tsx` renders `hidden ... lg:flex` -- below `lg` its `<aside>` (and the
    // `<nav aria-label="系统设置">` inside it) is `display:none`, which excludes it from the
    // accessibility tree entirely, not just visually. A role-based query correctly finds nothing,
    // the same distinction `agent-global-config.spec.ts` already documents (`getByText` would
    // still match the hidden DOM node; `getByRole` does not).
    await expect(page.getByRole("navigation", { name: "系统设置" })).toHaveCount(0);
    await expect(page.getByRole("button", { name: /^切换设置页面/ })).toBeVisible();
  });

  test("opening the trigger reveals a dialog listing all 20 settings pages across every group", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    const trigger = page.getByRole("button", { name: /^切换设置页面/ });

    await trigger.click();
    await expect(trigger).toHaveAttribute("aria-expanded", "true");

    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("heading", { name: "系统设置", level: 3 })).toBeVisible();

    // One entry from "general" (the active default page) and one from "integrations" -- proves
    // every group is listed, not just the active page's own, the same shape the component's own
    // unit test proves. `settingsPageNavEntries` in settings-pages.ts has exactly 20 entries and
    // the sheet renders no other button (no close button -- Sheet.tsx has none, only a backdrop
    // click), so the dialog's total button count is a direct proof every page is reachable here.
    await expect(dialog.getByRole("button", { name: /^基础配置/ })).toBeVisible();
    await expect(dialog.getByRole("button", { name: /^SSH 连接/ })).toBeVisible();
    await expect(dialog.getByRole("button")).toHaveCount(20);
  });

  test("a filter query matching nothing shows the real no-results state, not an empty or broken list", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^切换设置页面/ }).click();

    const dialog = page.getByRole("dialog");
    await dialog.getByPlaceholder("筛选设置页面...").fill("zzz-no-such-settings-page-exists-12345");

    // `settings.compactNav.noResults` is a distinct i18n key from the top-bar cross-page search's
    // own `settings.search.noResults` ("没有匹配的设置项。") -- this is a separate, simpler
    // filter-by-typed-text-only mechanism, not the same `SettingsSearchBox` component re-rendered.
    await expect(dialog.getByRole("status")).toHaveText("没有匹配的设置页面。");
    await expect(dialog.getByRole("button")).toHaveCount(0);
  });

  test("selecting a page from the compact nav navigates to it and closes the sheet", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^切换设置页面/ }).click();

    // Prefix-anchored regex, not an exact match: a nav entry's status dot carries an sr-only
    // sibling text node that folds into the button's own computed accessible name (the same
    // gotcha already documented for the shell-level status tests and the cross-page search spec).
    await page.getByRole("dialog").getByRole("button", { name: /^SSH 连接/ }).click();

    await expect(page.getByRole("button", { name: "新增连接" })).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);
  });

  test("the trigger's own accessible label names the active page and updates once it changes", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();

    // Default active page is "basic" (`defaultSettingsPageId` in settings-pages.ts). The trigger's
    // aria-label is `t("settings.compactNav.trigger", { page: t(activePage.labelKey) })`
    // ("切换设置页面，当前：{{page}}") -- a stable "switch page" label plus the current page's own
    // name, not just the page name alone, so a screen reader hears it as a navigation control.
    await expect(page.getByRole("button", { name: /^切换设置页面.*基础配置/ })).toBeVisible();

    await page.getByRole("button", { name: /^切换设置页面/ }).click();
    await page.getByRole("dialog").getByRole("button", { name: /^SSH 连接/ }).click();
    await expect(page.getByRole("dialog")).toHaveCount(0);

    // Re-open the same trigger: its label now names the newly active page, not the old one.
    await page.getByRole("button", { name: /^切换设置页面/ }).click();
    await expect(page.getByRole("button", { name: /^切换设置页面.*SSH 连接/ })).toBeVisible();
    await expect(page.getByRole("button", { name: /^切换设置页面.*基础配置/ })).toHaveCount(0);
  });
});
