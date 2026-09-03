import { expect, test } from "@playwright/test";

/**
 * Task 11.8: backward navigation through the 4-step create-session wizard must not lose already
 * entered draft values, and switching workspace mode must reset only the fields that mode change
 * genuinely invalidates -- not the rest of the draft. `create-session-wizard-steps.ts`'s step
 * position and `create-session-draft-model.ts`'s draft are two entirely separate hooks (task
 * 11.1's own split), so these are integration-level checks that the two never got coupled by
 * accident, not something a reducer-only unit test can prove.
 */
test.describe("create-session wizard back navigation", () => {
  test("preserves the project path and session title across a full back-and-forward round trip", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog");
    const next = dialog.getByRole("button", { name: "下一步" });
    const back = dialog.getByRole("button", { name: "上一步" });

    await next.click(); // Step 1 -> Step 2, defaults left as-is.
    await next.click(); // Step 2 -> Step 3.

    const projectPath = dialog.getByPlaceholder(/code.*project/);
    await projectPath.fill("D:\\wizard-back-nav-test");
    await projectPath.press("Tab");
    await expect(next).toBeEnabled({ timeout: 10_000 });
    await next.click(); // Step 3 -> Step 4.

    const title = dialog.getByPlaceholder("新会话");
    await title.fill("往返测试会话");

    // Step 4 -> Step 3: the field this test cares about most, since it sits right behind the
    // step whose own Next button gated on async validation.
    await back.click();
    await expect(dialog.getByPlaceholder(/code.*project/)).toHaveValue("D:\\wizard-back-nav-test");

    // Keep going all the way back to Step 1, then all the way forward again.
    await back.click(); // Step 3 -> Step 2.
    await back.click(); // Step 2 -> Step 1.
    await expect(dialog.getByRole("button", { name: "上一步" })).toHaveCount(0);

    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3.
    await expect(dialog.getByPlaceholder(/code.*project/)).toHaveValue("D:\\wizard-back-nav-test");
    await next.click(); // Step 3 -> Step 4.
    await expect(dialog.getByPlaceholder("新会话")).toHaveValue("往返测试会话");
  });

  test("resets worktree state on a workspace-mode round trip without touching the project path", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog");
    const next = dialog.getByRole("button", { name: "下一步" });
    const back = dialog.getByRole("button", { name: "上一步" });

    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3.

    const projectPath = dialog.getByPlaceholder(/code.*project/);
    await projectPath.fill("D:\\wizard-worktree-test");
    await projectPath.press("Tab");
    const worktreeCheckbox = dialog.getByRole("checkbox", { name: "创建新 Git worktree" });
    await expect(worktreeCheckbox).toBeEnabled({ timeout: 10_000 });
    await worktreeCheckbox.check();
    await expect(worktreeCheckbox).toBeChecked();

    // Step 3 -> Step 1 to flip the workspace mode, the one action task 11.8 says should reset a
    // field -- worktreeEnabled has no meaning once the workspace is remote.
    await back.click(); // Step 3 -> Step 2.
    await back.click(); // Step 2 -> Step 1.
    await dialog.getByRole("button", { name: "远端", exact: true }).click();

    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3 (now RemoteWorkspaceSection; no worktree checkbox at all).
    await expect(dialog.getByRole("checkbox", { name: "创建新 Git worktree" })).toHaveCount(0);

    // Back to local: worktreeEnabled must come back unchecked (reset, not restored), while the
    // project path this mode change never invalidated must still be exactly what was typed.
    await back.click(); // Step 3 -> Step 2.
    await back.click(); // Step 2 -> Step 1.
    await dialog.getByRole("button", { name: "本地", exact: true }).click();
    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3.

    await expect(dialog.getByPlaceholder(/code.*project/)).toHaveValue("D:\\wizard-worktree-test");
    await expect(dialog.getByRole("checkbox", { name: "创建新 Git worktree" })).not.toBeChecked();
  });
});

/**
 * Task 11.10: validation reasons (`create-session-validation.ts`) existed as structured data
 * with nowhere they were actually shown -- Next/Create were silently disabled with no explanation
 * anywhere in the wizard. These check the display layer added on top, not the validation logic
 * itself (already covered by `create-session-validation.test.ts`).
 */
