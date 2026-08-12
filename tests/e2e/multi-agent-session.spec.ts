import { expect, test, type Page } from "@playwright/test";

/**
 * Covers what the Web/mock adapter can actually exercise: composing a line-up, seeing who is in
 * the room, and switching a seat-scoped tab.
 *
 * Handoff routing and the three human intents are deliberately absent. They are decided in the
 * native turn coordinator, which does not exist in the mock adapter, so an E2E assertion here
 * would be testing a mock rather than the feature. Their coverage is the Rust suite in
 * `seat_turn_tests.rs` plus the real-Agent check in task 11.6.
 */

async function openCreateSessionDialog(page: Page) {
  await page.goto("/");
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled({ timeout: 1_000 });
  }).toPass({ timeout: 10_000 });
}

async function createMultiSeatSession(page: Page, title: string) {
  await openCreateSessionDialog(page);
  await page.getByRole("button", { name: /多个 Agent 在同一会话里协作/ }).click();

  // Switching to multi seeds two seats so the editor opens usable; no clicks are needed here.
  // A seat defaults to the first *available* Agent, which the mock registry may not have, so each
  // seat is bound explicitly rather than relying on that default.
  // A seat row is the block holding a remove button; the dialog has other Agent selects that are
  // not seats, so each row is located through its own control rather than by page-wide role.
  const seatRows = page.locator("div.ucd-list-row").filter({
    has: page.getByRole("button", { name: /删除席位/ }),
  });
  await expect(seatRows).toHaveCount(2);
  for (let index = 0; index < 2; index += 1) {
    const select = seatRows.nth(index).getByRole("combobox", { name: "Agent" });
    const value = await select.locator("option").nth(index).getAttribute("value");
    if (value) await select.selectOption(value);
    const roleSelect = seatRows.nth(index).getByRole("combobox", { name: "角色" });
    const roleValue = await roleSelect.locator("option").nth(index + 1).getAttribute("value");
    if (roleValue) await roleSelect.selectOption(roleValue);
  }
  await page.getByPlaceholder("新会话").fill(title);

  const createButton = page.getByRole("button", { name: "创建", exact: true });
  await expect(createButton).toBeEnabled();
  await createButton.click();
}

