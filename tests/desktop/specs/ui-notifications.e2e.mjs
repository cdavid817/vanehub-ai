import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

import { fill } from "../helpers/form-control.mjs";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
let createdSessionId = null;

/**
 * The notification center (src/notifications/) has no IPC surface at all -- `notify()` is a
 * frontend-only React reducer, so this is a `ui-*` spec even though it needs no `domain-*`
 * counterpart to be complete: there is nothing on the backend to invoke.
 *
 * `notify()` fires from a handful of frontend call sites (use-main-layout-model.ts), not from any
 * command completing. Session creation through the dialog is the cheapest of them to reach --
 * every other call site needs a session to already exist -- so this file creates one exactly the
 * way ui-workspace.e2e.mjs does, purely as a vehicle for a real notification, and does not repeat
 * that file's assertions about the dialog itself.
 *
 * Selectors are structural. The app's default language on this host is zh-CN
 * (src/i18n/supported-locales.ts:47), and every button in the notification center's header and
 * per-item row carries a translated aria-label -- lucide-react's own class name on each icon
 * (`svg.lucide-<name>`) is what survives locale, matching how ui-settings.e2e.mjs already picks
 * the Skills page out by `svg.lucide-puzzle`.
 */
const blocked = [];
let repository = null;
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "vanehub-notif-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function bootstrapReady(timeoutMsg) {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg },
  );
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

function bell() {
  return globalThis.$('[aria-controls="notification-center"]');
}

function center() {
  return globalThis.$("#notification-center");
}

/**
 * Lucide's own class is the one label in this panel that does not move with locale. Resolved by
 * walking up from the icon in the page rather than a `:has()` selector -- this suite has already
 * hit more than one WebKitGTK selector-engine gap (form-control.mjs's `selectOption` history), and
 * `closest("button")` needs nothing from the driver beyond what every other spec here already
 * relies on.
 */
async function iconButton(scope, iconName) {
  // Not a raw DOM element returned from `execute()`: this driver does not reliably serialize an
  // element handle computed inside the page back to the WDIO side (the same class of gap
  // `selectOption` above already works around for `<select>`). An index resolved through the
  // page, then re-located with WDIO's own `$$`, sidesteps it entirely.
  const buttons = await scope.$$("button");
  const index = await globalThis.browser.execute((root, className) => {
    const icon = root.querySelector(`svg.${className}`);
    const button = icon?.closest("button") ?? null;
    return button ? Array.from(root.querySelectorAll("button")).indexOf(button) : -1;
  }, scope, `lucide-${iconName}`);
  return index >= 0 ? buttons[index] : null;
}

async function unreadCount() {
  const badge = await (await bell()).$("span");
  if (!(await badge.isExisting())) return 0;
  const text = (await badge.getText()).trim();
  return text === "99+" ? 100 : Number.parseInt(text, 10);
}

/** Creates one session through the dialog purely to produce a real "session created" notification. */
async function createSessionForNotification() {
  const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
  const usable = agents.find((agent) => agent.availabilityState === "available"
    && agent.supportedInteractionModes.includes("cli"));
  if (!usable) {
    blocked.push("notification source: no installed CLI Agent to create a session with");
    return false;
  }

  await navigate("/workspace/sessions");
  const newButton = await globalThis.$(
    '[data-testid="session-sidebar"] > div:first-child button:not([aria-haspopup="menu"])',
  );
  await newButton.waitForClickable({ timeout: 30_000 });
  await newButton.click();

  const dialog = await globalThis.$('[role="dialog"][aria-modal="true"]');
  await dialog.waitForExist({ timeout: 20_000 });
  const projectInput = await dialog.$('input[placeholder*="code"][placeholder*="project"]');
  await projectInput.waitForExist({ timeout: 20_000 });
  await fill("The project folder field", projectInput, repository);
  await globalThis.browser.execute((input) => input.blur(), projectInput);

  const worktreeCheckbox = await dialog.$('input[type="checkbox"]');
  await globalThis.browser.waitUntil(
    async () => worktreeCheckbox.isEnabled(),
    { timeout: 30_000, timeoutMsg: "The dialog never recognised the fixture as a Git project." },
  );

  const agentButton = await globalThis.browser.execute((ids) => {
    const buttons = Array.from(globalThis.document.querySelectorAll('[role="dialog"] button[aria-pressed]'));
    return buttons.findIndex((button) => ids.some(
      (id) => Array.from(button.querySelectorAll("span")).some((span) => (span.textContent ?? "").trim() === id),
    ));
  }, [usable.id]);
  if (agentButton >= 0) {
    const buttons = await dialog.$$('button[aria-pressed]');
    await buttons[agentButton].click();
  }

  const dialogButtons = await dialog.$$("button");
  const createButton = dialogButtons[dialogButtons.length - 1];
  await globalThis.browser.waitUntil(
    async () => createButton.isEnabled(),
    { timeout: 15_000, timeoutMsg: "Create stayed disabled after the dialog was filled in." },
  );
  await createButton.click();

  await globalThis.browser.waitUntil(async () => {
    const current = await globalThis.browser.getUrl();
    const segment = decodeURIComponent((current.split("/workspace/sessions/")[1] ?? "").split(/[?#]/)[0]);
    return segment && segment !== "new" ? segment : false;
  }, { timeout: 60_000, timeoutMsg: "Session creation for the notification test never settled." });
  const current = await globalThis.browser.getUrl();
  createdSessionId = decodeURIComponent((current.split("/workspace/sessions/")[1] ?? "").split(/[?#]/)[0]);
  return true;
}

globalThis.describe("VaneHub AI desktop notification center", () => {
  globalThis.before(async () => {
    await globalThis.browser.refresh();
    await bootstrapReady("React bootstrap did not become ready.");
    repository = await createRepository();
  });

  globalThis.it("badges, lists, reads, and clears a real 'session created' notification", async function notificationFlow() {
    const before = await unreadCount();
    const created = await createSessionForNotification();
    if (!created) this.skip();

    await globalThis.browser.waitUntil(
      async () => (await unreadCount()) === before + 1,
      { timeout: 30_000, timeoutMsg: "The bell badge did not increment after creating a session." },
    );

    await (await bell()).click();
    const panel = await center();
    await panel.waitForDisplayed({ timeout: 10_000 });

    // newest first (notification-center.tsx: `notifications.slice().reverse()`).
    const article = await panel.$("article");
    await article.waitForExist({ timeout: 10_000 });

    // The mark-all-read control only renders while unreadCount > 0
    // (notification-center.tsx header), so clicking it here also proves the badge and the panel
    // agree on there being something unread.
    const markAllRead = await iconButton(panel, "check-check");
    assert.ok(markAllRead, "the mark-all-read control was not offered with an unread notification present");
    await markAllRead.click();
    await globalThis.browser.waitUntil(
      async () => (await unreadCount()) === 0,
      { timeout: 10_000, timeoutMsg: "Mark-all-read did not clear the unread badge." },
    );
    assert.equal(
      await iconButton(panel, "check-check"),
      null,
      "the mark-all-read control stayed offered after nothing was unread",
    );

    const clearButton = await iconButton(panel, "trash-2");
    await clearButton.click();
    await globalThis.browser.waitUntil(
      async () => !(await (await center()).$("article").then((element) => element.isExisting())),
      { timeout: 10_000, timeoutMsg: "Clear did not remove the notification list." },
    );
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    if (createdSessionId) {
      await invoke(
        ({ core }, sessionId) => core.invoke("delete_session", { sessionId }),
        createdSessionId,
      ).catch(() => undefined);
    }
  });
});
