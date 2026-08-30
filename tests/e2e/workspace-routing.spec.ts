import { expect, test, type Page } from "@playwright/test";
import { createSession } from "./session-helpers";

async function openWorkspace(page: Page) {
  await page.goto("/");
  await expect(page).toHaveURL(/\/workspace\/sessions/);
}

/** Loops is a Runs section now (runs-destination.tsx), not its own activity-bar entry. */
async function openLoopsFromActivityBar(page: Page) {
  await page.getByRole("button", { name: "运行", exact: true }).click();
  await page.getByRole("tab", { name: "循环工程" }).click();
}

test.describe("workspace routing", () => {
  test("addresses every destination and restores them with Back and Forward", async ({ page }) => {
    await openWorkspace(page);

    await openLoopsFromActivityBar(page);
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    // Board is a Plan section now (plan-destination.tsx) and its own default, so this is one
    // click, not two.
    await page.getByRole("button", { name: "计划", exact: true }).click();
    await expect(page).toHaveURL(/\/workspace\/plan\/board$/);

    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    await expect(page.getByTestId("loop-center")).toBeVisible();
    // One more than before: reaching Loops is now itself two history entries (Runs' own default
    // section, then the Loops tab within it), where it used to be a single destination click.
    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/runs\/attention$/);
    await page.goBack();
    await expect(page).toHaveURL(/\/workspace\/sessions$/);
    await expect(page.getByTestId("session-sidebar")).toBeVisible();

    // Forward retraces the exact same stack, not just the URL bar — content restores too.
    await page.goForward();
    await expect(page).toHaveURL(/\/workspace\/runs\/attention$/);
    await page.goForward();
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    await expect(page.getByTestId("loop-center")).toBeVisible();
    await page.goForward();
    await expect(page).toHaveURL(/\/workspace\/plan\/board$/);
  });

  test("opens a destination directly from its URL", async ({ page }) => {
    await page.goto("/workspace/runs/loops");
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
    await expect(page.getByTestId("loop-center")).toBeVisible();
    // A deep link has never been "visited" by a click, which is what proves this renders from
    // the route itself rather than from click-driven state.
    await expect(page.getByText("暂无循环定义")).toBeVisible();
  });

  test("falls back to sessions for an unknown destination", async ({ page }) => {
    await page.goto("/workspace/nonsense");
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
  });

  test("redirects every pre-redesign flat destination URL to its new home, in the URL bar itself", async ({ page }) => {
    await page.goto("/workspace/loops");
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    await expect(page.getByTestId("loop-center")).toBeVisible();

    await page.goto("/workspace/mission-control");
    await expect(page).toHaveURL(/\/workspace\/runs\/attention$/);

    await page.goto("/workspace/work-board");
    await expect(page).toHaveURL(/\/workspace\/plan\/board$/);

    await page.goto("/workspace/goals");
    await expect(page).toHaveURL(/\/workspace\/plan\/goals$/);

    await page.goto("/workspace/evaluations");
    await expect(page).toHaveURL(/\/workspace\/quality\/evaluations$/);
  });

  test("explains a legacy redirect once, then stays quiet on the next one", async ({ page }) => {
    await page.goto("/workspace/loops");
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    await expect(page.getByText("已迁移")).toBeVisible();

    await page.goto("/workspace/work-board");
    await expect(page).toHaveURL(/\/workspace\/plan\/board$/);
    await expect(page.getByText("已迁移")).toHaveCount(0);
  });

  /**
   * Sessions is the one destination the "stays mounted" guarantee names: main-layout.tsx keeps
   * its route outlet mounted with a CSS `hidden` toggle rather than conditionally rendering it,
   * specifically so switching away and back does not reset it. Runs/Plan/Quality/Projects are
   * plainly conditionally rendered (`location.destination === "runs" ? <RunsDestination /> :
   * null`) and do not get this treatment — a deliberate scope boundary from task 4.3 (see
   * tasks.md), not an oversight this spec should paper over.
   *
   * Session identity and an in-progress composer draft both survive. The draft needed a real fix,
   * not just the `hidden` toggle: `DestinationLayout`'s `ResizeObserver`-driven tier classification
   * (use-layout-tier.ts) reports a momentary zero width while the whole subtree is `display: none`
   * and reclassifies to "narrow" and back once it reappears, and `DestinationLayoutBody` used to
   * key whether it wrapped `main` in a `SplitPane` at all on that tier — a reclassify-and-back
   * remounted it exactly like the open/close toggle this window's SplitPane fix already covered.
   * Fixed by wrapping whenever a region exists at all, independent of tier, so only `SplitPane`'s
   * own `open` prop (already tier-aware) governs presentation.
   */
  test("preserves session state, including an in-progress draft, across navigating to another destination and back", async ({ page }) => {
    await openWorkspace(page);
    await createSession(page, "路由保留会话");
    await page.getByRole("textbox", { name: "工作区命令输入" }).fill("draft survives a destination round trip");

    await openLoopsFromActivityBar(page);
    await expect(page.getByTestId("loop-center")).toBeVisible();

    await page.getByRole("button", { name: "折叠会话栏" }).click();
    await expect(page.getByTestId("session-conversation-header").getByText("路由保留会话")).toBeVisible();
    await expect(page.getByRole("textbox", { name: "工作区命令输入" })).toHaveValue("draft survives a destination round trip");
  });

  test("puts the active session in the URL", async ({ page }) => {
    await openWorkspace(page);
    await createSession(page, "路由会话");

    await expect(page).toHaveURL(/\/workspace\/sessions\/.+/);
    await expect(page.getByTestId("session-conversation-header").getByText("路由会话")).toBeVisible();
  });

  /**
   * Only the location is asserted after the reload. Web/mock session state lives in module
   * memory and does not survive a full document load, so asserting the session would test the
   * mock's lifetime rather than the restore behaviour.
   */
  test("resumes the previous destination on relaunch", async ({ page }) => {
    await openWorkspace(page);
    await openLoopsFromActivityBar(page);
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    // The location is recorded from an effect, so reloading on the URL alone races the write.
    await expect(page.getByTestId("loop-center")).toBeVisible();

    await page.goto("/");
    await expect(page).toHaveURL(/\/workspace\/runs\/loops$/);
    await expect(page.getByTestId("loop-center")).toBeVisible();
  });

  /**
   * The warning notification this fires alongside the fallback (4.10) needs a real session
   * already in the list to fire at all — `useWorkspaceSessionRoute` treats an empty list as a
   * deep link still loading, not a genuine not-found. That precondition can't survive a
   * `page.goto()` here: Web/mock session state lives in module memory and does not survive a full
   * document load (same constraint documented on "resumes the previous destination on relaunch",
   * above) — a session created just before this navigation would already be gone by the time it
   * runs. Proven instead at the unit level, where the precondition is just a prop, not a live
   * backend: `use-workspace-session-route.test.tsx`'s "explains why, not just where...".
   */
  test("falls back to the session list for a session that does not exist", async ({ page }) => {
    await page.goto("/workspace/sessions/session-does-not-exist");
    await expect(page.getByTestId("session-sidebar")).toBeVisible();
    await expect(page.getByTestId("workspace-frame")).toBeVisible();
  });

  test("expresses session creation as a route", async ({ page }) => {
    await page.goto("/workspace/sessions/new");
    await expect(page.getByRole("dialog", { name: "创建会话" })).toBeVisible();

    await page.getByRole("button", { name: "取消", exact: true }).click();
    await expect(page.getByRole("dialog", { name: "创建会话" })).toHaveCount(0);
    await expect(page).toHaveURL(/\/workspace\/sessions/);
  });
});
