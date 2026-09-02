import { expect, test, type Page } from "@playwright/test";

/**
 * Task 12.20's "Save/Discard" and "leave protection" gap: `settings-shell.tsx`'s `guardedLeave`
 * and `navigationGuardDialog` (task 12.11/12.12's draft-guard model) already had real production
 * code but no Playwright coverage naming this mechanism directly.
 *
 * CLI Parameters (`cli-parameters-page.tsx`) is the only page actually wired to it -- confirmed by
 * grepping the whole `src/` tree, it is the sole production caller of `onDraftStateChange`.
 * Personalization and Local Media are both `saveMode: "draft"` in `settings-page-search-metadata.ts`
 * and both hold a real in-memory draft of their own, but neither one's top-level page component
 * (`pages/personalization-page.tsx`, `pages/local-media-page.tsx`) destructures or calls
 * `onDraftStateChange` at all, so the shell never receives a guard for either of them today.
 *
 * `settings-page-lifecycle.ts` marks all three `saveMode: "draft"` pages -- including CLI
 * Parameters itself -- `keepAlive: "draft-only"`, and `settings-shell.tsx`'s own `handleSelectPage`
 * only ever calls `guardedLeave` when the page being left has `keepAlive === "never"`. So switching
 * *between* settings pages while CLI Parameters is dirty never shows `navigationGuardDialog` --
 * `cli-parameters-page.tsx`'s own comment on its `onDraftStateChange` effect confirms this is
 * deliberate ("the page already survives an in-app page switch on its own via `keepAlive:
 * draft-only`"). The guard exists only for the one departure that lifecycle can't cover: leaving
 * Settings entirely (`onReturn`/`guardedOnReturn`), which is unconditional regardless of
 * `keepAlive`. The test below for inter-page switching therefore covers the real behavior (the
 * dirty draft survives silently, with neither a dialog nor a discard) rather than asserting a
 * dialog that no real page today can trigger.
 *
 * `tests/e2e/cli-parameters-settings.spec.ts` already exercises this same page's Model and
 * Reasoning-effort fields for a save-and-reload proof and a Stay/Discard-and-leave guard proof --
 * both predate this task. Tests here deliberately use a different, simpler field (`safeMode`, a
 * single-click boolean switch visible under the page's default "chat" scope, so no scope toggle is
 * needed first) and add the one guard outcome that pre-existing spec never exercises: confirming
 * "保存并离开" (Save and Leave), not just Stay or Discard.
 */
function safeModeSwitch(page: Page) {
  return page.getByRole("switch", { name: "安全模式" });
}

async function openCliParameters(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /设置|Settings/ }).click();
  await page.locator("nav").getByRole("button", { name: /^CLI 参数/ }).click();
}

