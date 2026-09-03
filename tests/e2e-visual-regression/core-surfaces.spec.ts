import { expect, test, type Page } from "@playwright/test";

/**
 * Tasks 21.17-21.19 -- the first real slice of design.md Decision 20's "Required screenshot
 * matrix" (10 surfaces x 2 themes x 3 locales x 5 widths). See `playwright.visual.config.ts` for
 * why this is a real `toHaveScreenshot()` baseline layer, distinct from the existing
 * `page.screenshot()`-to-`test-results/` capture tests elsewhere in this repo.
 *
 * Scope of this increment, disclosed rather than silently partial -- the full matrix is 10 x 2 x 3
 * x 5 = 300 base combinations, far too large to build, review, and keep genuinely stable (verified
 * by two consecutive live runs, per this task's own requirement) in one pass:
 *
 *   surfaces: 3 of design.md's 10 -- `sessions/default`, `runs/attention` (Mission Control),
 *             `plan/board` (Work Board). Chosen as the most heavily rebuilt and most stable
 *             destinations from this session's own work; the other 7 (`sessions/runtime-panel`,
 *             `sessions/inspector`, `runs/detail`, `loops/action-required`, `quality/comparison`,
 *             `schedules/editor`, `settings/search-result`) are a future increment.
 *   themes:   2 of 2 -- `futuristic`, `minimal`. Full theme coverage from the first slice.
 *   locales:  2 of 3 -- `zh-CN` (this app's own default locale) and `en`. `ja` is deferred, along
 *             with the 2 further registered locales (`zh-TW`, `ko`) design.md's own matrix never
 *             named either.
 *   widths:   2 of 5 -- `1600` (wide desktop) and `768` (below Work Board's own 900px compact-mode
 *             breakpoint, `work-board.tsx`'s `useMediaQuery("(max-width: 900px)")`). `1280`/`1024`/
 *             `640` are deferred.
 *
 * Total: 3 x 2 x 2 x 2 = 24 baseline images, committed under `core-surfaces.spec.ts-snapshots/`
 * (Playwright's own default `snapshotPathTemplate`).
 */

type Locale = "zh-CN" | "en";
type Theme = "futuristic" | "minimal";

const THEMES: Theme[] = ["futuristic", "minimal"];
const LOCALES: Locale[] = ["zh-CN", "en"];
const WIDTHS = [1600, 768] as const;
const VIEWPORT_HEIGHT = 900;

const copy = {
  runsButton: { "zh-CN": "运行", en: "Runs" },
  planButton: { "zh-CN": "计划", en: "Plan" },
  boardTab: { "zh-CN": "任务看板", en: "Todo Board" },
  boardHeading: { "zh-CN": "任务看板", en: "Todo Board" },
  attentionHeading: { "zh-CN": "待处理收件箱", en: "Attention inbox" },
  startChat: {
    "zh-CN": "新建或选择一个会话后开始聊天",
    en: "Create or select a session to start chatting",
  },
  newWorkItem: { "zh-CN": "新建工作项", en: "New work item" },
  fieldTitle: { "zh-CN": "标题", en: "Title" },
  fieldProject: { "zh-CN": "项目路径", en: "Project path" },
  fieldDescription: { "zh-CN": "描述", en: "Description" },
  fieldPriority: { "zh-CN": "优先级", en: "Priority" },
  createButton: { "zh-CN": "创建", en: "Create" },
  itemTitle: {
    "zh-CN": "视觉回归基线项",
    en: "Visual regression baseline item",
  },
  itemDescription: {
    "zh-CN": "用于 21.17 视觉回归矩阵的确定性夹具，内容固定不变。",
    en: "Deterministic fixture for the 21.17 visual regression matrix; content never varies.",
  },
} as const satisfies Record<string, Record<Locale, string>>;

/**
 * Kills animation/transition/caret variance beyond `playwright.visual.config.ts`'s own
 * `animations: "disabled"` (which only freezes CSS animations/transitions, not caret blink) and
 * pins a font stack known to be installed on this machine -- the same technique
 * `tests/docs/documentation-screenshots.spec.ts` already established for the same reason: font
 * rendering is the least portable part of any screenshot, and this repo has no shared helper for it
 * (that file's own constant is not exported, so this is a deliberate, matching duplication rather
 * than a new cross-directory dependency between two independent Playwright configs).
 */
const DETERMINISTIC_CSS = `
  *, *::before, *::after {
    animation-duration: 0s !important;
    caret-color: transparent !important;
    transition-duration: 0s !important;
  }
  body {
    font-family: Arial, "Microsoft YaHei UI", sans-serif !important;
  }
`;

