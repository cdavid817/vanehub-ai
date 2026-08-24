import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

const WORKSPACE_DESTINATIONS = [
  "sessions", "loops", "work-board", "goals", "evaluations", "mission-control",
];
// Mirrors `SessionTabId` in src/session-workspace/session-tab-bar.tsx. The tab buttons carry
// `aria-controls`, which is the one selector on this screen that does not move with the locale.
const SESSION_TABS = [
  "chat", "changes", "documents", "files", "terminal", "shell", "logs", "traces", "report",
];

const shots = join(process.env.VANEHUB_DESKTOP_RESULT_DIR ?? tmpdir(), "screens");
// The run root is the parent of the isolated data directory (see scripts/desktop/run-context.mjs).
// Fixtures placed under it are removed by `disposeRunContext` once the app has exited, which is
// the only point at which a directory a terminal tab rooted its PTY in is actually unlocked --
// deleting it from inside the test fails with EBUSY on Windows while the child is still alive.
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();
const rendered = [];

async function capture(name) {
  await globalThis.browser.saveScreenshot(join(shots, `${name}.png`));
  rendered.push(name);
}

/**
 * A screen counts as rendered only if React owns it, it put visible text on the page, and it is
 * not showing an error.
 *
 * The error check is the one that earns its keep: the Goals screen passed a "no fatal boundary,
 * some text" assertion while displaying `Command list_goals not found` in a red banner, because a
 * handled error still renders a perfectly healthy-looking page. A screen sweep that cannot tell
 * those apart reports coverage it does not have.
 */