test.describe("multi-Agent session", () => {
  test("the multi-Agent mode is offered and composes a line-up", async ({ page }) => {
    await openCreateSessionDialog(page);

    const multi = page.getByRole("button", { name: /多个 Agent 在同一会话里协作/ });
    await expect(multi).toBeVisible();
    // The mode shipped disabled with a coming-soon hint; enabling it is what task 4.3 delivered.
    await expect(page.getByText(/暂未实现/)).toHaveCount(0);

    await multi.click();
    await expect(page.getByText(/^席位$/).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /添加席位/ })).toBeVisible();
  });

  test("a seat can be added and removed before the session is created", async ({ page }) => {
    await openCreateSessionDialog(page);
    await page.getByRole("button", { name: /多个 Agent 在同一会话里协作/ }).click();

    const removeButtons = page.getByRole("button", { name: /删除席位/ });
    await expect(removeButtons).toHaveCount(2);

    await page.getByRole("button", { name: /添加席位/ }).click();
    await expect(removeButtons).toHaveCount(3);
    await removeButtons.last().click();
    await expect(removeButtons).toHaveCount(2);
  });

  test("a multi-seat session shows its seats and switches a seat-scoped tab", async ({ page }) => {
    await createMultiSeatSession(page, "多 Agent 协作");

    await expect(page.getByTestId("contiguous-chat-workspace")).toBeVisible();
    await expect(page.getByTestId("attached-chat-composer")).toBeVisible();
    const composerSurface = page.getByTestId("wechat-style-composer");
    await expect(composerSurface).toBeVisible();
    await expect(composerSurface.getByTestId("composer-toolbar")).toBeVisible();
    await expect(composerSurface.getByPlaceholder(/输入指令/)).toBeVisible();

    // The seats view answers who is in the room without offering a control that dispatches them.
    await expect(page.getByText(/^席位$/).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /指派|派给|dispatch/i })).toHaveCount(0);

    await page.getByRole("tab", { name: /终端|Terminal/ }).first().click();
    await expect(page.getByRole("tablist", { name: /席位切换/ })).toBeVisible();
  });

  test("a running shared session exposes roster presence, membership edits, and participant mentions", async ({ page }) => {
    await createMultiSeatSession(page, "共享成员会话");

    const sessionCard = page.locator("[data-session-id]").filter({ hasText: "共享成员会话" });
    await expect(sessionCard.getByTestId("multi-agent-session-badge")).toHaveText("多 Agent");
    await expect(sessionCard.locator(".ucd-agent-claude")).toBeVisible();
    await expect(sessionCard.locator("[data-role-icon]")).toHaveCount(0);
    const titleBox = await sessionCard.getByText("共享成员会话", { exact: true }).boundingBox();
    const badgeBox = await sessionCard.getByTestId("multi-agent-session-badge").boundingBox();
    expect(badgeBox?.y).toBeGreaterThan(titleBox?.y ?? 0);
    const sessionList = page.locator("#workspace-session-sidebar .overflow-x-hidden").first();
    expect(await sessionList.evaluate((element) => element.scrollWidth <= element.clientWidth)).toBe(true);
    const conversationHeader = page.getByTestId("session-conversation-header");
    await expect(conversationHeader.getByTestId("session-roster-chips")).toHaveCount(0);
    await expect(conversationHeader.getByText("codex-cli", { exact: true })).toHaveCount(0);
    await expect(conversationHeader.getByText("claude-code", { exact: true })).toHaveCount(0);
    const editor = page.getByTestId("session-roster-editor");
    await expect(editor).toBeVisible();
    await expect(editor).toHaveAccessibleName("成员信息");
    const memberTab = page.getByRole("tab", { name: "成员信息" });
    await expect(memberTab).toHaveAttribute("aria-selected", "true");
    await page.getByRole("tab", { name: "基本信息" }).click();
    await expect(editor).toBeHidden();
    await memberTab.click();
    await expect(editor).toBeVisible();
    await expect(editor.locator('[data-role-icon="architect"]')).toBeVisible();
    await expect(editor.locator('[data-role-icon="implementer"]')).toBeVisible();
    await page.getByRole("button", { name: "专注对话" }).click();
    await expect(page.locator(".ucd-workspace-grid")).toHaveAttribute("data-info-collapsed", "true");
    await page.getByRole("button", { name: "恢复工作区" }).click();
    await expect(editor).toBeVisible();
    const participantRows = editor.locator("li.ucd-list-row");
    await expect(participantRows).toHaveCount(2);

    const addAgent = editor.getByRole("combobox", { name: "Agent" });
    const addAgentId = await addAgent.locator("option").nth(1).getAttribute("value");
    if (addAgentId) await addAgent.selectOption(addAgentId);
    await editor.getByRole("button", { name: /添加席位/ }).click();
    await expect(participantRows).toHaveCount(3);
    await editor.getByRole("button", { name: /成员离席/ }).last().click();
    await expect(participantRows).toHaveCount(2);
    await expect(editor.getByText(/已离席成员/)).toBeVisible();

    const composer = page.getByPlaceholder(/输入指令/);
    await composer.fill("@cod");
    await expect(page.getByRole("group", { name: /会话成员/ })).toBeVisible();
    const longPrompt = Array.from({ length: 80 }, () => "请一起检查这个方案").join("，");
    await composer.fill(longPrompt);
    await page.getByRole("button", { name: "发送", exact: true }).click();
    const historicalSpeaker = page.getByTestId("message-speaker").first();
    await expect(historicalSpeaker).toBeVisible();
    const speakerLabel = await historicalSpeaker.textContent();
    const messageRegion = page.getByTestId("message-scroll-region");
    const messageCanvas = page.getByTestId("message-readable-measure");
    await expect.poll(() => messageRegion.evaluate((element) => element.scrollHeight > element.clientHeight)).toBe(true);
    await messageRegion.evaluate((element) => { element.scrollTop = element.scrollHeight; });
    const expectAdaptiveMessageCanvas = async () => {
      await expect.poll(async () => {
        const regionBox = await messageRegion.boundingBox();
        const canvasBox = await messageCanvas.boundingBox();
        return (canvasBox?.width ?? 0) / Math.max(regionBox?.width ?? 1, 1);
      }).toBeGreaterThan(0.88);
    };
    const expectLatestMessageAnchored = async () => {
      await expect.poll(() => messageRegion.evaluate((element) => Math.abs(element.scrollHeight - element.scrollTop - element.clientHeight))).toBeLessThan(4);
      await expect(messageRegion.locator("article").first()).toContainText("请一起检查这个方案");
    };

    await expectAdaptiveMessageCanvas();
    const normalCanvasWidth = (await messageCanvas.boundingBox())?.width ?? 0;
    await page.getByRole("button", { name: "专注对话" }).click();
    await expectLatestMessageAnchored();
    await expectAdaptiveMessageCanvas();
    await expect.poll(async () => (await messageCanvas.boundingBox())?.width ?? 0).toBeGreaterThan(normalCanvasWidth + 150);
    await page.getByRole("button", { name: "恢复工作区" }).click();
    await expectLatestMessageAnchored();
    await expectAdaptiveMessageCanvas();
    await page.getByTestId("conversation-overflow-trigger").click();
    await page.getByTestId("toggle-info-panel").click();
    await expectLatestMessageAnchored();
    await expectAdaptiveMessageCanvas();
    await page.getByTestId("conversation-overflow-trigger").click();
    await page.getByTestId("toggle-info-panel").click();
    await expectLatestMessageAnchored();

    await editor.getByRole("button", { name: /成员离席/ }).first().click();
    await expect(participantRows).toHaveCount(1);
    await expect(historicalSpeaker).toHaveText(speakerLabel ?? "");
  });

  test("a single-Agent session offers no seat switcher", async ({ page }) => {
    await openCreateSessionDialog(page);
    await page.getByPlaceholder("新会话").fill("单 Agent 会话");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    const sessionCard = page.locator("[data-session-id]").filter({ hasText: "单 Agent 会话" });
    await expect(sessionCard.getByTestId("multi-agent-session-badge")).toHaveCount(0);
    await expect(page.getByTestId("session-roster-editor")).toHaveCount(0);
    await expect(page.getByRole("tab", { name: "成员信息" })).toHaveCount(0);
    await expect(page.getByText("Token: 2,340")).toHaveCount(0);
    await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled();

    await page.getByRole("tab", { name: /终端|Terminal/ }).first().click();
    await expect(page.getByRole("tablist", { name: /席位切换/ })).toHaveCount(0);
  });
});