/**
 * Sets both real theme/locale storage keys before the app's own bundle evaluates. Two keys because
 * the two most recent visual-capture precedents in this repo disagree on which one is load-bearing
 * (`session-workspace-console.visual.spec.ts` sets both and is the only one of the two that also
 * asserts `data-theme`; `docs/ui-redesign/baseline.md` separately records `vanehub.uiStyle` as
 * possibly dead). Setting both is harmless either way, and `gotoAndPrepare` below asserts the real
 * `data-theme` attribute afterward rather than trusting either claim.
 */
async function applyStyle(page: Page, theme: Theme, locale: Locale) {
  await page.addInitScript(
    ([selectedTheme, selectedLocale]) => {
      window.localStorage.setItem(
        "vanehub.appSettings",
        JSON.stringify({ applicationLanguage: selectedLocale, theme: selectedTheme }),
      );
      window.localStorage.setItem("vanehub.uiStyle", selectedTheme);
    },
    [theme, locale] as const,
  );
}

async function gotoAndPrepare(page: Page, theme: Theme, locale: Locale) {
  await applyStyle(page, theme, locale);
  await page.goto("/");
  // Read off the document rather than assumed from the init script: a style/locale that failed to
  // apply looks exactly like one that applied and changed nothing (same reasoning
  // session-workspace-console.visual.spec.ts's own comment gives for this same assertion).
  await expect(page.locator("html")).toHaveAttribute("data-theme", theme);
  await expect(page.locator("html")).toHaveAttribute("lang", locale);
  await page.addStyleTag({ content: DETERMINISTIC_CSS });
}

/** `sessions` is the app's default destination and the Web mock's session list starts empty (`let
 *  sessions: Session[] = []` in web-session-state.ts) -- the true landing state needs no setup at
 *  all, which also makes it the simplest, most deterministic of the three surfaces in this slice. */
async function openSessionsDefault(page: Page, locale: Locale) {
  await expect(page.getByText(copy.startChat[locale], { exact: true })).toBeVisible();
}

/** Mission Control's Web mock runs (web-agent-run-state.ts) are fixed, hand-authored UUID-shaped
 *  ids seeded at module load, not generated per test run -- no setup needed for real content. */
async function openRunsAttention(page: Page, locale: Locale) {
  await page.getByRole("button", { name: copy.runsButton[locale], exact: true }).click();
  await expect(page.getByTestId("mission-control")).toBeVisible();
  await expect(page.getByRole("heading", { name: copy.attentionHeading[locale] })).toBeVisible();
}

/** Unlike Mission Control, the Web mock's work items start empty (`const items = new Map()` in
 *  web-work-board-client.ts) -- an unpopulated board would exercise only the toolbar/empty-state
 *  chrome, not the redesigned card/column layout this session actually rebuilt. Creates exactly one
 *  fixed, deterministic item through the real form (mirroring tests/e2e/todo-board.spec.ts's own
 *  proven field sequence) so the baseline protects real card content. */
async function openPlanBoard(page: Page, locale: Locale) {
  await page.getByRole("button", { name: copy.planButton[locale], exact: true }).click();
  await page.getByRole("tab", { name: copy.boardTab[locale] }).click();
  await expect(page.getByRole("heading", { name: copy.boardHeading[locale] })).toBeVisible();

  await page.getByRole("button", { name: copy.newWorkItem[locale] }).click();
  await page.getByLabel(copy.fieldTitle[locale]).fill(copy.itemTitle[locale]);
  await page.getByLabel(copy.fieldProject[locale]).fill("D:\\visual-regression-fixture");
  await page.getByLabel(copy.fieldDescription[locale]).fill(copy.itemDescription[locale]);
  await page.getByLabel(copy.fieldPriority[locale], { exact: true }).selectOption("high");
  await page.getByRole("button", { name: copy.createButton[locale], exact: true }).click();
  await expect(page.getByText(copy.itemTitle[locale], { exact: true })).toBeVisible();
}

const SURFACES: Record<string, (page: Page, locale: Locale) => Promise<void>> = {
  "sessions-default": openSessionsDefault,
  "runs-attention": openRunsAttention,
  "plan-board": openPlanBoard,
};

test.describe.configure({ timeout: 120_000 });

for (const [surfaceId, open] of Object.entries(SURFACES)) {
  for (const theme of THEMES) {
    for (const locale of LOCALES) {
      for (const width of WIDTHS) {
        test(`${surfaceId} ${theme} ${locale} ${width}`, async ({ page }) => {
          await page.setViewportSize({ width, height: VIEWPORT_HEIGHT });
          await gotoAndPrepare(page, theme, locale);
          await open(page, locale);
          await expect(page).toHaveScreenshot(
            `${surfaceId}-${theme}-${locale}-${width}.png`,
            { fullPage: true },
          );
        });
      }
    }
  }
}
