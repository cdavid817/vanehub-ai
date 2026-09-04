import assert from "node:assert/strict";
import {
  assertNoFatalError,
  bootDesktopUi,
  createSessionButton,
  createWorkspaceFolder,
  dialog,
  submitCreateSession,
} from "../helpers/native-ui.mjs";

const WORKSPACE_TABS = '//*[@role="tablist" and @aria-label="会话工作区"]';
const workspaceTab = (title) => globalThis.$(`${WORKSPACE_TABS}//*[@role="tab" and @title="${title}"]`);

// Real pixels, not the Playwright web-adapter's sub-1100 widths: the client enforces
// `minWidth: 1100` / `minHeight: 700` (tauri.conf.json) on its own native window, so a WebDriver
// resize below the floor would silently clamp and the assertions below would be comparing a
// resize that never happened.
const WIDE = { width: 1440, height: 900 };
const NARROW = { width: 1150, height: 760 };
// Far enough from WIDE/NARROW that a real re-fit cannot be mistaken for measurement noise.
const REFIT_DELTA = 150;
// The documented, intentional sub-cell overhang (styles.css `.ucd-shell-terminal` — xterm sizes its
// canvas in whole rows, and the leftover fraction overhangs by a few px) is a *height* story; this
// spec only asserts on width, where fit() rounds cols down and should leave ~0 overhang.
const OVERHANG_TOLERANCE_PX = 6;

/** Selects a workspace tab and waits for it to actually become the selected one. */
async function openWorkspaceTab(title) {
  const tab = await workspaceTab(title);
  await tab.waitForClickable({ timeout: 30000 });
  await tab.click();
  await globalThis.browser.waitUntil(async () => (await tab.getAttribute("aria-selected")) === "true", {
    timeout: 30000,
    timeoutMsg: `The ${title} tab never became the selected tab.`,
  });
  return tab;
}

async function shellTerminalHost() {
  const host = await globalThis.$('//*[@id="session-tab-panel-shell"]//div[@role="log"]');
  await host.waitForExist({ timeout: 60000 });
  return host;
}

/**
 * Container vs. rendered xterm geometry, read from the DOM rather than from xterm's own JS state:
 * the `Terminal` instance lives inside a React ref this spec has no handle on, and `.xterm-screen`
 * is the one element xterm sizes to its cols/rows computation (unlike `.xterm-viewport`, which is
 * absolutely positioned edge-to-edge and would report the container's own size even if `fit()` had
 * never run), so reading it is what actually proves a re-fit happened.
 */
async function terminalGeometry() {
  return globalThis.browser.execute(() => {
    const host = globalThis.document.querySelector('#session-tab-panel-shell [role="log"]');
    const screen = host?.querySelector(".xterm-screen");
    if (!host || !screen) return null;
    const hostRect = host.getBoundingClientRect();
    const screenRect = screen.getBoundingClientRect();
    return {
      hostWidth: Math.round(hostRect.width),
      screenWidth: Math.round(screenRect.width),
      overhangRight: Math.round(screenRect.right - hostRect.right),
    };
  });
}

async function waitForFit(predicate, timeoutMsg) {
  return globalThis.browser.waitUntil(async () => {
    const geometry = await terminalGeometry();
    return geometry && predicate(geometry) ? geometry : false;
  }, { timeout: 30000, timeoutMsg });
}

/**
 * Listens for the same native event the mounted terminal consumes (`session-shell:notice`), so a
 * typed marker can be confirmed as echoed without reading rendered text out of the DOM -- xterm
 * draws to canvas by default, so nothing under `.xterm-screen` exposes it through `innerText`. This
 * is a second, passive listener alongside the UI's own; Tauri events broadcast to every subscriber
 * in the webview, so adding one does not touch the live attachment the visible terminal depends on.
 */
async function armShellOutputCapture() {
  await globalThis.browser.execute(() => {
    globalThis.__shellNotices = [];
    globalThis.__TAURI__.event.listen("session-shell:notice", (message) => {
      globalThis.__shellNotices.push(message.payload);
    });
  });
}

