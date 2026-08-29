/**
 * Throwaway evidence capture for `redesign-unified-workbench-ui` tasks 0.7 and 0.8.
 *
 * This is pure measurement: it does not touch any application source file. It boots the real
 * Vite dev server (same `vite.config.ts` as `npm run dev`) on a freshly allocated loopback port,
 * drives the Web/mock adapter with plain `playwright` (not the `@playwright/test` runner, since
 * this is a one-off script rather than a suite), and for every major "before" surface:
 *
 *   - screenshots it at 1600/1280/1024/768/640 widths (fixed 900px height), and
 *   - reads DOM node count, a rough interval/observer/listener footprint, and
 *     `performance.getEntriesByType('resource').length`, both while the surface is active and
 *     again after navigating away, so a later reader can see whether a destination that is now
 *     "hidden but mounted" (main-layout.tsx keeps Board/Goals/Mission Control/Loop/Evaluation and
 *     every visited Settings page mounted with a CSS `hidden` toggle rather than unmounting them)
 *     keeps accumulating work in the background.
 *
 * The whole capture runs inside ONE browser tab / ONE document (a single `page.goto` at the very
 * start, everything else is in-app client-side navigation). That is deliberate: the counters are
 * window-level and cumulative, and only staying on one document lets "after navigating away" mean
 * anything. The one documented exception is the Settings route: it is a *separate* top-level
 * React Router branch (`/workspace/*` vs `/settings` in `src/App.tsx`), so crossing that boundary
 * really does unmount and remount the whole workspace tree, even though the document/window (and
 * therefore the counters) survive.
 *
 * Usage:
 *   node scripts/ui-redesign/capture-baseline.mjs
 *
 * Env overrides:
 *   VANEHUB_BASELINE_PORT      - explicit loopback port for the throwaway dev server (default: OS-assigned free port)
 *   VANEHUB_BASELINE_HEADLESS  - set to "0" to run the capture with a visible browser window
 */
