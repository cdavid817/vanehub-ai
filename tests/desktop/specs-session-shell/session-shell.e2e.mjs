import assert from "node:assert/strict";
import {
  assertNoFatalError,
  bootDesktopUi,
  clickWorkspaceTab,
  createSessionButton,
  createWorkspaceFolder,
  dialog,
  submitCreateSession,
} from "../helpers/native-ui.mjs";

const SHELL_PANEL = '//*[@id="session-tab-panel-shell"]';

const shellStrip = () => globalThis.$('[role="tablist"][aria-label="已打开的 Shell"]');
const shellTabs = () => globalThis.$$('[role="tablist"][aria-label="已打开的 Shell"] [role="tab"]');

/**
 * Scoped to the Shell panel, not to the document.
 *
 * "关闭" and "取消" appear on other surfaces of the application, and an unscoped match resolves to
 * whichever one the document happens to hold first — usually one that is not even visible, which a
 * driver reports as a click that timed out rather than as the wrong element.
 */
async function shellButton(label) {
  const button = await globalThis.$(`${SHELL_PANEL}//button[normalize-space(.)="${label}"]`);
  await button.waitForDisplayed({ timeout: 30000 });
  assert.equal(await button.isEnabled(), true, `The ${label} Shell action was disabled.`);
  return button;
}

const shellDialog = () => globalThis.$(`${SHELL_PANEL}//*[@role="dialog"]`);

/**
 * The Shells the native registry is holding, read through the registered command.
 *
 * Read from the registry rather than from the DOM: the whole claim under test is that a Shell
 * outlives its view, and a check that could only see what the view is currently rendering could
 * not tell "still running" from "re-created on the way back".
 */
async function registrySnapshot(sessionId) {
  return globalThis.browser.tauri.execute(
    ({ core }, id) => core.invoke("list_session_shells", { sessionId: id }),
    sessionId,
  );
}

async function activeSessionId() {
  return globalThis.browser.execute(() => {
    const panel = globalThis.document.querySelector('[id^="session-tab-panel-"]');
    return panel?.getAttribute("data-session-id") ?? null;
  });
}

