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

    // The seats view answers who is in the room without offering a control that dispatches them.
    await expect(page.getByText(/^席位$/).first()).toBeVisible();
    await expect(page.getByRole("button", { name: /指派|派给|dispatch/i })).toHaveCount(0);

    await page.getByRole("tab", { name: /终端|Terminal/ }).first().click();
    await expect(page.getByRole("tablist", { name: /席位切换/ })).toBeVisible();
  });

  test("a single-Agent session offers no seat switcher", async ({ page }) => {
    await openCreateSessionDialog(page);
    await page.getByPlaceholder("新会话").fill("单 Agent 会话");
    await page.getByRole("button", { name: "创建", exact: true }).click();
    await expect(page.getByRole("textbox", { name: "Terminal input" })).toBeEnabled();

    await page.getByRole("tab", { name: /终端|Terminal/ }).first().click();
    await expect(page.getByRole("tablist", { name: /席位切换/ })).toHaveCount(0);
  });
});