import { mkdir, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { createServer as createViteServer } from "vite";
import { allocateScreenshotPort } from "../docs-screenshot-port.mjs";

const repoRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");
const outDir = path.join(repoRoot, "docs", "ui-redesign", "screenshots", "baseline");
const widths = [1600, 1280, 1024, 768, 640];
const viewportHeight = 900;
const metricsViewport = { width: 1280, height: viewportHeight };
const headless = process.env.VANEHUB_BASELINE_HEADLESS !== "0";
const locale = "zh-CN";
const theme = "minimal";

const sessionTabs = [
  { id: "chat", label: "工作区" },
  { id: "changes", label: "变更" },
  { id: "documents", label: "文档" },
  { id: "files", label: "文件" },
  { id: "terminal", label: "终端记录" },
  { id: "shell", label: "Shell" },
  { id: "logs", label: "日志" },
  { id: "traces", label: "链路" },
  { id: "report", label: "报告" },
];

function log(message) {
  process.stdout.write(`[baseline] ${message}\n`);
}

// Runs inside the page. Must be fully self-contained (no closures over outer script state) since
// Playwright serializes it as a fresh function for `context.addInitScript`.
function installBaselineCounters() {
  const counters = {
    intervalsStarted: 0,
    intervalsCleared: 0,
    resizeObserversCreated: 0,
    intersectionObserversCreated: 0,
    listenersAdded: 0,
    listenersRemoved: 0,
  };
  window.__baselineCounters = counters;

  // Chromium's default Resource Timing buffer holds only 250 entries and silently stops recording
  // once full rather than growing; a Vite dev server serving many unbundled ES modules blows past
  // that during the very first page load, which would otherwise flatline this proxy at 250 for
  // every later surface regardless of real network activity.
  if (typeof performance.setResourceTimingBufferSize === "function") {
    performance.setResourceTimingBufferSize(20_000);
  }

  const originalSetInterval = window.setInterval.bind(window);
  window.setInterval = function patchedSetInterval(...args) {
    counters.intervalsStarted += 1;
    return originalSetInterval(...args);
  };

  const originalClearInterval = window.clearInterval.bind(window);
  window.clearInterval = function patchedClearInterval(...args) {
    counters.intervalsCleared += 1;
    return originalClearInterval(...args);
  };

  if (typeof window.ResizeObserver === "function") {
    const OriginalResizeObserver = window.ResizeObserver;
    window.ResizeObserver = class extends OriginalResizeObserver {
      constructor(...args) {
        super(...args);
        counters.resizeObserversCreated += 1;
      }
    };
  }

  if (typeof window.IntersectionObserver === "function") {
    const OriginalIntersectionObserver = window.IntersectionObserver;
    window.IntersectionObserver = class extends OriginalIntersectionObserver {
      constructor(...args) {
        super(...args);
        counters.intersectionObserversCreated += 1;
      }
    };
  }

  const originalAddEventListener = EventTarget.prototype.addEventListener;
  EventTarget.prototype.addEventListener = function patchedAddEventListener(...args) {
    counters.listenersAdded += 1;
    return originalAddEventListener.apply(this, args);
  };

  const originalRemoveEventListener = EventTarget.prototype.removeEventListener;
  EventTarget.prototype.removeEventListener = function patchedRemoveEventListener(...args) {
    counters.listenersRemoved += 1;
    return originalRemoveEventListener.apply(this, args);
  };
}

// Also self-contained; installed before every navigation so the app boots in a fixed locale/theme
// regardless of host machine locale (see AGENTS.md on why native/zh-CN selectors must not drift).
function installLocalePreference() {
  localStorage.clear();
  localStorage.setItem(
    "vanehub.appSettings",
    JSON.stringify({ applicationLanguage: "zh-CN", fontSize: "medium", theme: "minimal" }),
  );
}

async function waitForAttributeMatch(locator, attribute, regex, timeout) {
  const deadline = Date.now() + timeout;
  for (;;) {
    const value = await locator.getAttribute(attribute);
    if (value && regex.test(value)) return value;
    if (Date.now() > deadline) {
      throw new Error(`Timed out waiting for [${attribute}] to match ${regex} (last value: ${value})`);
    }
    await new Promise((resolve) => setTimeout(resolve, 150));
  }
}

/** Loads the app exactly once. Every later "navigation" in this script is in-app client routing. */
async function visitRoot(page, baseURL) {
  await page.goto(baseURL, { waitUntil: "domcontentloaded" });
  const root = page.locator("#root");
  for (let attempt = 0; attempt < 2; attempt += 1) {
    // Generous on purpose: the first request against a freshly started dev server can still need
    // to finish esbuild dependency pre-bundling even with a warm `.vite/deps` cache, and this
    // machine may be running other concurrent sessions competing for CPU.
    await waitForAttributeMatch(root, "data-vanehub-bootstrap", /^(failed|ready)$/, 60_000);
    const value = await root.getAttribute("data-vanehub-bootstrap");
    if (value === "ready") return;
    if (attempt === 1) throw new Error("The application failed to bootstrap after one recovery reload.");
    await page.reload({ waitUntil: "domcontentloaded" });
  }
}

async function waitHeading(page, level, name, timeout = 20_000) {
  await page.getByRole("heading", { level, name, exact: true }).first().waitFor({ state: "visible", timeout });
}

async function settle(page, ms = 400) {
  await page.waitForTimeout(ms);
}

async function takeScreenshots(page, name, options = {}) {
  const files = [];
  for (const width of widths) {
    await page.setViewportSize({ width, height: viewportHeight });
    await page.waitForTimeout(150);
    if (options.beforeEachWidth) await options.beforeEachWidth(width);
    const target = options.locator ? options.locator() : page;
    const buffer = await target.screenshot({ type: "png" });
    const fileName = `${name}-${width}.png`;
    await writeFile(path.join(outDir, fileName), buffer);
    files.push({
      width,
      file: `docs/ui-redesign/screenshots/baseline/${fileName}`,
      bytes: buffer.length,
    });
  }
  return files;
}

async function readMetrics(page) {
  await page.setViewportSize(metricsViewport);
  await page.waitForTimeout(150);
  return page.evaluate(() => {
    const counters = window.__baselineCounters ?? {
      intervalsStarted: 0,
      intervalsCleared: 0,
      resizeObserversCreated: 0,
      intersectionObserversCreated: 0,
      listenersAdded: 0,
      listenersRemoved: 0,
    };
    return {
      domNodeCount: document.querySelectorAll("*").length,
      resourceEntryCount: performance.getEntriesByType("resource").length,
      intervalsNet: counters.intervalsStarted - counters.intervalsCleared,
      observersCreated: counters.resizeObserversCreated + counters.intersectionObserversCreated,
      listenersNet: counters.listenersAdded - counters.listenersRemoved,
      raw: { ...counters },
    };
  });
}

/**
 * Runs one surface capture, recording status regardless of outcome. A failing surface must not
 * abort the rest of the run: each entry stands on its own so the final report can say exactly
 * which surfaces succeeded, which were skipped (and why), and which errored (and how).
 */
async function step(results, name, fn) {
  const entry = { name, status: "running", error: null, note: null, screenshots: [], active: null, after: null };
  results.push(entry);
  log(`capturing "${name}"...`);
  try {
    await fn(entry);
    entry.status = "ok";
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    if (message.startsWith("SKIPPED:")) {
      entry.status = "skipped";
      entry.note = message.slice("SKIPPED:".length).trim();
      log(`  skipped: ${entry.note}`);
    } else {
      entry.status = "error";
      entry.error = message;
      log(`  FAILED: ${message}`);
    }
  }
  return entry;
}

function requireSession(sessionReady) {
  if (!sessionReady) throw new Error("SKIPPED: the representative session was not created earlier in this run.");
}

async function switchSessionTab(page, label) {
  const shell = page.locator("main").first();
  await shell.getByRole("tab", { name: label, exact: true }).click();
  await shell.getByRole("tabpanel", { name: label, exact: true }).waitFor({ state: "visible", timeout: 20_000 });
}

async function createRepresentativeSession(page) {
  const newButton = page.getByRole("button", { name: "新建", exact: true });
  await newButton.waitFor({ state: "visible", timeout: 15_000 });
  await newButton.click();
  const dialog = page.getByRole("dialog");
  await dialog.waitFor({ state: "visible", timeout: 15_000 });
  await dialog.locator('input[placeholder*="code"]').fill("D:\\VaneHub-Demo");
  await dialog.getByPlaceholder("新会话", { exact: true }).fill("UI Redesign Baseline");
  await dialog.getByRole("button", { name: "创建", exact: true }).click();
  await dialog.waitFor({ state: "detached", timeout: 20_000 });
  const shell = page.locator("main").first();
  await shell.getByRole("tablist", { name: "会话工作区", exact: true }).waitFor({ state: "visible", timeout: 20_000 });
}

/**
 * Invokes the DOM `.click()` method directly, bypassing pointer hit-testing entirely.
 *
 * Needed because of a real layout defect: at some widths the expanded info panel visually sits
 * over the conversation header, so a coordinate-based click (even Playwright's `force: true`,
 * which skips its own actionability check but still dispatches at that coordinate) lands on the
 * panel instead of the button underneath it. Recorded in `docs/ui-redesign/baseline.md` as a
 * finding worth the redesign's attention, not silently worked around.
 */
async function clickTestId(page, testId) {
  await page.evaluate((id) => {
    const el = document.querySelector(`[data-testid="${id}"]`);
    if (!(el instanceof HTMLElement)) throw new Error(`No clickable element with data-testid="${id}"`);
    el.click();
  }, testId);
}

async function setInfoPanelExpanded(page, expand) {
  await clickTestId(page, "conversation-overflow-trigger");
  const item = page.getByTestId("toggle-info-panel");
  await item.waitFor({ state: "visible", timeout: 5_000 });
  const expanded = (await item.getAttribute("aria-checked")) === "true";
  if (expanded !== expand) await clickTestId(page, "toggle-info-panel");
  else await page.keyboard.press("Escape");
}

async function openScheduledTasksDialog(page) {
  await page.locator('button[aria-haspopup="dialog"][aria-label="定时任务"]').click();
  const dialog = page.getByRole("dialog");
  await dialog.getByRole("heading", { name: "定时任务", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  return dialog;
}

async function createRepresentativeScheduledTask(dialog) {
  await dialog.getByLabel("任务名称", { exact: true }).fill("UI Redesign Baseline Task");
  await dialog.getByLabel("任务内容", { exact: true }).fill("Summarize repository activity for the UI redesign baseline.");
  await dialog.getByRole("button", { name: "创建任务", exact: true }).click();
  await dialog.getByText("UI Redesign Baseline Task", { exact: true }).first().waitFor({ state: "visible", timeout: 15_000 });
}

async function createRepresentativeGoal(shell) {
  await shell.getByRole("button", { name: "新建目标", exact: true }).click();
  await shell.getByLabel("标题", { exact: true }).fill("UI Redesign Baseline Goal");
  await shell.getByRole("button", { name: "创建", exact: true }).click();
  await shell.getByText("UI Redesign Baseline Goal", { exact: true }).first().waitFor({ state: "visible", timeout: 10_000 });
}

async function runRepresentativeEvaluation(shell) {
  await shell.getByTestId("evaluation-run").click();
  await shell.getByTestId("evaluation-row").first().waitFor({ state: "visible", timeout: 10_000 });
}

async function captureSequence(page, results) {
  let sessionReady = false;

  // 1. Sessions destination, default view (nothing created yet).
  await step(results, "sessions-list", async (entry) => {
    await page.getByRole("button", { name: "新建", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
    entry.screenshots = await takeScreenshots(page, "sessions-list");
    entry.active = await readMetrics(page);
    // Navigate away and back so "after" reflects a real destination switch. Work Board is
    // captured for real later (with a session and scheduled task in place); this transient visit
    // is only to read sessions-list's post-navigation counters, so it takes no screenshots.
    await page.locator('button[aria-controls="work-board"]').click();
    await waitHeading(page, 1, "任务看板");
    await settle(page);
    entry.after = await readMetrics(page);
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await page.getByRole("button", { name: "新建", exact: true }).waitFor({ state: "visible", timeout: 15_000 });
  });

  // 2. Create the one representative session; lands on the "chat" (Workspace) tab.
  await step(results, "session-chat", async (entry) => {
    await createRepresentativeSession(page);
    sessionReady = true;
    entry.screenshots = await takeScreenshots(page, "session-chat");
    entry.active = await readMetrics(page);
    await switchSessionTab(page, "变更");
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 3-10. The remaining eight session tabs. Each one's "away" target is always "chat" (or
  // "changes" for chat itself), so no step depends on where a previous one happened to leave off.
  for (const tab of sessionTabs.filter((candidate) => candidate.id !== "chat")) {
    await step(results, `session-${tab.id}`, async (entry) => {
      requireSession(sessionReady);
      await switchSessionTab(page, tab.label);
      entry.screenshots = await takeScreenshots(page, `session-${tab.id}`);
      entry.active = await readMetrics(page);
      await switchSessionTab(page, "工作区");
      await settle(page);
      entry.after = await readMetrics(page);
    });
  }

  // 11. Information panel (a persistent aside, not a route; "away" is collapsing it).
  await step(results, "information-panel", async (entry) => {
    requireSession(sessionReady);
    await setInfoPanelExpanded(page, true);
    entry.screenshots = await takeScreenshots(page, "information-panel", {
      // Crossing the 900px breakpoint force-collapses the panel (main-layout.tsx's narrowLayout
      // effect), and does not restore it on widening back out. Re-expanding at each narrow width
      // is what lets this surface's own screenshots show the panel rather than that side effect.
      beforeEachWidth: async (width) => {
        if (width <= 900) await setInfoPanelExpanded(page, true);
      },
    });
    entry.active = await readMetrics(page);
    await setInfoPanelExpanded(page, false);
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 12. Scheduled Tasks: a dialog opened from the activity bar over whatever destination is
  // active, not a route. "Away" is closing it, which fully unmounts the dialog component.
  await step(results, "scheduled-tasks", async (entry) => {
    const dialog = await openScheduledTasksDialog(page);
    await createRepresentativeScheduledTask(dialog);
    entry.screenshots = await takeScreenshots(page, "scheduled-tasks", {
      locator: () => page.getByRole("dialog"),
    });
    entry.active = await readMetrics(page);
    await page.keyboard.press("Escape");
    await page.getByRole("dialog").waitFor({ state: "detached", timeout: 10_000 });
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 13. Work Board: dedicated capture. A session and a scheduled task already exist, and
  // web-work-board-client.ts reconciles both into Board items on read, so this is a non-empty
  // view (unlike the transient visit at step 1).
  await step(results, "work-board", async (entry) => {
    await page.locator('button[aria-controls="work-board"]').click();
    await waitHeading(page, 1, "任务看板");
    entry.screenshots = await takeScreenshots(page, "work-board");
    entry.active = await readMetrics(page);
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 14. Goals: the Web/mock goal store starts empty with no auto-reconcile, so one goal is
  // created through the real UI form to give a non-empty baseline.
  await step(results, "goals", async (entry) => {
    await page.locator('button[aria-controls="goal-center"]').click();
    await waitHeading(page, 1, "目标");
    const shell = page.locator("main").first();
    await createRepresentativeGoal(shell);
    entry.screenshots = await takeScreenshots(page, "goals");
    entry.active = await readMetrics(page);
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 15. Mission Control: web-agent-run-state.ts seeds fixture runs by default, so no setup needed.
  await step(results, "mission-control", async (entry) => {
    await page.locator('button[aria-controls="mission-control"]').click();
    await waitHeading(page, 1, "Agent 任务控制台");
    entry.screenshots = await takeScreenshots(page, "mission-control");
    entry.active = await readMetrics(page);
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 16. Loop Center: captured in its true default/first-run state. The Web/mock loop store starts
  // empty and creating a definition goes through a four-step wizard with project discovery; that
  // is out of scope for a factual "as-is" snapshot, so this documents the empty state rather than
  // faking one up. See the final report for the explicit callout.
  await step(results, "loop-center", async (entry) => {
    await page.locator('button[aria-controls="loop-center"]').click();
    await waitHeading(page, 1, "循环工程");
    entry.note = "Web/mock default empty state; the 4-step creation wizard was not driven for this baseline.";
    entry.screenshots = await takeScreenshots(page, "loop-center");
    entry.active = await readMetrics(page);
    await page.locator('button[aria-controls="workspace-session-sidebar"]').click();
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 17. Evaluation: the task catalog and agent registry are pre-seeded, so clicking "Run" once
  // is enough to populate one arena with real (mocked) results.
  await step(results, "evaluation", async (entry) => {
    await page.locator('button[aria-controls="evaluation-center"]').click();
    await waitHeading(page, 1, "Agent 评测");
    const shell = page.locator("main").first();
    await runRepresentativeEvaluation(shell);
    entry.screenshots = await takeScreenshots(page, "evaluation");
    entry.active = await readMetrics(page);
    // Settings is a separate top-level route (src/App.tsx), so this is the one transition in the
    // whole run that actually unmounts the entire workspace tree rather than hiding it.
    await page.getByTestId("desktop-smoke-settings").click();
    await waitHeading(page, 2, "基础配置");
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 18. Settings, default page ("basic"). Reached as evaluation's away-step above.
  await step(results, "settings-basic", async (entry) => {
    await waitHeading(page, 2, "基础配置");
    entry.screenshots = await takeScreenshots(page, "settings-basic");
    entry.active = await readMetrics(page);
    await page.locator("nav").getByRole("button", { name: "CLI 管理", exact: true }).click();
    await waitHeading(page, 2, "CLI 管理");
    await settle(page);
    entry.after = await readMetrics(page);
  });

  // 19. Settings, a second page ("providers" / CLI management). Settings keeps every visited page
  // mounted-but-hidden (settings-shell.tsx), the same pattern as the five workspace destinations.
  await step(results, "settings-providers", async (entry) => {
    await waitHeading(page, 2, "CLI 管理");
    entry.screenshots = await takeScreenshots(page, "settings-providers");
    entry.active = await readMetrics(page);
    await page.getByRole("button", { name: "返回", exact: true }).click();
    await page.locator('button[aria-controls="workspace-session-sidebar"]').waitFor({ state: "visible", timeout: 15_000 });
    await settle(page);
    entry.after = await readMetrics(page);
  });
}

function renderMarkdown(results, meta) {
  const lines = [];
  lines.push("# UI redesign baseline capture");
  lines.push("");
  lines.push(
    `Captured ${meta.capturedAt} against the Web/mock adapter (locale \`${meta.locale}\`, theme \`${meta.theme}\`), ` +
      "generated by `scripts/ui-redesign/capture-baseline.mjs`. This is task 0.7/0.8 evidence for " +
      "`openspec/changes/redesign-unified-workbench-ui`: a factual snapshot, not a judgment.",
  );
  lines.push("");
  lines.push(
    `Screenshot widths: ${meta.widths.join(", ")} (fixed viewport height ${meta.viewportHeight}). ` +
      `Metrics are read at ${meta.metricsViewport.width}x${meta.metricsViewport.height}.`,
  );
  lines.push("");
  lines.push("Methodology notes:");
  lines.push(
    "- The whole run is one continuous browser document (a single initial page load; every other " +
      "transition is in-app client-side navigation). The counters below are window-level and " +
      "cumulative for that whole session, **not** isolated per surface — read the active/after " +
      "delta for a given row as \"what changed while this surface was the one being exercised\", " +
      "not as an absolute cost.",
  );
  lines.push(
    "- \"Intervals net\" / \"Listeners net\" = cumulative started/added minus cleared/removed, via a " +
      "monkeypatch of `window.setInterval`/`clearInterval` and " +
      "`EventTarget.prototype.addEventListener`/`removeEventListener`. \"Observers created\" sums " +
      "`ResizeObserver` + `IntersectionObserver` instantiations (creation is tracked; disposal is not, " +
      "since neither type exposes a disconnect-all hook to patch generically).",
  );
  lines.push(
    "- \"Resource entries\" is `performance.getEntriesByType('resource').length`: a rough, cumulative " +
      "network/chunk-load proxy for the whole document, not exclusive to one surface.",
  );
  lines.push(
    "- The one real exception to \"one continuous document\": Settings is a separate top-level React " +
      "Router route (`/workspace/*` vs `/settings` in `src/App.tsx`), so the `evaluation` → " +
      "`settings-basic` transition actually unmounts and remounts the whole workspace tree (session, " +
      "tabs, and all five destinations), while every other destination switch only toggles a CSS " +
      "`hidden` class and keeps the previous destination mounted (`src/main-layout/main-layout.tsx`).",
  );
  lines.push(
    "- Work Board is visited twice: transiently as `sessions-list`'s navigate-away target (before any " +
      "session or scheduled task exists), and again as its own dedicated row after a session and a " +
      "scheduled task exist (`web-work-board-client.ts` reconciles both into Board items). The " +
      "dedicated row is therefore a second mount, not a cold first mount.",
  );
  lines.push(
    "- Loop Center is captured in its true default/first-run empty state; the Web/mock loop store " +
      "starts empty and creating a definition requires a four-step wizard with project discovery, " +
      "which was out of scope for a factual \"as-is\" snapshot.",
  );
  lines.push(
    "- A negative \"Intervals net\" is not a bug in the counter: browsers share one id namespace " +
      "between `setInterval`/`setTimeout`, so application code calling our-patched `clearInterval` " +
      "on an id that actually came from an unpatched `setTimeout` increments the cleared side " +
      "without a matching started side. Read the sign as \"more clears observed than starts " +
      "observed\", not as a literal negative interval count.",
  );
  lines.push("");
  lines.push(
    "| Surface | Status | DOM nodes (active → after) | Intervals net (active → after) | " +
      "Observers created (active → after) | Listeners net (active → after) | " +
      "Resource entries (active → after) | Note |",
  );
  lines.push("|---|---|---|---|---|---|---|---|");
  for (const result of results) {
    const note = (result.note ?? result.error ?? "").replace(/\|/g, "\\|").replace(/\n/g, " ");
    if (result.status === "ok" && result.active && result.after) {
      lines.push(
        `| ${result.name} | ok | ${result.active.domNodeCount} → ${result.after.domNodeCount} | ` +
          `${result.active.intervalsNet} → ${result.after.intervalsNet} | ` +
          `${result.active.observersCreated} → ${result.after.observersCreated} | ` +
          `${result.active.listenersNet} → ${result.after.listenersNet} | ` +
          `${result.active.resourceEntryCount} → ${result.after.resourceEntryCount} | ${note} |`,
      );
    } else {
      lines.push(`| ${result.name} | ${result.status} | - | - | - | - | - | ${note} |`);
    }
  }
  lines.push("");
  lines.push("## Screenshots");
  lines.push("");
  for (const result of results) {
    if (result.screenshots?.length) {
      const links = result.screenshots
        .map((shot) => `[${shot.width}](${path.basename(shot.file)})`)
        .join(", ");
      lines.push(`- \`${result.name}\`: ${links}`);
    }
  }
  lines.push("");
  return lines.join("\n");
}

async function writeSummary(results, meta) {
  await mkdir(outDir, { recursive: true });
  await writeFile(
    path.join(outDir, "summary.json"),
    `${JSON.stringify({ meta, results }, null, 2)}\n`,
  );
  await writeFile(path.join(outDir, "README.md"), renderMarkdown(results, meta));
}

async function main() {
  await mkdir(outDir, { recursive: true });

  const port = await allocateScreenshotPort(process.env.VANEHUB_BASELINE_PORT);
  log(`starting dev server on 127.0.0.1:${port}...`);
  const vite = await createViteServer({
    root: repoRoot,
    configFile: path.join(repoRoot, "vite.config.ts"),
    server: { host: "127.0.0.1", port, strictPort: true },
    logLevel: "warn",
  });
  await vite.listen();
  const baseURL = `http://127.0.0.1:${port}`;
  log(`dev server ready at ${baseURL}`);

  const browser = await chromium.launch({ headless });
  const context = await browser.newContext({ viewport: { width: 1600, height: viewportHeight } });
  await context.addInitScript(installBaselineCounters);
  await context.addInitScript(installLocalePreference);
  const page = await context.newPage();

  const results = [];
  try {
    log("loading application...");
    await visitRoot(page, baseURL);
    await captureSequence(page, results);
  } catch (error) {
    const message = error instanceof Error ? error.stack ?? error.message : String(error);
    log(`FATAL: ${message}`);
    results.push({ name: "FATAL", status: "error", error: message, screenshots: [], active: null, after: null });
  } finally {
    await context.close().catch(() => {});
    await browser.close().catch(() => {});
    await vite.close().catch(() => {});
  }

  const meta = {
    capturedAt: new Date().toISOString(),
    locale,
    theme,
    widths,
    viewportHeight,
    metricsViewport,
  };
  await writeSummary(results, meta);

  const screenshotCount = results.reduce((sum, r) => sum + (r.screenshots?.length ?? 0), 0);
  const totalBytes = results.reduce(
    (sum, r) => sum + (r.screenshots?.reduce((inner, shot) => inner + shot.bytes, 0) ?? 0),
    0,
  );
  const failed = results.filter((r) => r.status === "error");
  const skipped = results.filter((r) => r.status === "skipped");
  log(`done: ${results.length} surfaces, ${screenshotCount} screenshots, ${totalBytes} bytes total.`);
  log(`ok=${results.length - failed.length - skipped.length} skipped=${skipped.length} failed=${failed.length}`);
  log(`summary written to ${path.relative(repoRoot, path.join(outDir, "README.md"))}`);

  process.exitCode = failed.length > 0 ? 1 : 0;
}

main().catch((error) => {
  console.error("[baseline] unhandled error:", error);
  process.exitCode = 1;
});
