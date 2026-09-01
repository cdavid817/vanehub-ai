import { expect, test, type Page } from "@playwright/test";

/**
 * The evidence console rendered, rather than reasoned about.
 *
 * Everything up to here was checked in jsdom, which has no layout engine — it can read the classes
 * that decide a box but never the box. The three failures that only exist once something is laid
 * out are the ones this file is for: a panel that fits at 1440 and spills at 390, a style whose
 * tokens resolve to nothing in one theme and to a colour in the other, and a label that fits in
 * Chinese and pushes its row off the screen in Korean.
 *
 * The states are not staged. The Web fixtures already carry a running record, a failed
 * verification, and two kinds of partial coverage, because those are the states the console was
 * built to tell apart — so these read what the app actually renders rather than a scenario
 * assembled to look interesting.
 */

const VARIANTS = [
  { name: "futuristic-desktop", theme: "futuristic", width: 1440, height: 900 },
  { name: "minimal-desktop", theme: "minimal", width: 1440, height: 900 },
  { name: "futuristic-narrow", theme: "futuristic", width: 390, height: 844 },
  { name: "minimal-narrow", theme: "minimal", width: 390, height: 844 },
] as const;

test.describe.configure({ timeout: 120_000 });

async function useStyle(page: Page, theme: string, language: string) {
  await page.addInitScript(
    ([style, locale]) => {
      localStorage.setItem(
        "vanehub.appSettings",
        JSON.stringify({ applicationLanguage: locale, theme: style }),
      );
      localStorage.setItem("vanehub.uiStyle", style);
    },
    [theme, language] as const,
  );
}

/**
 * The shared helper reads zh-CN labels, and one case here deliberately runs in Korean.
 *
 * Copied rather than parameterized upstream: every other spec calls that helper, and widening its
 * selectors to match five locales would widen them for all of them.
 */
/**
 * Task 11.3-11.7 turned the single-screen create-session dialog into a 4-step wizard: Step 1
 * (mode) -> Step 2 (participant) -> Step 3 (workspace, the project path field) -> Step 4 (review,
 * the session name field — the old single-screen dialog's own "Identity" section landed there).
 * `copy.next` names each locale's "Next" button label alongside the pre-existing labels.
 */
async function createSession(page: Page, title: string, copy: {
  create: string;
  newSession: string;
  next: string;
  start: RegExp;
  terminal: string;
}) {
  await page.getByRole("button", { name: copy.start }).click();
  const nextButton = page.getByRole("button", { name: copy.next });
  await nextButton.click(); // Step 1 (mode) -> Step 2, defaults left as-is.
  await nextButton.click(); // Step 2 (participant) -> Step 3, defaults left as-is.

  const projectPath = page.getByPlaceholder(/code.*project/);
  await projectPath.fill("D:\\example-workspace");
  await projectPath.press("Tab");
  await expect(nextButton).toBeEnabled({ timeout: 10_000 });
  await nextButton.click(); // Step 3 -> Step 4.

  await page.getByPlaceholder(copy.newSession).fill(title);
  const createButton = page.getByRole("button", { name: copy.create, exact: true });
  await expect(createButton).toBeEnabled();
  await createButton.click();
  await expect(page.getByRole("textbox", { name: copy.terminal })).toBeEnabled();
}

const ZH = {
  create: "创建",
  newSession: "新会话",
  next: "下一步",
  start: /新建/,
  terminal: "Terminal input",
};

/** Nothing may be reachable only by scrolling sideways. */
async function expectNoHorizontalSpill(page: Page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth)).toBe(
    true,
  );
}

function workspaceTabs(page: Page) {
  return page.getByRole("tablist", { name: "会话工作区" });
}

