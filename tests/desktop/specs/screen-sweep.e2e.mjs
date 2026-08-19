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
  "sessions", "plans", "loops", "work-board", "goals", "evaluations", "mission-control",
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
  assert.equal(
    await root.getAttribute("data-vanehub-fatal-error"),
    null,
    `${name} tripped the fatal error boundary`,
  );
  const text = (await root.getText()).trim();
  assert.ok(text.length > 0, `${name} rendered no text`);
  assert.ok(
    !/^VaneHub AI\s*Starting\.\.\.$/.test(text),
    `${name} is still the static bootstrap shell`,
  );
  const notFound = text.match(/Command \w+ not found/);
  assert.equal(notFound, null, `${name} called an unregistered command: ${notFound?.[0]}`);
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

    await navigate(`/workspace/sessions/${encodeURIComponent(session.id)}`);
    const firstTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
    // The session workspace mounts nine lazily-loaded tab panels, which on a loaded machine takes
    // longer than the default. A short wait here fails as "the tab bar is missing" when the truth
    // is that it had not finished mounting.
    await firstTab.waitForExist({ timeout: 90_000 });

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