globalThis.describe("VaneHub AI desktop retained session shells", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("keeps a shell running across tab and session switches", async function () {
    this.timeout(600000);
    const root = await bootDesktopUi();
    const folder = await createWorkspaceFolder("vanehub-shell-");

    // 1. A session to hold the Shells.
    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();
    await (await dialog()).waitForExist({ timeout: 20000 });
    await submitCreateSession({ projectPath: folder, title: "Shell 保留验证", agentId: "opencode" });

    // 2. Showing the Shell tab opens the session's default Shell once the registry has loaded.
    await clickWorkspaceTab("Shell");
    await (await shellStrip()).waitForExist({ timeout: 60000 });
    const sessionId = await activeSessionId();
    assert.ok(sessionId, "the workspace panel did not report which session it belongs to");

    const opened = await globalThis.browser.waitUntil(
      async () => {
        const shells = await registrySnapshot(sessionId);
        return Array.isArray(shells) && shells.length === 1 ? shells : false;
      },
      { timeout: 60000, timeoutMsg: "The Shell tab never opened a Shell for this session." },
    );
    const firstShellId = opened[0].shellId;
    assert.equal(opened[0].sessionId, sessionId);

    // 3. Add produces a second Shell rather than replacing the first.
    await (await shellButton("新建 Shell")).click();
    const both = await globalThis.browser.waitUntil(
      async () => {
        const shells = await registrySnapshot(sessionId);
        return Array.isArray(shells) && shells.length === 2 ? shells : false;
      },
      { timeout: 60000, timeoutMsg: "Add did not produce a second Shell." },
    );
    assert.ok(
      both.some((shell) => shell.shellId === firstShellId),
      "Add replaced the existing Shell instead of adding one",
    );
    await globalThis.browser.waitUntil(async () => (await shellTabs()).length === 2, {
      timeout: 30000,
      timeoutMsg: "The strip never showed both Shells.",
    });

    // 4. Rename goes through the registry, so the new title survives a re-read.
    await (await shellButton("重命名")).click();
    const nameField = await globalThis.$("#shell-rename-input");
    await nameField.waitForExist({ timeout: 20000 });
    await nameField.setValue("构建");
    await (await shellButton("保存")).click();
    await globalThis.browser.waitUntil(
      async () => {
        const shells = await registrySnapshot(sessionId);
        return shells.some((shell) => shell.title === "构建");
      },
      { timeout: 30000, timeoutMsg: "The rename never reached the registry." },
    );

    // 5. Leaving for another workspace tab must not end anything.
    await clickWorkspaceTab("日志");
    await globalThis.browser.waitUntil(
      async () => (await registrySnapshot(sessionId)).length === 2,
      { timeout: 30000, timeoutMsg: "Switching workspace tabs ended a Shell." },
    );

    // 6. Coming back finds the same Shells, by id.
    await clickWorkspaceTab("Shell");
    const returned = await globalThis.browser.waitUntil(
      async () => {
        const shells = await registrySnapshot(sessionId);
        return shells.length === 2 ? shells : false;
      },
      { timeout: 60000, timeoutMsg: "Returning to the Shell tab did not find the retained Shells." },
    );
    assert.deepEqual(
      returned.map((shell) => shell.shellId).sort(),
      both.map((shell) => shell.shellId).sort(),
      "the Shells that came back are not the ones that were left",
    );
    assert.ok(
      returned.every((shell) => shell.state === "running" || shell.state === "starting"),
      `a retained Shell stopped running: ${JSON.stringify(returned.map((shell) => shell.state))}`,
    );

    // 7. A second session, then back — the first session's Shells are still there.
    const otherFolder = await createWorkspaceFolder("vanehub-shell-other-");
    const secondOpener = await createSessionButton();
    await secondOpener.waitForClickable({ timeout: 30000 });
    await secondOpener.click();
    await (await dialog()).waitForExist({ timeout: 20000 });
    await submitCreateSession({ projectPath: otherFolder, title: "另一个会话", agentId: "opencode" });
    await globalThis.browser.waitUntil(
      async () => (await activeSessionId()) !== sessionId,
      { timeout: 60000, timeoutMsg: "The second session never became active." },
    );
    await globalThis.browser.waitUntil(
      async () => (await registrySnapshot(sessionId)).length === 2,
      { timeout: 30000, timeoutMsg: "Switching sessions ended the first session's Shells." },
    );

    globalThis.console.warn(`SESSION_SHELL_RETAINED ${JSON.stringify(returned)}`);
    await assertNoFatalError(root);
  });

  globalThis.it("ends a shell only through an explicit confirmed close", async function () {
    this.timeout(600000);
    const root = await bootDesktopUi();

    await clickWorkspaceTab("Shell");
    await (await shellStrip()).waitForExist({ timeout: 60000 });
    const sessionId = await activeSessionId();
    const before = await globalThis.browser.waitUntil(
      async () => {
        const shells = await registrySnapshot(sessionId);
        return Array.isArray(shells) && shells.length >= 1 ? shells : false;
      },
      { timeout: 60000, timeoutMsg: "This session has no Shell to close." },
    );

    // 8. Close asks first, and cancelling leaves the Shell alone.
    await (await shellButton("关闭")).click();
    await (await shellDialog()).waitForExist({ timeout: 20000 });
    await (await shellButton("取消")).click();
    await globalThis.browser.waitUntil(
      async () => (await registrySnapshot(sessionId)).length === before.length,
      { timeout: 30000, timeoutMsg: "Cancelling the close dialog ended the Shell anyway." },
    );

    // 9. Confirming is the one path that ends it.
    await (await shellButton("关闭")).click();
    await (await shellDialog()).waitForExist({ timeout: 20000 });
    await (await shellButton("关闭 Shell")).click();
    await globalThis.browser.waitUntil(
      async () => (await registrySnapshot(sessionId)).length === before.length - 1,
      { timeout: 30000, timeoutMsg: "The confirmed close did not end the Shell." },
    );

    globalThis.console.warn(`SESSION_SHELL_CLOSED ${before.length} -> ${before.length - 1}`);
    await assertNoFatalError(root);
  });
});