for (const variant of VARIANTS) {
  test(`evidence console ${variant.name}`, async ({ page }, testInfo) => {
    await useStyle(page, variant.theme, "zh-CN");
    await page.setViewportSize({ width: variant.width, height: variant.height });
    await page.goto("/");
    await createSession(page, `证据控制台 ${variant.name}`, ZH);

    // The theme is read off the document rather than assumed from the init script, because a style
    // that failed to apply looks exactly like one that applied and changed nothing.
    await expect(page.locator("html")).toHaveAttribute("data-theme", variant.theme);

    // Terminal History lives in the closed-by-default Runtime Panel now, not the primary 4-tab
    // workspace (§8's split of the old flat 9-tab workspace into 4 primary + 4 Runtime Panel
    // surfaces) -- open it first, then find the tab inside its own tablist.
    await page.getByRole("button", { name: "运行时面板" }).click();
    await page.getByRole("tablist", { name: "运行时面板" }).getByRole("tab", { name: "终端记录" }).click();
    // Unlike the primary tabs, the Runtime Panel's own tab content (`RuntimeSurfaceShell` in
    // session-runtime-panel.tsx) never sets a `session-tab-panel-*` id -- scope by ARIA role/name.
    const records = page.getByRole("tabpanel", { name: "终端记录" });

    // Live, failed, and incomplete in one view. These are the console's whole reason for existing:
    // a run still going, a run that finished badly, and a row whose start was never observed. A
    // surface that renders them identically has thrown away the distinction it was built to draw.
    await expect(records.getByText("运行中").first()).toBeVisible();
    await expect(records.getByText("已失败").first()).toBeVisible();
    await expect(records.getByText("不完整").first()).toBeVisible();

    await expectNoHorizontalSpill(page);
    // The panel's own scroll, separately. The document can fit while a pane inside it clips its
    // right-hand column, and the reader loses whatever was in that column either way.
    const panelSpill = await records.evaluate(
      (element) => element.scrollWidth > element.clientWidth + 1,
    );
    expect(panelSpill).toBe(false);

    // The diff is the surface most likely to spill, and the only one that is *supposed* to be
    // wider than a phone: its rows carry a 520px minimum so code stays legible rather than
    // reflowing. That minimum is only safe because a scrollable ancestor absorbs it, which is
    // exactly the kind of pairing that survives a refactor by accident and breaks by accident too.
    await workspaceTabs(page).getByRole("tab", { name: "变更" }).click();
    const changes = page.locator('[id^="session-tab-panel-"]:not(.hidden)');
    // Read with the sign attached, which is the point: this is the only place the marker is
    // checked against a real layout engine, and a `+` that renders outside its row or under a
    // sibling looks correct in jsdom and wrong on screen.
    await expect(changes.getByText('-export const runtime = "web";')).toBeVisible();
    await expect(changes.getByText('+export const runtime = "web-mock";')).toBeVisible();
    await expectNoHorizontalSpill(page);

    // A diff row is wider than a phone on purpose — code that reflows is code nobody can read — so
    // the rule is not "nothing is wide" but "whatever is wide can be scrolled to". Read off the
    // computed style rather than the class list, because the class is what someone deletes and the
    // computed style is what the reader actually gets.
    const overflowX = await changes.locator("section").first().evaluate((element) =>
      element.scrollWidth > element.clientWidth
        ? getComputedStyle(element).overflowX
        : "fits",
    );
    expect(["auto", "scroll", "fits"]).toContain(overflowX);

    await workspaceTabs(page).getByRole("tab", { name: "报告" }).click();
    const report = page.locator('[id^="session-tab-panel-"]:not(.hidden)');
    // Partial coverage, which the report carries per section rather than per report — a report is
    // worth reading while one of its sections cannot be substantiated, and the badge is how a
    // reader learns which section that is instead of discounting the whole page.
    await expect(report.getByText(/部分/).first()).toBeVisible();
    await expectNoHorizontalSpill(page);

    await page.screenshot({ fullPage: true, path: testInfo.outputPath(`${variant.name}.png`) });
  });
}

test("holds the narrow layout under the longest localized labels", async ({ page }, testInfo) => {
  // Korean and Japanese run longer than Chinese for the same label, and the narrow viewport is
  // where that stops being cosmetic. Checked at 390 in Korean because that is the pairing with the
  // least room, not because Korean is special.
  await useStyle(page, "minimal", "ko");
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/");
  await createSession(page, "긴 라벨", {
    create: "만들기",
    newSession: "새 세션",
    next: "다음",
    start: /새로운/,
    terminal: "Terminal input",
  });

  await expect(page.locator("html")).toHaveAttribute("lang", "ko");
  await expectNoHorizontalSpill(page);

  // Still four (§8 split the old flat nine-tab workspace into 4 primary + 4 Runtime Panel
  // surfaces -- this tablist is the primary one), and still reachable. The failure worth catching
  // here is the quiet one: a tab strip that runs out of room and drops its last entries rather
  // than scrolling, which leaves a reader with a workspace whose Report tab does not appear to
  // exist in their language.
  const tabs = page.getByRole("tablist", { name: "세션 작업공간" });
  await expect(tabs.getByRole("tab")).toHaveCount(4);
  await tabs.getByRole("tab", { name: "신고" }).click();
  await expect(tabs.getByRole("tab", { name: "신고" })).toHaveAttribute("aria-selected", "true");
  await expectNoHorizontalSpill(page);

  const panel = page.locator('[id^="session-tab-panel-"]:not(.hidden)');
  expect(await panel.evaluate((element) => element.scrollWidth > element.clientWidth + 1)).toBe(
    false,
  );

  await page.screenshot({ fullPage: true, path: testInfo.outputPath("ko-narrow.png") });
});
