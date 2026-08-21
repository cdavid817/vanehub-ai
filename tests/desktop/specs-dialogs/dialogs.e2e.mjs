import assert from "node:assert/strict";
import {
  assertNoFatalError,
  bootDesktopUi,
  createSessionButton,
  createWorkspaceFolder,
  dialog,
  dialogButton,
  activeElementInsideDialog,
  submitCreateSession,
} from "../helpers/native-ui.mjs";

globalThis.describe("VaneHub AI desktop dialogs", () => {
  // One application instance serves the whole spec file, so the shutdown belongs here rather than
  // at the end of a test: exiting inside one test leaves the next one talking to a dead runtime.
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("opens, focuses, dismisses, and submits a main-path dialog", async function () {
    this.timeout(240000);
    const root = await bootDesktopUi();
    const folder = await createWorkspaceFolder("vanehub-dialog-");

    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();

    const created = await dialog();
    await created.waitForExist({ timeout: 20000 });
    assert.equal(await created.getAttribute("aria-modal"), "true", "the dialog must be modal to assistive technology");
    assert.ok(await activeElementInsideDialog(), "focus never entered the dialog");

    // Escape is the contract every dialog in this client shares; a dialog that traps the user is a
    // desktop-only failure mode, since there is no browser chrome to escape to.
    await globalThis.browser.keys(["Escape"]);
    await created.waitForExist({ timeout: 10000, reverse: true });
    assert.ok(
      await globalThis.browser.execute(() => globalThis.document.activeElement?.textContent?.trim() === "新建"),
      "focus did not return to the control that opened the dialog",
    );

    // Reopening and submitting proves the dialog drives the real service boundary, not just its
    // own local state: the session has to exist natively afterwards.
    await opener.click();
    const reopened = await dialog();
    await reopened.waitForExist({ timeout: 20000 });
    const title = await submitCreateSession({ projectPath: folder, title: "对话框提交验证", agentId: "opencode" });

    await reopened.waitForExist({ timeout: 20000, reverse: true });
    const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
    const session = sessions.find((item) => item.title === title);
    assert.ok(session, "the submitted dialog did not create a session natively");
    assert.equal(session.agentId, "opencode");

    await assertNoFatalError(root);
  });

  globalThis.it("keeps the cancel action non-destructive", async function () {
    this.timeout(180000);
    const root = await bootDesktopUi();
    const before = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));

    const opener = await createSessionButton();
    await opener.waitForClickable({ timeout: 30000 });
    await opener.click();
    const open = await dialog();
    await open.waitForExist({ timeout: 20000 });

    const cancel = await dialogButton("取消");
    await cancel.click();
    await open.waitForExist({ timeout: 10000, reverse: true });

    const after = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
    assert.equal(after.length, before.length, "cancelling the dialog changed native state");

    await assertNoFatalError(root);
  });
});