test.describe("create-session wizard validation display", () => {
  test("shows the workspace error at the owning field until a path is entered", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog");
    const next = dialog.getByRole("button", { name: "下一步" });
    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3.

    const workspaceError = dialog.getByText("请填写项目路径。");
    await expect(workspaceError).toBeVisible();
    await expect(next).toBeDisabled();

    await dialog.getByPlaceholder(/code.*project/).fill("D:\\wizard-validation-test");
    await dialog.getByPlaceholder(/code.*project/).press("Tab");
    await expect(workspaceError).toHaveCount(0);
    await expect(next).toBeEnabled({ timeout: 10_000 });
  });

  test("shows the seats error at the owning field after removing a seat below the minimum", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog");
    await dialog.getByRole("button", { name: "多 Agent" }).click();
    const next = dialog.getByRole("button", { name: "下一步" });
    await next.click(); // Step 1 -> Step 2 (multi mode, seeded with 2 seats).

    await expect(dialog.getByText("请至少添加两个席位。")).toHaveCount(0);
    await dialog.getByRole("button", { name: "删除席位" }).first().click();

    await expect(dialog.getByText("请至少添加两个席位。")).toBeVisible();
    await expect(next).toBeDisabled();
  });
});

/**
 * Task 11.11: submission commits through an async operation
 * (`use-create-session-draft.ts`'s 600ms poll against `operationService`), which is a real window
 * where a second click or a dismissal attempt could otherwise land mid-commit.
 */
test.describe("create-session wizard submission safety", () => {
  test("disables Create and blocks Escape/backdrop dismissal while the session is being created", async ({ page }) => {
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog");
    const next = dialog.getByRole("button", { name: "下一步" });
    await next.click(); // Step 1 -> Step 2.
    await next.click(); // Step 2 -> Step 3.
    await dialog.getByPlaceholder(/code.*project/).fill("D:\\wizard-submit-safety-test");
    await dialog.getByPlaceholder(/code.*project/).press("Tab");
    await expect(next).toBeEnabled({ timeout: 10_000 });
    await next.click(); // Step 3 -> Step 4.

    await dialog.getByPlaceholder("新会话").fill("提交安全测试会话");
    const create = dialog.getByRole("button", { name: "创建", exact: true });
    await create.click();

    // Mid-commit: the operation is still polling, so the dialog must still be exactly one dialog
    // (not a second one from a duplicate submission) and must refuse to close. Step 4 is not the
    // first step, so the footer's dismiss-adjacent control here is "上一步" (Back), not "取消"
    // (Cancel) -- both share the same `disabled={lifecycle.loading}`.
    await expect(create).toBeDisabled();
    await page.keyboard.press("Escape");
    await page.mouse.click(4, 4); // Backdrop corner, outside the dialog's own section element.
    await expect(dialog).toHaveCount(1);
    await expect(dialog.getByRole("button", { name: "上一步" })).toBeDisabled();

    // Let the real operation finish so the test does not leave a dangling poll behind it.
    await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled({ timeout: 15_000 });
  });
});

/**
 * Task 11.12: `create-session-dialog-content.tsx` picks `ApplicationDialog` (centered, bounded
 * width) at 640px and above, `Sheet` `placement="full"` (edge-to-edge) below it -- both share the
 * same `role="dialog"` and title, so the only externally observable difference is layout, not
 * semantics. `Sheet`'s `placement="full"` sizes itself via `inset-0` on a `position: fixed`
 * ancestor rather than an explicit `100vh`, which is what keeps its footer reachable once a real
 * mobile browser's visual viewport shrinks for an on-screen keyboard -- not independently provable
 * in Playwright (no real virtual keyboard to trigger), so this only checks the layout switch
 * itself actually fires at the right breakpoint.
 */
test.describe("create-session wizard responsive dialog", () => {
  test("renders as a centered, width-bounded dialog at desktop width", async ({ page }) => {
    await page.setViewportSize({ width: 1440, height: 900 });
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog", { name: "创建会话" });
    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.width).toBeLessThan(900); // Bounded by `max-w-2xl` (~672px), nowhere near 1440.
    expect(box!.x).toBeGreaterThan(50); // Centered, not flush against the viewport edge.
  });

  test("renders as an edge-to-edge full-height sheet below the wide breakpoint", async ({ page }) => {
    await page.setViewportSize({ width: 500, height: 800 });
    await page.goto("/");
    await page.getByRole("button", { name: /新建/ }).click();

    const dialog = page.getByRole("dialog", { name: "创建会话" });
    const box = await dialog.boundingBox();
    expect(box).not.toBeNull();
    expect(box!.x).toBe(0);
    expect(box!.width).toBe(500); // Fills the viewport edge to edge, unlike the bounded dialog.
  });
});

