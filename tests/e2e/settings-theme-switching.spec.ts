import { expect, test, type Page } from "@playwright/test";

/**
 * Task 12.20's "both themes" dimension. The only existing precedent anywhere in this codebase is
 * `extensions-settings.spec.ts`'s "preserves the page while switching both registered themes" --
 * it drives the real `<select aria-label="主题">` in `basic-settings-page.tsx`, then only asserts
 * two things: the already-visible page content survives the switch, and `<html>`'s `data-theme`
 * attribute equals the selection. It never reads anything the CSS attribute selector actually
 * produces, never checks persistence, and never checks layout -- this file covers exactly those
 * three gaps rather than re-running that same page-survives-a-switch proof.
 *
 * The real mechanism, read directly rather than assumed:
 * - `basic-settings-page.tsx`'s theme row calls `saveSetting("theme", value)` from `useSettings()`
 *   (`settings-provider.tsx`), whose `applySettings()` sets `document.documentElement.dataset.theme`
 *   optimistically, then persists through the injected `SettingsService`.
 * - `src/theme/theme-registry.ts` registers exactly two themes, `"minimal"` (the default) and
 *   `"futuristic"`, and separately exports `themeStorageKey = "vanehub.uiStyle"` -- grepping the
 *   whole repo shows zero production read/write call sites for that key
 *   (`docs/ui-redesign/baseline.md`'s own audit already flags it as dead). The real persistence key
 *   is a different, local constant inside `web-settings-client.ts`: `"vanehub.appSettings"`, the
 *   whole `AppSettings` blob (this suite runs the Web/mock adapter, selected by
 *   `runtime-settings-client.ts` outside of Tauri -- not the native settings service the desktop
 *   client uses). `application-locales.spec.ts` already reads this same key for its own
 *   `applicationLanguage` field; this file uses the identical key and idiom for `theme`.
 * - `src/styles.css`'s `:root[data-theme="futuristic"]` and `:root[data-theme="minimal"]` blocks
 *   are where `data-theme` actually does something: futuristic is a dark palette
 *   (`color-scheme: dark`, `--background: 216 28% 7%`), minimal is a light palette
 *   (`color-scheme: light`, `--background: 0 0% 100%`). Reading `--background` and the standard
 *   `color-scheme` property back out via `getComputedStyle` is the "did the switch actually repaint
 *   anything" signal this task asks for, sourced from this CSS rather than guessed.
 */

type ThemeId = "minimal" | "futuristic";

const appSettingsStorageKey = "vanehub.appSettings";

async function openSettings(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
}

function themeSelect(page: Page) {
  return page.getByLabel(/主题|Theme/);
}

async function selectTheme(page: Page, themeId: ThemeId) {
  await themeSelect(page).selectOption(themeId);
  await expect(page.locator("html")).toHaveAttribute("data-theme", themeId);
}

async function readBackgroundToken(page: Page): Promise<string> {
  return page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("--background").trim());
}

async function readColorScheme(page: Page): Promise<string> {
  return page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue("color-scheme").trim());
}

async function readPersistedTheme(page: Page) {
  return page.evaluate((key) => JSON.parse(window.localStorage.getItem(key) ?? "{}").theme, appSettingsStorageKey);
}

async function expectNoHorizontalOverflow(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(true);
}

test.describe("settings theme switching (task 12.20)", () => {
  test("switching between minimal and futuristic changes the --background design token and computed color-scheme", async ({ page }) => {
    await openSettings(page);

    await selectTheme(page, "futuristic");
    const futuristicBackground = await readBackgroundToken(page);
    const futuristicScheme = await readColorScheme(page);

    await selectTheme(page, "minimal");
    const minimalBackground = await readBackgroundToken(page);
    const minimalScheme = await readColorScheme(page);

    expect(futuristicBackground).not.toBe(minimalBackground);
    expect(futuristicScheme).not.toBe(minimalScheme);
    // Concrete values, not just "differs" -- proves the switch applied the intended dark/light
    // palette rather than merely toggling to some other differing value.
    expect(futuristicScheme).toBe("dark");
    expect(minimalScheme).toBe("light");
  });

  test("the selected theme persists through the Web adapter's own settings storage and survives a reload", async ({ page }) => {
    await openSettings(page);
    await selectTheme(page, "futuristic");

    // `expect.poll`, not a one-shot read: `saveSetting` (`settings-provider.tsx`) applies the
    // `data-theme` DOM update optimistically *before* it awaits the `SettingsService.saveSetting`
    // call that actually writes `localStorage` -- the same ordering `application-locales.spec.ts`
    // already accounts for with `expect.poll` on this identical storage key for a different field.
    await expect.poll(() => readPersistedTheme(page)).toBe("futuristic");

    await page.reload();
    // Applied at the provider root before any settings page is even shown -- true immediately after
    // reload without navigating back into Settings first, which is what proves this is real
    // boot-time persistence rather than something only the settings page itself remembers.
    await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");

    // A reload of a URL already inside Settings stays inside Settings, back on the default page
    // (`settings-save-discard.spec.ts` establishes the same behavior) -- the theme `<select>` is
    // already showing, no extra navigation needed to confirm it also remembers the selection
    // itself, not just the DOM attribute.
    await expect(themeSelect(page)).toHaveValue("futuristic");
  });

  test("the futuristic theme renders both page-header shapes without horizontal overflow at a narrow representative viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openSettings(page);
    await selectTheme(page, "futuristic");

    // Basic Settings is `defaultSettingsPageId` -- already showing, no compact-nav trigger needed
    // to reach this first page.
    await expect(page.getByRole("heading", { level: 2, name: /^基础配置/ })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    // Below `lg` the sidebar `<nav>` used by every other spec's nav clicks is gone from the
    // accessibility tree entirely (`settings-compact-navigation.spec.ts`) -- reach the second,
    // structurally different page (the shared `ui/page-header/PageHeader`, an `<h1>`, vs. Basic
    // Settings' local `page-parts.tsx` `PageHeader`, an `<h2>`) through the compact-nav sheet
    // trigger instead.
    await page.getByRole("button", { name: /^切换设置页面/ }).click();
    await page.getByRole("dialog").getByRole("button", { name: /^SSH 连接/ }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "futuristic");
    await expect(page.getByRole("heading", { level: 1, name: /^SSH 连接/ })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });

  test("the minimal theme renders both page-header shapes without horizontal overflow at a narrow representative viewport", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await openSettings(page);
    await selectTheme(page, "minimal");

    await expect(page.getByRole("heading", { level: 2, name: /^基础配置/ })).toBeVisible();
    await expectNoHorizontalOverflow(page);

    await page.getByRole("button", { name: /^切换设置页面/ }).click();
    await page.getByRole("dialog").getByRole("button", { name: /^SSH 连接/ }).click();
    await expect(page.locator("html")).toHaveAttribute("data-theme", "minimal");
    await expect(page.getByRole("heading", { level: 1, name: /^SSH 连接/ })).toBeVisible();
    await expectNoHorizontalOverflow(page);
  });
});
