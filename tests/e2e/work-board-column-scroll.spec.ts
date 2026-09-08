import { expect, test } from "@playwright/test";

test.describe("work board columns", () => {
  test("a full column scrolls inside the board instead of being clipped", async ({ page }) => {
    // Short enough that an inbox with a handful of cards cannot show every card at once.
    await page.setViewportSize({ width: 1280, height: 480 });
    await page.goto("/workspace/inbox/board");
    await expect(page.getByRole("heading", { name: "任务看板" })).toBeVisible();
    for (let index = 1; index <= 8; index += 1) {
      await page.getByRole("button", { name: "新建工作项" }).click();
      await page.getByLabel("标题").fill(`收件箱滚动 ${index}`);
      await page.getByRole("button", { name: "创建", exact: true }).click();
      await expect(page.getByTestId(/work-item-web-/).filter({ hasText: `收件箱滚动 ${index}` })).toBeAttached();
    }
    // Scoped to the board: the Inbox destination hosting it has a heading of the same name.
    const inbox = page.locator("#todo-board").getByRole("heading", { name: "收件箱" }).locator("xpath=ancestor::section[1]");
    await expect(inbox.getByTestId(/work-item-web-/).first()).toBeVisible();

    const measured = await inbox.evaluate((column) => {
      const board = column.closest("#todo-board")!;
      const list = column.querySelector<HTMLElement>(".overflow-y-auto")!;
      const cards = [...column.querySelectorAll<HTMLElement>('[data-testid^="work-item-"]')];
      return {
        columnBottom: column.getBoundingClientRect().bottom,
        boardBottom: board.getBoundingClientRect().bottom,
        listScrolls: list.scrollHeight > list.clientHeight + 1,
        lastCardBottom: cards.at(-1)?.getBoundingClientRect().bottom ?? 0,
        listBottom: list.getBoundingClientRect().bottom,
      };
    });
    // Eight cards do not fit in a 480px window, so the list must be the thing that scrolls, and the
    // column must end inside the board rather than under its clip.
    expect(measured.lastCardBottom).toBeGreaterThan(measured.listBottom);
    expect(measured.listScrolls).toBe(true);
    expect(measured.columnBottom).toBeLessThanOrEqual(measured.boardBottom + 1);
  });
});