/**
 * Task 11.14: single+CLI+remote reaching a real, working session was the one supported mode
 * combination no existing spec completed end to end (every other combination -- single+CLI+local,
 * single+API+local, multi+CLI+local -- already had one). The Web mock's own remote-workspace
 * normalization (`web-known-workspace-client.ts`) only validates host/port/path syntax, never
 * reachability, so completing this proves the wizard's remote path threads data through to a
 * successful `createSession` call end-to-end -- the same thing every other mode's own mock-backed
 * test already proves for its path, not a claim about real SSH connectivity.
 */
test("completes a single-Agent CLI session against a remote workspace", async ({ page }) => {
  await page.goto("/");
  await page.getByRole("button", { name: /新建/ }).click();

  const dialog = page.getByRole("dialog");
  const next = dialog.getByRole("button", { name: "下一步" });
  await dialog.getByRole("button", { name: "远端", exact: true }).click(); // Step 1: remote workspace, single/CLI left as-is.
  await next.click(); // Step 1 -> Step 2, defaults left as-is.
  await next.click(); // Step 2 -> Step 3.

  await dialog.getByLabel("主机").fill("build.example.com");
  await dialog.getByLabel("远端路径").fill("/home/vane/project");
  await expect(next).toBeEnabled({ timeout: 10_000 });
  await next.click(); // Step 3 -> Step 4.

  await dialog.getByPlaceholder("新会话").fill("远端会话完整流程");
  const create = dialog.getByRole("button", { name: "创建", exact: true });
  await expect(create).toBeEnabled();
  await create.click();

  await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled({ timeout: 15_000 });
});

/**
 * 20.19: every test above drives the wizard via `.click()` on Next/Back/mode buttons -- the
 * `.press("Tab")` calls in `session-helpers.ts`'s own `createSession` (reused throughout this file)
 * only trigger the project-path field's own blur validation, and the one `page.keyboard.press
 * ("Escape")` in the submission-safety test above is a negative check that dismissal stays blocked,
 * not a real navigation step. This is the first full pass: all 4 steps (mode, participant,
 * workspace, review) advance via a focused button and a real `Enter`, and both text fields are
 * filled with real keystrokes (`page.keyboard.type`), not `.fill()`. `.focus()` establishes each
 * step's own starting point, the same house style this whole task's other new keyboard tests use.
 */
test("creates a session end to end using only the keyboard", async ({ page }) => {
  await page.goto("/");
  const trigger = page.getByRole("button", { name: "新建" });
  await trigger.focus();
  await page.keyboard.press("Enter");

  const dialog = page.getByRole("dialog");
  const next = dialog.getByRole("button", { name: "下一步" });

  // Step 1 (mode) -> Step 2, single-Agent/CLI defaults left as-is.
  await next.focus();
  await expect(next).toBeFocused();
  await page.keyboard.press("Enter");
  // Step 2 (participant) -> Step 3, defaults left as-is.
  await next.focus();
  await page.keyboard.press("Enter");

  // Step 3 (workspace): real keystrokes into the project-path field, then Tab to blur it -- the
  // same field/trigger `createSession` (session-helpers.ts) establishes Next's own enablement
  // gates on.
  const projectPath = dialog.getByPlaceholder(/code.*project/);
  await projectPath.focus();
  await page.keyboard.type("D:\\keyboard-only-wizard-test");
  await page.keyboard.press("Tab");
  await expect(next).toBeEnabled({ timeout: 10_000 });
  await next.focus();
  await page.keyboard.press("Enter");

  // Step 4 (review): real keystrokes into the session-title field, then Enter on Create.
  const title = dialog.getByPlaceholder("新会话");
  await title.focus();
  await page.keyboard.type("键盘创建会话");
  const create = dialog.getByRole("button", { name: "创建", exact: true });
  await create.focus();
  await expect(create).toBeEnabled();
  await page.keyboard.press("Enter");

  await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled({ timeout: 15_000 });
});
