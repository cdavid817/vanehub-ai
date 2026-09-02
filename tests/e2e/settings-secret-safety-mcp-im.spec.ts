import { expect, test } from "@playwright/test";

// Task 12.20: secret-safety coverage for the two remaining sensitive-risk pages 12.19's own audit
// named (`mcp`, `im`) -- SSH Connections and Execution Observability are already covered in their
// own spec files. Style/structure follows `ssh-connections-settings.spec.ts`'s own task-12.20 test
// (fill a secret, save, confirm it never touches `page.locator("body")`, confirm the real
// re-display behavior on reopen) and reuses `execution-observability.spec.ts`'s own documented
// lesson: `type="password"` has no implicit ARIA role, so `getByRole("textbox", ...)` cannot find
// it -- use `getByLabel(...)` instead.

test.describe("MCP server settings — secret safety (task 12.20)", () => {
  test("never shows a saved header secret in the page body, and re-masks it on every reopen", async ({ page }) => {
    const secret = "playwright-e2e-mcp-header-secret";
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByText(/^(MCP 服务器|MCP Servers)$/).click();

    await page.getByRole("button", { name: /添加 MCP|Add MCP/ }).click();
    const addDialog = page.getByRole("dialog");
    await addDialog.getByRole("textbox", { name: /^(名称|Name)$/ }).fill("secret-safety-server");
    // Switch off the default stdio transport: task 12.13's own env-var masking test already covers
    // stdio/env. Headers/streamable_http is the other half of the same symmetric masking mechanism
    // (`envRevealed`/`headersRevealed` in mcp-server-form.tsx) and is not covered by that test.
    await addDialog.getByRole("combobox", { name: /^(传输方式|Transport)$/ }).selectOption("streamable_http");
    await addDialog.getByRole("textbox", { name: /^(URL)$/ }).fill("https://mcp.example.com/stream");
    await addDialog.getByRole("textbox", { name: /请求头 JSON|Headers JSON/ }).fill(`{"Authorization": "Bearer ${secret}"}`);
    await addDialog.getByRole("button", { name: /^(保存|Save)$/ }).click();
    await expect(addDialog).toBeHidden();
    await expect(page.locator("body")).not.toContainText(secret);

    // The nearest `article` ancestor specifically -- `.filter({ has })` would also match an outer
    // wrapping element that contains every card. Task 12.18: the card root moved to `<article>`.
    const card = page.getByRole("heading", { name: "secret-safety-server", exact: true }).locator("xpath=ancestor::article[1]");
    await card.getByRole("button", { name: /secret-safety-server的操作|Actions for secret-safety-server/ }).click();
    await card.getByRole("menuitem", { name: /^(编辑|Edit)$/ }).click();
    const editDialog = page.getByRole("dialog");
    await expect(editDialog).toBeVisible();

    // Unlike SSH/OTLP's write-only password fields, a saved MCP server's headers/env start masked
    // behind an explicit "Reveal" (task 12.13) rather than empty -- editing a server needs its
    // other, non-secret entries visible to modify them in place. This is an intentional, different,
    // already-spec'd design (mcp-server-form.tsx's own comment), not a bug -- but the raw secret
    // must still never be shown just from opening the dialog, and must never touch the page body.
    await expect(editDialog.getByRole("textbox", { name: /请求头 JSON|Headers JSON/ })).toHaveCount(0);
    await expect(editDialog.getByRole("button", { name: /^(显示|Reveal)$/ })).toBeVisible();
    await expect(page.getByText(secret)).toHaveCount(0);
    await expect(page.locator("body")).not.toContainText(secret);

    await editDialog.getByRole("button", { name: /^(显示|Reveal)$/ }).click();
    await expect(editDialog.getByRole("textbox", { name: /请求头 JSON|Headers JSON/ })).toHaveValue(new RegExp(secret));

    // Revealing is a one-time, per-open action, not a permanent unmask: `McpPage` fully unmounts
    // `McpServerForm` on close (`editingServer !== undefined ? <McpServerForm ... /> : null`), so
    // its `headersRevealed` state can't survive into the next open -- closing and reopening the
    // same server's edit dialog must mask the secret again, not remember last time's Reveal click.
    await page.keyboard.press("Escape");
    await expect(editDialog).toBeHidden();
    await card.getByRole("button", { name: /secret-safety-server的操作|Actions for secret-safety-server/ }).click();
    await card.getByRole("menuitem", { name: /^(编辑|Edit)$/ }).click();
    await expect(editDialog).toBeVisible();
    await expect(editDialog.getByRole("textbox", { name: /请求头 JSON|Headers JSON/ })).toHaveCount(0);
    await expect(editDialog.getByRole("button", { name: /^(显示|Reveal)$/ })).toBeVisible();
    await expect(page.getByText(secret)).toHaveCount(0);
    await expect(page.locator("body")).not.toContainText(secret);
  });
});

test.describe("IM connector settings — secret safety (task 12.20)", () => {
  test("never shows a saved bot token again, even after navigating away and back", async ({ page }) => {
    const secret = "playwright-e2e-telegram-bot-token";
    await page.goto("/");
    await page.getByRole("button", { name: /设置|Settings/ }).click();
    await page.getByRole("button", { name: /^(IM 能力|IM Connectors)$/ }).click();

    const telegram = page.locator('[data-connector="telegram"]');
    await telegram.getByRole("button", { expanded: false }).click();
    await telegram.getByLabel("Bot Token").fill(secret);
    await telegram.getByRole("button", { name: "保存凭据" }).click();
    await expect(page.getByText("连接器凭据已保存。")).toBeVisible();
    await expect(page.locator("body")).not.toContainText(secret);

    // The just-saved token is never re-populated -- only an "already configured" placeholder,
    // matching this session's own already-proven SSH-password write-only pattern.
    // `im-connector-row.tsx` always sources a secret field's `value` from the in-memory,
    // per-save-cleared `credentials` draft (`credentialDraftAfterSave` in `im-form.ts`), never from
    // `view.config.publicConfig` or any other value the mock/native backend would have persisted.
    const tokenField = telegram.getByLabel("Bot Token");
    await expect(tokenField).toHaveValue("");
    await expect(tokenField).toHaveAttribute("placeholder", "已配置，仅在替换时输入新值");
    await expect(page.locator("body")).not.toContainText(secret);

    // Reopen for real: the `im` settings page has `keepAlive: "never"` (settings-page-lifecycle.ts,
    // Decision 6's own default), so navigating to another page and back fully unmounts and
    // remounts `ImPage` -- a fresh `ImConnectorRow` instance with fresh `credentials`/`expanded`
    // state, not just the same live component instance sitting on state already cleared by the
    // save above. This proves the secret doesn't come back from the "server" side either.
    await telegram.getByRole("button", { expanded: true }).click();
    await page.getByRole("button", { name: /^(SSH 连接|SSH Connections)$/ }).click();
    await page.getByRole("button", { name: /^(IM 能力|IM Connectors)$/ }).click();

    const telegramReopened = page.locator('[data-connector="telegram"]');
    await telegramReopened.getByRole("button", { expanded: false }).click();
    const reopenedTokenField = telegramReopened.getByLabel("Bot Token");
    await expect(reopenedTokenField).toHaveValue("");
    await expect(reopenedTokenField).toHaveAttribute("placeholder", "已配置，仅在替换时输入新值");
    await expect(page.locator("body")).not.toContainText(secret);
  });
});
