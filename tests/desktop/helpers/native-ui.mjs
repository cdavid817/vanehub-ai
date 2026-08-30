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

export async function waitForDesktopBootstrap() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120000 });
  await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
    timeout: 120000,
    timeoutMsg: "React bootstrap did not become ready.",
  });
  return root;
}

export async function bootDesktopUi() {
  const root = await waitForDesktopBootstrap();
  await globalThis.browser.tauri.execute(({ core }) => core.invoke("save_setting", {
    input: { key: "applicationLanguage", value: "zh-CN" },
  }));
  // The startup activity is intentionally not assumed. The activity bar is common to every
  // workspace surface, so it is the stable readiness signal each layer can navigate from.
  await globalThis.browser.waitUntil(async () => (await scheduledTasksButton()).isExisting(), {
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

/**
 * Selects a session workspace tab without WebKitGTK's unreliable centre-point hit test.
 *
 * Buttons inside the horizontally scrollable tab strip can be visible, enabled, and accept a
 * real WebDriver click while `waitForClickable` reports the strip itself at the button's centre.
 * The selected state remains the product assertion, so this only removes the false precondition.
 */
export async function clickWorkspaceTab(title, timeout = 30000) {
  const selector = `//*[@role="tablist" and @aria-label="会话工作区"]//*[@role="tab" and @title="${title}"]`;
  for (let attempt = 0; attempt < 3; attempt += 1) {
    const tab = await globalThis.$(selector);
    await tab.waitForDisplayed({ timeout });
    assert.equal(await tab.isEnabled(), true, `The ${title} workspace tab was disabled.`);
    try {
      await tab.click();
    } catch {
      const hitTest = await globalThis.browser.execute((element) => {
        const rect = element.getBoundingClientRect();
        const covering = globalThis.document.elementFromPoint(
          rect.left + rect.width / 2,
          rect.top + rect.height / 2,
        );
        return {
          coveringClass: covering?.getAttribute("class") ?? null,
          coveringTag: covering?.tagName ?? null,
          rect: { height: rect.height, left: rect.left, top: rect.top, width: rect.width },
        };
      }, tab);
      throw new Error(`Clicking the ${title} workspace tab failed: ${JSON.stringify(hitTest)}`);
    }
    const controls = await tab.getAttribute("aria-controls");
    const panel = controls ? await globalThis.$(`[id="${controls}"]`) : null;
    const selected = await globalThis.browser.waitUntil(async () => (
      (await tab.getAttribute("aria-selected")) === "true" && Boolean(await panel?.isDisplayed())
    ), { timeout, timeoutMsg: `The ${title} tab never exposed its selected panel.` }).then(
      () => true,
      () => false,
    );
    if (!selected) continue;
    // Session creation can commit a late activation that resets the tab immediately after the
    // first selected frame. Re-check after that transition window before returning to the caller.
    await globalThis.browser.pause(500);
    if ((await tab.getAttribute("aria-selected")) === "true" && await panel?.isDisplayed()) return tab;
  }
  throw new Error(`The ${title} tab did not remain selected after session activation settled.`);
}

export async function createSessionButton() {
  const create = await globalThis.$('//button[normalize-space(.)="新建"]');
  if (await create.isDisplayed()) return create;
  const sessions = await globalThis.$('button[aria-controls="workspace-session-sidebar"]');
  await sessions.waitForClickable({ timeout: 30000 });
  await sessions.click();
  await create.waitForDisplayed({ timeout: 30000 });
  return create;
}

export const scheduledTasksButton = () => globalThis.$('button[aria-haspopup="dialog"][aria-label="定时任务"]');
export const dialog = () => globalThis.$('[role="dialog"]');
export const dialogButton = (label) => globalThis.$(`//*[@role="dialog"]//button[normalize-space(.)="${label}"]`);

export async function dialogField(label) {
  const fieldLabel = await globalThis.$(`//*[@role="dialog"]//label[normalize-space(.)="${label}"]`);
  await fieldLabel.waitForExist({ timeout: 20000 });
  const id = await fieldLabel.getAttribute("for");
  if (!id) throw new Error(`Dialog field has no associated control: ${label}`);
  return globalThis.$(`[id="${id}"]`);
}

export async function selectDialogOption(label, value) {
  const select = await dialogField(label);
  const id = await select.getAttribute("id");
  await globalThis.browser.execute((controlId, selectedValue) => {
    const control = globalThis.document.getElementById(controlId);
    if (!(control instanceof globalThis.HTMLSelectElement)) throw new Error(`Not a select: ${controlId}`);
    const setter = Object.getOwnPropertyDescriptor(globalThis.HTMLSelectElement.prototype, "value")?.set;
    setter?.call(control, selectedValue);
    control.dispatchEvent(new globalThis.Event("input", { bubbles: true }));
    control.dispatchEvent(new globalThis.Event("change", { bubbles: true }));
  }, id, value);
  await globalThis.browser.waitUntil(async () => (await select.getValue()) === value, {
    timeout: 10000,
    timeoutMsg: `${label} did not select ${value}.`,
  });
  return select;
}

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
