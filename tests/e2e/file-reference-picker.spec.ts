import { expect, test, type Page } from "@playwright/test";

/**
 * The one thing unit tests structurally cannot reach: selecting a line range across a
 * scroll in the virtualized preview. Component tests mock the virtual list and render
 * every row, because the virtualizer measures a container jsdom reports as zero-sized —
 * so the windowing itself is only exercised here.
 *
 * Everything else about the preview (click order, whole-file action, unreadable states,
 * focus) is covered by component tests and is deliberately not duplicated.
 *
 * A multi-seat session is the cheapest route to the chat composer in the mock adapter —
 * a OnePiece API session needs a whole provider configured first.
 */
async function createComposerSession(page: Page, title: string) {
  // A cold Vite compile of this route exceeds the default navigation budget on a loaded
  // machine; the wait is for the dev server, not for anything this change owns.
  await page.goto("/", { timeout: 90_000 });
  await page.getByRole("button", { name: /新建/ }).click();
  const projectPath = page.getByPlaceholder(/code.*project/);
  await expect(async () => {
    await projectPath.fill("D:\\example-workspace");
    await projectPath.press("Tab");
    await expect(page.getByRole("button", { name: "创建", exact: true })).toBeEnabled({ timeout: 1_000 });
  }).toPass({ timeout: 10_000 });

  await page.getByRole("button", { name: /多个 Agent 在同一会话里协作/ }).click();
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
  await page.getByRole("button", { name: "创建", exact: true }).click();
  await expect(page.getByTestId("wechat-style-composer")).toBeVisible();
}

test.describe("file reference picker", () => {
  test("selects a line range across a scroll and keeps the numbers consistent", async ({ page }) => {
    await createComposerSession(page, "引用选行会话");

    await page.getByPlaceholder(/输入指令/).fill("@long-module");
    await page.getByRole("button", { name: "src/long-module.ts" }).click();
    await expect(page.getByTestId("preview-line-1")).toBeVisible();

    // Anchor near the top, then reach a line far outside the initial window. That line is
    // not in the DOM until the list scrolls, which is exactly what this test is for.
    await page.getByTestId("preview-line-3").click();
    const list = page.getByTestId("file-preview-lines");
    await list.evaluate((element) => { element.scrollTop = element.scrollHeight; });

    const farLine = page.getByTestId("preview-line-380");
    await expect(farLine).toBeVisible();
    // The fixture writes its own line number into each line, so this proves the preview
    // labels line 380 with the content that really is on line 380.
    await expect(farLine).toContainText("value380");

    await farLine.click();
    await expect(page.getByText("已选第 3-380 行")).toBeVisible();
    await page.getByRole("button", { name: "引用所选范围" }).click();

    // The chip must agree with what was picked — that number is what the prompt injects.
    const chips = page.getByTestId("composer-file-references");
    await expect(chips.getByText("L3-380")).toBeVisible();
  });

  test("drags a file out of the Files tab onto the composer", async ({ page }) => {
    await createComposerSession(page, "拖拽引用会话");

    await page.getByRole("tab", { name: "文件" }).click();
    // A root-level file, so the tree needs no expanding first.
    const fileRow = page.getByRole("tabpanel", { name: "文件" }).getByRole("button", { name: /README\.md/ }).first();
    await expect(fileRow).toBeVisible();
    expect(await fileRow.getAttribute("draggable")).toBe("true");

    // Playwright's dragTo drives the mouse, which HTML5 drag-and-drop does not follow
    // reliably; dispatching the events with a shared DataTransfer is the supported way to
    // exercise a real drag source and drop target.
    const transfer = await page.evaluateHandle(() => new DataTransfer());
    await fileRow.dispatchEvent("dragstart", { dataTransfer: transfer });
    const composer = page.getByTestId("wechat-style-composer");
    await composer.dispatchEvent("dragenter", { dataTransfer: transfer });
    await composer.dispatchEvent("drop", { dataTransfer: transfer });

    // Attached, not visible: the Files tab is what the viewport is showing right now, so
    // this asserts the reference exists rather than that it is on screen.
    await expect(page.getByTestId("composer-file-references").getByText("README.md")).toHaveCount(1);
  });
});
