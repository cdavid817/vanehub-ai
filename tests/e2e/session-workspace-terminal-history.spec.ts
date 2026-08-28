import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

function panel(page: Page) {
  return page.getByRole("tabpanel", { name: "终端记录" });
}

async function openTerminalHistory(page: Page, title = "执行记录测试") {
  await page.goto("/");
  await createSession(page, title);
  await page.getByRole("tab", { name: /终端记录/ }).click();
  await expect(panel(page).getByRole("tablist", { name: "执行记录视图" })).toBeVisible();
}

test.describe("terminal history execution records", () => {
  test("walks the native views, filters, details, and a cross-panel jump", async ({ page }) => {
    await openTerminalHistory(page);

    // Every native view is its own corpus slice, and the view owns which kinds are asked for.
    await expect(panel(page).getByText("npm test")).toBeVisible();
    await expect(panel(page).getByText("read_file")).toBeVisible();

    await panel(page).getByRole("tab", { name: "命令", exact: true }).click();
    await expect(panel(page).getByText("npm test")).toBeVisible();
    await expect(panel(page).getByText("read_file")).toBeHidden();

    await panel(page).getByRole("tab", { name: "工具", exact: true }).click();
    await expect(panel(page).getByText("read_file")).toBeVisible();
    await expect(panel(page).getByText("npm test")).toBeHidden();

    await panel(page).getByRole("tab", { name: "委派", exact: true }).click();
    await expect(panel(page).getByText("未命名委派")).toBeVisible();

    await panel(page).getByRole("tab", { name: "全部", exact: true }).click();

    // A status filter narrows without the view and the filter ever contradicting each other.
    await panel(page).getByRole("button", { name: "已失败", exact: true }).click();
    await expect(panel(page).getByText("npm test")).toBeVisible();
    await expect(panel(page).getByText("read_file")).toBeHidden();
    await panel(page).getByRole("button", { name: "已失败", exact: true }).click();

    // The detail drawer shows the record's own fields, and states the ones nobody observed.
    await panel(page).getByRole("button", { name: /npm test/ }).first().click();
    const drawer = page.getByTestId("execution-record-detail");
    await expect(drawer).toBeVisible();
    await expect(drawer).toContainText("退出码 1");
    await expect(drawer).toContainText("合并流");
    await expect(drawer).toContainText("截断");

    // A jump is offered because the record carries a trace, and it lands filtered.
    await page.getByTestId("execution-record-action-trace").click();
    await expect(page.getByRole("tab", { name: "链路" })).toHaveAttribute("aria-selected", "true");
    // Scoped to the destination panel: every mounted panel renders its own chips, and each shows
    // only the fields it consumes, so an unscoped lookup reads whichever panel is first in the DOM.
    const traces = page.getByRole("tabpanel", { name: "链路" });
    await expect(traces.getByTestId("workspace-scope-chips")).toContainText("链路");
    await expect(traces.getByTestId("workspace-scope-chips")).toContainText("跨度");
  });

  test("keeps filters and the open record when the tab is left and returned to", async ({ page }) => {
    await openTerminalHistory(page);

    await panel(page).getByRole("tab", { name: "工具", exact: true }).click();
    await panel(page).getByLabel("搜索已脱敏的记录文本").fill("read");
    await panel(page).getByRole("button", { name: /read_file/ }).first().click();
    await expect(page.getByTestId("execution-record-detail")).toBeVisible();

    await page.getByRole("tab", { name: "报告" }).click();
    await page.getByRole("tab", { name: /终端记录/ }).click();

    // The panel stays mounted, so the view, the search, and the open record survive the round trip.
    await expect(panel(page).getByRole("tab", { name: "工具", exact: true })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    await expect(panel(page).getByLabel("搜索已脱敏的记录文本")).toHaveValue("read");
    await expect(page.getByTestId("execution-record-detail")).toBeVisible();
  });

  test("keeps legacy activity in its own view and says where it came from", async ({ page }) => {
    await openTerminalHistory(page);

    await panel(page).getByRole("tab", { name: "历史活动", exact: true }).click();

    // The rows are rendered by the same list as native records, so the notice is what tells the
    // reader that one list was observed and the other reconstructed from chat messages.
    const notice = page.getByTestId("legacy-source-notice");
    await expect(notice).toBeVisible();
    await expect(notice).toContainText("并非记录下来的证据");
    // Native rows are absent here: the two corpora are never interleaved.
    await expect(panel(page).getByText("npm test")).toBeHidden();
  });

  test("states partial coverage rather than letting the rows stand for everything", async ({ page }) => {
    await openTerminalHistory(page);
    await panel(page).getByRole("tab", { name: "历史活动", exact: true }).click();

    // Legacy coverage is never complete, so its own list says so on every visit.
    await expect(page.getByTestId("legacy-source-notice")).toBeVisible();

    await panel(page).getByRole("tab", { name: "全部", exact: true }).click();
    // A search that matches nothing is a filter result, not a claim that nothing happened.
    await panel(page).getByLabel("搜索已脱敏的记录文本").fill("no-such-record-anywhere");
    await expect(panel(page)).toContainText("没有执行记录符合当前筛选条件。");
  });
});