test.describe("settings draft guard: save/discard and leave protection (task 12.20)", () => {
  test("editing a draft-mode field marks it dirty and the shared DraftActionBar becomes visible", async ({ page }) => {
    await openCliParameters(page);
    const toggle = safeModeSwitch(page);
    await expect(toggle).toHaveAttribute("aria-checked", "false");
    await expect(page.getByText("1 项未保存的更改")).toHaveCount(0);

    await toggle.click();

    await expect(toggle).toHaveAttribute("aria-checked", "true");
    // DraftActionBar (`ui/forms/DraftActionBar.tsx`): `role="region"`, renders nothing at
    // dirtyCount 0, so its own presence together with the exact interpolated count is the real
    // visible signal this proves, not just the Save button happening to be enabled.
    const draftBar = page.getByRole("region").filter({ hasText: "1 项未保存的更改" });
    await expect(draftBar).toBeVisible();
    await expect(draftBar.getByRole("button", { name: "保存", exact: true })).toBeEnabled();
    await expect(draftBar.getByRole("button", { name: "放弃", exact: true })).toBeVisible();
  });

  test("discarding the draft via the DraftActionBar reverts the field and clears the dirty signal", async ({ page }) => {
    await openCliParameters(page);
    const toggle = safeModeSwitch(page);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("1 项未保存的更改")).toBeVisible();

    // `discardActiveDraft()` in `cli-parameters-page.tsx` confirms before discarding -- a real
    // dialog distinct from the shell-level navigation guard, built on the same `ApplicationDialog`
    // primitive but reached through `useConfirmation`, not `useDraftNavigationGuard`.
    // Exact match matters here: `getByRole`'s default name match is a substring, and "保存" is a
    // literal substring of "未保存" -- once dirty, the sidebar nav entry's own folded status text
    // ("CLI 参数 1 项未保存") and the rail's per-agent badge ("Claude Code 未安装 1 项未保存") are
    // both real `role="button"` elements that would otherwise also match a bare "保存" search.
    await page.getByRole("button", { name: "放弃", exact: true }).click();
    const confirmDialog = page.getByRole("dialog");
    await expect(confirmDialog).toBeVisible();
    await expect(confirmDialog.getByRole("heading", { name: "放弃未保存的参数改动?" })).toBeVisible();
    await confirmDialog.getByRole("button", { name: "确认", exact: true }).click();
    await expect(confirmDialog).toBeHidden();

    await expect(toggle).toHaveAttribute("aria-checked", "false");
    await expect(page.getByText("1 项未保存的更改")).toHaveCount(0);
    await expect(page.getByRole("button", { name: "保存", exact: true })).toHaveCount(0);
  });

  test("saving the draft persists the value across a reload", async ({ page }) => {
    await openCliParameters(page);
    const toggle = safeModeSwitch(page);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-checked", "true");

    // Exact match: "保存" is a substring of the dirty sidebar/rail buttons' own folded "未保存"
    // status text (see the discard test's comment for the full explanation).
    await page.getByRole("button", { name: "保存", exact: true }).click();
    await expect(page.getByText("CLI 参数已保存")).toBeVisible();
    await expect(page.getByText("1 项未保存的更改")).toHaveCount(0);

    // A reload keeps the Settings view but resets to the default page (already established by
    // `cli-parameters-settings.spec.ts` -- there is no Settings button left to click a second time).
    await page.reload();
    await page.locator("nav").getByRole("button", { name: /^CLI 参数/ }).click();
    await expect(safeModeSwitch(page)).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("1 项未保存的更改")).toHaveCount(0);
  });

  test("switching to a different settings page while dirty neither shows a guard dialog nor discards the edit", async ({ page }) => {
    await openCliParameters(page);
    const toggle = safeModeSwitch(page);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("1 项未保存的更改")).toBeVisible();

    await page.locator("nav").getByRole("button", { name: /^关于/ }).click();

    await expect(page.getByRole("heading", { level: 2, name: /^关于/ })).toBeVisible();
    await expect(page.getByRole("dialog")).toHaveCount(0);

    // CLI Parameters never unmounted (`keepAlive: "draft-only"`) -- switching back finds the exact
    // same in-progress edit rather than the guard having silently discarded or blocked the switch.
    await page.locator("nav").getByRole("button", { name: /^CLI 参数/ }).click();
    await expect(safeModeSwitch(page)).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("1 项未保存的更改")).toBeVisible();
  });

  test("leaving Settings entirely while dirty triggers the guard; Stay keeps the edit and Save and Leave both saves and navigates away", async ({ page }) => {
    await openCliParameters(page);
    const toggle = safeModeSwitch(page);
    await toggle.click();
    await expect(toggle).toHaveAttribute("aria-checked", "true");

    await page.getByRole("button", { name: "返回", exact: true }).click();
    const dialog = page.getByRole("dialog");
    await expect(dialog).toBeVisible();
    await expect(dialog.getByRole("heading", { name: "有未保存的更改" })).toBeVisible();
    await expect(dialog.getByText("现在离开将放弃 1 项未保存的更改。")).toBeVisible();

    // Stay cancels the departure and leaves the draft exactly as it was.
    await dialog.getByRole("button", { name: "留在此页", exact: true }).click();
    await expect(dialog).toBeHidden();
    await expect(page).toHaveURL(/\/settings/);
    await expect(toggle).toHaveAttribute("aria-checked", "true");

    // Leaving again and choosing Save and Leave -- the one outcome of the three-way dialog
    // `cli-parameters-settings.spec.ts`'s own pre-existing guard test never exercises (it only
    // covers Stay and Discard) -- both saves the draft and actually navigates away.
    await page.getByRole("button", { name: "返回", exact: true }).click();
    await expect(dialog).toBeVisible();
    await dialog.getByRole("button", { name: "保存并离开", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace/);

    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.locator("nav").getByRole("button", { name: /^CLI 参数/ }).click();
    await expect(safeModeSwitch(page)).toHaveAttribute("aria-checked", "true");
    await expect(page.getByText("1 项未保存的更改")).toHaveCount(0);
  });
});
