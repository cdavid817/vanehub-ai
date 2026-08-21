import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);

/**
 * Helpers for driving the real desktop client's UI.
 *
 * Selectors are written against the zh-CN rendering and the language is pinned before anything is
 * read, because the client's default follows the host locale: without pinning, the same spec would
 * look for different strings on a ja-JP or en-US runner and fail for a reason unrelated to the
 * behavior under test.
 */
// Chosen because its option values are literal and language-independent, so the assertion does not
// move when the client's copy does.
export const FONT_SIZE_TARGET = "18px";

export async function bootDesktopUi() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120000 });
  await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
    timeout: 120000,
    timeoutMsg: "React bootstrap did not become ready.",
  });
  await globalThis.browser.tauri.execute(({ core }) => core.invoke("save_setting", {
    input: { key: "applicationLanguage", value: "zh-CN" },
  }));
  // Settle on a control this suite actually drives, and match its rendered text rather than an
  // aria-label: an icon button's label never reaches innerText, so labels make a poor ready signal.
  await globalThis.browser.waitUntil(async () => (await createSessionButton()).isExisting(), {
    timeout: 30000,
    timeoutMsg: "The UI did not settle on the pinned language.",
  });
  return root;
}

export async function createWorkspaceFolder(prefix) {
  const folder = await mkdtemp(join(tmpdir(), prefix));
  await run("git", ["init"], { cwd: folder });
  await writeFile(join(folder, "readme.md"), "native ui fixture", "utf8");
  return folder;
}

export const createSessionButton = () => globalThis.$('//button[normalize-space(.)="新建"]');
export const dialog = () => globalThis.$('[role="dialog"]');
export const dialogButton = (label) => globalThis.$(`//*[@role="dialog"]//button[normalize-space(.)="${label}"]`);

export async function activeElementInsideDialog() {
  return globalThis.browser.execute(() => {
    const node = globalThis.document.querySelector('[role="dialog"]');
    const active = globalThis.document.activeElement;
    return Boolean(node && active && (node === active || node.contains(active)));
  });
}

/** Fills the create-session dialog and submits it, returning the created session's title. */
export async function submitCreateSession({ projectPath, title, agentId }) {
  const agent = await globalThis.$(`//*[@role="dialog"]//button[contains(., "${agentId}")]`);
  await agent.waitForClickable({ timeout: 20000 });
  await agent.click();

  const path = await globalThis.$('//*[@role="dialog"]//input[contains(@placeholder, "code")]');
  await path.waitForEnabled({ timeout: 20000 });
  await path.setValue(projectPath);

  const name = await globalThis.$('//*[@role="dialog"]//input[@placeholder="新会话"]');
  await name.setValue(title);

  const create = await dialogButton("创建");
  await globalThis.browser.waitUntil(async () => create.isEnabled(), {
    timeout: 20000,
    timeoutMsg: "The create action never became available.",
  });
  await create.click();
  await globalThis.browser.waitUntil(async () => {
    const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
    return sessions.some((session) => session.title === title);
  }, { timeout: 30000, timeoutMsg: "The session was never created through the dialog." });
  return title;
}

export async function assertNoFatalError(root) {
  assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);
}
