import { expect, test } from "@playwright/test";
import { createSession } from "./session-helpers";

/**
 * What a reader is told when a search did not see the whole workspace.
 *
 * The component tests cover the wording; what only a browser can show is that the wording survives
 * the whole path — the toolbar button, the panel, the service boundary, the mock adapter, and the
 * coverage that comes back through it. A notice that is correct in a unit test and unreachable in
 * the application is a notice nobody reads.
 *
 * The ceilings come from the address bar because the mock's defaults are generous enough that the
 * fixture workspace is genuinely complete. The alternative — a fixture with thousands of invented
 * files in it — would make every unrelated checkout carry them.
 */
async function openContentSearch(
  page: Parameters<typeof createSession>[0],
  limits: string,
  title: string,
) {
  await page.goto(`/?workspaceSearchLimits=${limits}`);
  await createSession(page, title);
  await page.getByRole("tab", { name: "文件" }).click();
  await page.getByRole("button", { name: "在文件中搜索" }).click();
  return page.getByRole("combobox", { name: "在此工作区中查找文本" });
}

test.describe("session workspace content search", () => {
  test("says a search matched nothing only when it looked everywhere", async ({ page }) => {
    const input = await openContentSearch(page, "maxFiles:64", "内容搜索完整性测试");

    await input.fill("zzz-nothing-matches-this");

    // Complete and empty: the string is not in the workspace, which is a claim this search is
    // entitled to make.
    await expect(page.getByText("没有匹配的行。")).toBeVisible();
  });

  test("does not claim certainty when a budget stopped the search", async ({ page }) => {
    const input = await openContentSearch(page, "maxFiles:1", "内容搜索预算测试");

    await input.fill("zzz-nothing-matches-this");

    // The distinction the whole coverage contract exists for. "No matches" here would tell the
    // reader the text is not in their workspace, about files nothing opened — and the way they act
    // on that is to conclude it does not exist and move on.
    await expect(page.getByText("在已搜索到的那部分工作区中没有匹配。")).toBeVisible();
    await expect(page.getByText("此工作区有一部分未被搜索。")).toBeVisible();
  });

  test("reports a refused search as a queue rather than as an empty workspace", async ({ page }) => {
    const input = await openContentSearch(page, "maxConcurrent:0", "内容搜索繁忙测试");

    await input.fill("needle");

    // A refusal is the one stop a reader can act on directly: wait and ask again. Folding it into
    // "no matches" turns a queue into a fact about their files.
    await expect(page.getByText("此工作区不支持内容搜索。")).toBeVisible();
    await expect(page.getByText(/同时进行的搜索过多/)).toBeVisible();
  });
});