async function waitForEcho(marker, timeoutMsg) {
  await globalThis.browser.waitUntil(async () => globalThis.browser.execute((needle) => (
    (globalThis.__shellNotices ?? []).some(
      (notice) => notice.type === "output" && typeof notice.data === "string" && notice.data.includes(needle),
    )
  ), marker), { timeout: 30000, timeoutMsg });
}

/** Types a unique marker into the real terminal and waits for the shell to echo it back. */
async function typeAndEcho(marker) {
  const host = await shellTerminalHost();
  await host.click();
  await globalThis.browser.keys(marker);
  await globalThis.browser.keys(["Enter"]);
  await waitForEcho(marker, `the Shell never echoed "${marker}" back`);
}

/** No element renders wider than the viewport -- the same composition contract
 * `ui-agent-configuration.e2e.mjs` checks for Settings pages, applied here to a live session
 * workspace under a real window resize instead of a fixed size chosen before mount. */
async function assertNoHorizontalOverflow(context) {
  const diagnostics = await globalThis.browser.execute(() => ({
    clientWidth: globalThis.document.documentElement.clientWidth,
    scrollWidth: globalThis.document.documentElement.scrollWidth,
  }));
  assert.equal(
    diagnostics.scrollWidth > diagnostics.clientWidth,
    false,
    `${context} overflowed the desktop WebView horizontally: ${JSON.stringify(diagnostics)}`,
  );
}

globalThis.describe("VaneHub AI desktop window resize composition", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("keeps the Shell terminal live and unclipped across a real native window resize", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();
    const folder = await createWorkspaceFolder("vanehub-resize-");

    await globalThis.browser.setWindowSize(WIDE.width, WIDE.height);
    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();
    await (await dialog()).waitForExist({ timeout: 20000 });
    await submitCreateSession({ projectPath: folder, title: "窗口缩放终端验证", agentId: "opencode" });

    await openWorkspaceTab("Shell");
    await shellTerminalHost();
    await armShellOutputCapture();

    const wide = await waitForFit(
      (geometry) => geometry.hostWidth > 0 && geometry.overhangRight <= OVERHANG_TOLERANCE_PX,
      "The Shell terminal never settled inside its container at the wide window.",
    );
    const stamp = Date.now().toString(36);
    await typeAndEcho(`resize-wide-${stamp}`);

    await globalThis.browser.setWindowSize(NARROW.width, NARROW.height);
    const narrow = await waitForFit(
      (geometry) => geometry.hostWidth > 0
        && geometry.hostWidth < wide.hostWidth - REFIT_DELTA
        && geometry.overhangRight <= OVERHANG_TOLERANCE_PX,
      "The Shell terminal did not re-fit inside its container after the window narrowed.",
    );
    await assertNoHorizontalOverflow("the narrow window with the Shell tab open");
    await typeAndEcho(`resize-narrow-${stamp}`);

    await globalThis.browser.setWindowSize(WIDE.width, WIDE.height);
    const restored = await waitForFit(
      (geometry) => geometry.hostWidth > narrow.hostWidth + REFIT_DELTA
        && geometry.overhangRight <= OVERHANG_TOLERANCE_PX,
      "The Shell terminal did not re-fit inside its container after the window widened again.",
    );
    await typeAndEcho(`resize-restored-${stamp}`);

    globalThis.console.warn("SHELL_RESIZE_GEOMETRY " + JSON.stringify({ wide, narrow, restored }));
    await assertNoFatalError(root);
  });

  globalThis.it(
    "reflows every workspace tab without horizontal overflow across a real window resize",
    async function () {
      this.timeout(180000);
      const root = await bootDesktopUi();

      // Reuses the session the previous test left active -- one shared application instance serves
      // this whole file, the same convention `session-shell.e2e.mjs` already relies on.
      await globalThis.browser.setWindowSize(NARROW.width, NARROW.height);
      for (const title of ["工作区", "变更", "文件", "Shell"]) {
        await openWorkspaceTab(title);
        await assertNoHorizontalOverflow(`the ${title} tab at a narrow native window`);
      }

      await globalThis.browser.setWindowSize(WIDE.width, WIDE.height);
      await openWorkspaceTab("工作区");
      await assertNoHorizontalOverflow("the 工作区 tab after widening the native window back");

      await assertNoFatalError(root);
    },
  );
});