async function assertScreenRendered(name) {
  const root = await globalThis.$("#root");
  const fatalDetail = await root.getAttribute("data-vanehub-fatal-error-detail");
  assert.equal(
    await root.getAttribute("data-vanehub-fatal-error"),
    null,
    `${name} tripped the fatal error boundary: ${fatalDetail ?? "diagnostic detail unavailable"}`,
  );
  const text = (await root.getText()).trim();
  assert.ok(text.length > 0, `${name} rendered no text`);
  assert.ok(
    !/^VaneHub AI\s*Starting\.\.\.$/.test(text),
    `${name} is still the static bootstrap shell`,
  );
  const notFound = text.match(/Command \w+ not found/);
  assert.equal(notFound, null, `${name} called an unregistered command: ${notFound?.[0]}`);
  // A `LazyFeature` fallback is a screen that has not arrived yet. Counting it as rendered made
  // the sweep race the chunk load: the same screen was captured as full content in one run and as
  // a bare spinner in the next, and both were reported as covered.
  assert.ok(
    !/^(正在加载功能|Loading feature)/.test(text),
    `${name} was captured as its loading placeholder rather than the screen`,
  );
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

globalThis.describe("VaneHub AI desktop screen sweep", () => {
  globalThis.before(async () => {
    await mkdir(shots, { recursive: true });
    // Reload rather than accept whatever the previous spec left behind. WDIO's `deleteSession`
    // does not terminate the Tauri app in this harness, so spec files can inherit a page that has
    // already driven an npm install, a pip install and an MCP lifecycle. On such a page the
    // `pushState` route change never commits -- react-router wraps every history-driven location
    // change in `startTransition`, which keeps the old DOM on screen with no error and no log, so
    // the sweep sat on Settings until it timed out. Every run that passed had a freshly launched
    // app; both that failed had inherited one.
    await globalThis.browser.refresh();
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("renders every workspace destination", async () => {
    for (const destination of WORKSPACE_DESTINATIONS) {
      await navigate(`/workspace/${destination}`);
      await globalThis.browser.waitUntil(
        async () => (await globalThis.browser.getUrl()).includes(`/workspace/${destination}`),
        { timeout: 20_000, timeoutMsg: `The WebView did not route to ${destination}.` },
      );
      const frame = await globalThis.$('[data-testid="workspace-frame"]');
      await frame.waitForExist({ timeout: 20_000 });
      await assertScreenRendered(`workspace/${destination}`);
      await capture(`workspace-${destination}`);
    }
  });

  globalThis.it("renders every Settings page", async () => {
    await navigate("/settings");
    const sidebar = await globalThis.$("nav");
    await sidebar.waitForExist({ timeout: 20_000 });
    const buttons = await globalThis.$$("nav button");
    assert.ok(
      buttons.length >= 17,
      `expected the 17 Settings destinations, found ${buttons.length}`,
    );

    for (let index = 0; index < buttons.length; index += 1) {
      // Re-query: activating a page re-renders the sidebar, which detaches the earlier handles.
      const current = (await globalThis.$$("nav button"))[index];
      const label = (await current.getText()).trim().replaceAll(/[^\p{L}\p{N}]+/gu, "-") || `page-${index}`;
      await current.click();
      await globalThis.browser.waitUntil(async () => {
        const panels = await globalThis.$$("section > div:not([hidden])");
        if (panels.length === 0) return false;
        return (await panels[0].getText()).trim().length > 0;
      }, { timeout: 30_000, timeoutMsg: `Settings page ${index} (${label}) rendered nothing.` });
      await assertScreenRendered(`settings/${label}`);
      await capture(`settings-${String(index).padStart(2, "0")}-${label}`);
    }
  });

  globalThis.it("renders every session workspace tab", async () => {
    await mkdir(fixtureRoot, { recursive: true });
    const repository = await mkdtemp(join(fixtureRoot, "screens-"));
    await run("git", ["init"], { cwd: repository });
    await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: repository });
    await run("git", ["config", "user.name", "Desktop E2E"], { cwd: repository });
    await writeFile(join(repository, "seed.txt"), "seed\n", "utf8");
    await run("git", ["add", "seed.txt"], { cwd: repository });
    await run("git", ["commit", "-m", "fixture"], { cwd: repository });

    // A CLI session, not an API one: `onepiece` needs a configured provider profile before a
    // session can start, and this sweep is about screens rather than provider setup. A CLI Agent
    // also renders the terminal tab with something real behind it.
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const cli = agents.find((agent) => agent.availabilityState === "available"
      && agent.supportedInteractionModes.includes("cli"));
    assert.ok(cli, "the session tab sweep needs one installed CLI Agent");

    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: cli.id,
      interactionMode: "cli",
      title: "Screen sweep session",
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    const settled = await globalThis.browser.waitUntil(async () => {
      const status = await invoke(
        ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
        operation.id,
      );
      return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
    }, { timeout: 30_000, timeoutMsg: "Session creation never settled." });
    assert.equal(settled.status, "succeeded", settled.error ?? "session creation failed");

    const session = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === "Screen sweep session") ?? false;
    }, { timeout: 30_000, timeoutMsg: "The sweep session was not created." });

    // Reload before navigating. The session was created over IPC, after this page had already
    // loaded its session list, so the UI does not know it exists -- routing straight to its id
    // lands on "session unavailable" and bounces back to the list. The route commits either way,
    // which is why asserting the URL is not enough on its own: the tab bar is simply never
    // rendered, and waiting longer reads that as a slow mount.
    await globalThis.browser.refresh();
    const root = await globalThis.$("#root");
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready after the reload." },
    );

    await navigate(`/workspace/sessions/${encodeURIComponent(session.id)}`);
    // The tab bar is not lazy -- `mountedTabs` starts as `{"chat"}` and `SessionTabBar` renders
    // unconditionally -- so if the button is missing, the workspace did not open for this session.
    const firstTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
    await firstTab.waitForExist({ timeout: 30_000 });

    for (const tab of SESSION_TABS) {
      const button = await globalThis.$(`[aria-controls="session-tab-panel-${tab}"]`);
      await button.waitForExist({ timeout: 20_000 });
      await button.click();
      const panel = await globalThis.$(`#session-tab-panel-${tab}`);
      await panel.waitForExist({ timeout: 30_000 });
      await globalThis.browser.waitUntil(
        async () => (await button.getAttribute("aria-selected")) === "true",
        { timeout: 20_000, timeoutMsg: `The ${tab} tab never became selected.` },
      );
      await assertScreenRendered(`session-tab/${tab}`);
      await capture(`session-tab-${tab}`);
    }
  });

  globalThis.after(async () => {
    // The app restores its last destination on relaunch, and spec files in one run share a data
    // directory — leaving this sweep parked on Settings decides where the next spec starts.
    await navigate("/workspace/sessions");
    // Session tab panels stay mounted once visited, so the terminal and shell tabs leave live
    // PTYs rooted in the fixture repository. Nothing else releases them: the shell tab only kills
    // its shell on unmount. Exiting here lets the runtime reap them, which is what makes the run
    // root deletable — otherwise the harness's own cleanup fails with EBUSY on Windows and the
    // whole verification reports FAILED with every test green.
    await invoke(({ core }) => core.invoke("exit_application"));
    globalThis.console.log(`Screens rendered (${rendered.length}):\n  ${rendered.join("\n  ")}`);
    globalThis.console.log(`Screenshots: ${shots}`);
  });
});
