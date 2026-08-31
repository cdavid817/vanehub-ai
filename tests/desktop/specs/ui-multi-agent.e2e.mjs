import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

/**
 * The multi-Agent group chat driven the way a user drives it: clicks on the roster editor,
 * keystrokes into the composer, an `@` completion picked with the pointer, and assertions on the
 * speaker identity the transcript actually paints. The `domain-multi-agent-*` specs prove the
 * runtime — routing, dispatch, attribution in the database; none of them proves the interface in
 * front of that runtime is wired to it: a broken mention popup, a roster editor that saves
 * nothing, or a bubble that drops its role label would pass every one of them.
 *
 * Setup (session creation, seating the first two participants) goes through `core.invoke`, the
 * same shortcut every `ui-*` spec takes for fixtures; everything a *user* would do to run a group
 * chat — grow the roster, address a seat, read who spoke — happens through the DOM.
 */

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];
const stamp = Date.now().toString(36);

// ---------------------------------------------------------------------------------------------
// Selectors, each read out of the component that renders it and cited file:line, so a rename
// surfaces as a stale citation rather than a silently non-matching query. Nothing matches
// localized copy: role display names (架构师 …) are data from the built-in role catalogue, not
// translation strings.
// ---------------------------------------------------------------------------------------------
const COMPOSER = '[data-testid="wechat-style-composer"]';        // ChatInputBox.tsx:173
const INPUT = `${COMPOSER} textarea`;                            // ChatInputBox.tsx:222
const TOOLBAR = '[data-testid="composer-toolbar"]';              // ButtonArea.tsx:62
const ACTION = `${TOOLBAR} > div:last-child > button:last-child`;
const SEND_ICON = `${ACTION} svg.lucide-send`;                   // ButtonArea.tsx:130
// SeatMentionCompletion is the only role="group" the composer overlay renders for an `@` draft in
// a multi-seat session (SeatMentionCompletion.tsx:27); its per-seat rows are its buttons.
const MENTION_OPTION = `${COMPOSER} [role="group"] button`;
const MEMBERS_TAB = '[data-testid="accordion-header-participants"]';   // session-overview-sections.tsx:67
const MEMBERS_PANE = '[data-testid="accordion-content-participants"]'; // Accordion.tsx:77
const ROSTER = '[data-testid="session-roster-editor"]';          // session-roster-editor.tsx:72
// The editor's add-seat form: two selects (Agent, then role — session-roster-editor.tsx:92-97)
// and the one button in that grid (the Plus — session-roster-editor.tsx:99-109).
const ROSTER_SELECTS = `${ROSTER} select`;
const ROSTER_ADD = `${ROSTER} div.grid.grid-cols-2 > button`;
// The seat-count badge beside the panel title (session-roster-editor.tsx:78-80).
const ROSTER_BADGE = `${ROSTER} span.rounded-full`;
const SPEAKER = '[data-testid="message-speaker"]';               // MessageItem.tsx:64
const SPEAKER_COLOR = '[data-testid="message-role-color"]';      // MessageItem.tsx:65
const CHAT_TAB = '[aria-controls="session-tab-panel-chat"]';     // session-tabs.tsx:154

const BUILTIN_ROLES = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

let repository = null;
let session = null;
let handles = [];

async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "ui-multi-agent-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function usableAgents() {
  const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
  return agents.filter((agent) => agent.availabilityState === "available"
    && agent.supportedInteractionModes.includes("cli"));
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

const countOf = async (selector) => (await globalThis.$$(selector)).length;

/**
 * Sets a `<select>`'s value the way React can see. WebDriver's `selectByVisibleText` clicks the
 * option, which this WebKitGTK driver performs without firing a `change` event — the DOM shows
 * the new value while React's state keeps the old one, and the roster then saved a seat whose
 * role was silently null. The prototype setter bypasses React's value tracker so the dispatched
 * event reads as a real edit.
 */
function chooseOption(selector, index, value) {
  return globalThis.browser.execute((sel, at, next) => {
    const element = globalThis.document.querySelectorAll(sel)[at];
    const setter = Object.getOwnPropertyDescriptor(
      globalThis.HTMLSelectElement.prototype,
      "value",
    ).set;
    setter.call(element, next);
    element.dispatchEvent(new globalThis.Event("change", { bubbles: true }));
  }, selector, index, value);
}

function textsOf(selector) {
  return globalThis.browser.execute((target) => Array.from(
    globalThis.document.querySelectorAll(target),
  ).map((node) => (node.textContent ?? "").replace(/\s+/g, " ").trim()), selector);
}

globalThis.describe("VaneHub AI desktop multi-Agent UI", () => {
  globalThis.before(async function prepare() {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();

    // Detection runs the CLIs' own version probes and finishes seconds after boot; asking once at
    // bootstrap-ready raced it and reported an equipped host as having no Agents at all.
    const usable = await globalThis.browser.waitUntil(async () => {
      const found = await usableAgents();
      return found.length >= 2 ? found : false;
    }, { timeout: 90_000, timeoutMsg: "" }).catch(() => []);
    if (usable.length < 2) {
      blocked.push(`ui multi-agent: needs two installed CLI Agents, found ${usable.length}`);
      return;
    }
    const roles = await invoke(({ core }) => core.invoke("list_expert_roles"));
    const seatRoles = BUILTIN_ROLES.map((id) => roles.find((role) => role.id === id)).filter(Boolean);
    if (seatRoles.length < 3) {
      blocked.push("ui multi-agent: fewer than three built-in expert roles are available");
      return;
    }
    handles = seatRoles.map((role) => role.displayName.split(/\s+/u).filter(Boolean).join("-"));

    const title = `ui-multiagent-${stamp}`;
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: usable[0].id,
      interactionMode: "cli",
      title,
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
    }, { timeout: 90_000, timeoutMsg: "Creating the UI session never settled." });
    assert.equal(settled.status, "succeeded", settled.error ?? "session creation failed");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === title) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The UI session was not created." });

    session = await invoke(({ core }, input) => core.invoke("update_session_seats", { input }), {
      sessionId: created.id,
      expectedUpdatedAt: created.updatedAt,
      seats: [
        { agentId: usable[0].id, roleId: seatRoles[0].id },
        { agentId: usable[1 % usable.length].id, roleId: seatRoles[1].id },
      ],
    });

    // The session was created behind the UI's back, and the workspace route only mounts a session
    // its own list already contains (use-workspace-session-route.ts:43-46 — an unknown id with no
    // loaded list waits forever). A reload is how the UI learns about fixtures it did not create.
    await globalThis.browser.refresh();
    const rebooted = await globalThis.$("#root");
    await rebooted.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await rebooted.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not come back after the reload." },
    );
  });

  globalThis.it("shows the roster in the members pane and grows it by one seat", async function roster() {
    if (!session) {
      blocked.push("roster: no session; the reason is in the setup entry above");
      this.skip();
    }
    await navigate(`/workspace/sessions/${encodeURIComponent(session.id)}`);
    await (await globalThis.$(CHAT_TAB)).waitForExist({ timeout: 30_000 });
    await globalThis.browser.waitUntil(async () => {
      const active = await invoke(({ core }) => core.invoke("get_active_session"));
      return active?.id === session.id;
    }, { timeout: 30_000, timeoutMsg: "The app never switched to the multi-seat session." });

    // The members tab renders only for a session the UI recognizes as multi-seat
    // (session-overview-sections.tsx:45), so its very existence is already an assertion.
    const membersTab = await globalThis.$(MEMBERS_TAB);
    await membersTab.waitForExist({
      timeout: 30_000,
      timeoutMsg: "the members tab never appeared for a two-seat session",
    });
    await membersTab.click();
    await (await globalThis.$(`${MEMBERS_PANE} ${ROSTER}`)).waitForDisplayed({ timeout: 30_000 });
    assert.equal((await textsOf(ROSTER_BADGE))[0], "2", "the roster badge does not count two seats");

    // Grow the roster from the form a user actually uses: pick the first Agent the picker offers,
    // pick the third built-in role by its display name (data, not translation), click add.
    const selects = await globalThis.$$(ROSTER_SELECTS);
    assert.equal(selects.length, 2, "the roster editor does not render its two add-seat selects");
    await chooseOption(ROSTER_SELECTS, 1, BUILTIN_ROLES[2]);
    await (await globalThis.$(ROSTER_ADD)).click();

    // The verdict is the backend's, not the badge's: the badge counts the optimistic draft, which
    // shows 3 while the mutation is still in flight and falls back to 2 if it fails — a first
    // version of this case asserted 200ms after the click and reported the race as a defect.
    const activeSeatCount = async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.id === session.id)
        ?.seats?.filter((seat) => !seat.leftAt).length ?? 0;
    };
    const firstTry = await globalThis.browser.waitUntil(
      async () => await activeSeatCount() === 3,
      { timeout: 20_000, timeoutMsg: "" },
    ).catch(() => false);
    if (!firstTry) {
      // The mutation failed and the editor said why. A second click after its refetch reset tells
      // conflict apart from a dead control — a retry that succeeds means the first attempt lost
      // an optimistic-revision race, which is worth reporting but is not a broken roster editor.
      const alerts = await textsOf(`${ROSTER} [role="alert"]`);
      blocked.push(`roster: the first add attempt did not commit (editor said `
        + `${JSON.stringify(alerts)}); retrying once`);
      await globalThis.browser.waitUntil(
        async () => (await textsOf(ROSTER_BADGE))[0] === "2",
        { timeout: 15_000, timeoutMsg: "the editor never settled after the failed add" },
      );
      await chooseOption(ROSTER_SELECTS, 1, BUILTIN_ROLES[2]);
      await (await globalThis.$(ROSTER_ADD)).click();
      await globalThis.browser.waitUntil(
        async () => await activeSeatCount() === 3,
        { timeout: 20_000, timeoutMsg: "the UI add produced no third active seat in the backend" },
      );
    }
    await globalThis.browser.waitUntil(
      async () => (await textsOf(ROSTER_BADGE))[0] === "3",
      { timeout: 15_000, timeoutMsg: "the roster badge never reflected the third seat" },
    );
    // The seat carries the role that was picked, not just a body in a chair — the silent-change
    // failure mode is a seat saved with a null role, which only surfaces later as a popup entry
    // named after its Agent.
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    const added = sessions.find((item) => item.id === session.id)
      ?.seats?.filter((seat) => !seat.leftAt)?.at(-1);
    assert.equal(added?.roleId, BUILTIN_ROLES[2], "the added seat did not keep the picked role");
  });

  globalThis.it("offers seats on `@` and routes the send to the seat picked from the popup", async function mention() {
    if (!session) {
      blocked.push("mention: no session; the reason is in the setup entry above");
      this.skip();
    }
    const input = await globalThis.$(INPUT);
    await input.waitForDisplayed({ timeout: 30_000 });
    await globalThis.browser.waitUntil(async () => await countOf(`${INPUT}:disabled`) === 0, {
      timeout: 30_000,
      timeoutMsg: "The composer stayed disabled, so this session refuses sends.",
    });

    // A real keystroke, because the completion query only ever updates from onChange.
    await input.click();
    await input.addValue("@");
    await globalThis.browser.waitUntil(
      async () => await countOf(MENTION_OPTION) >= 3,
      { timeout: 15_000, timeoutMsg: "typing @ did not offer the three seated participants" },
    );
    const labels = await textsOf(MENTION_OPTION);
    assert.ok(
      handles.every((handle) => labels.some((label) => label.includes(handle))),
      `the mention popup is missing a seat: offered ${JSON.stringify(labels)}, wanted ${JSON.stringify(handles)}`,
    );

    // Pick the SECOND seat with the pointer — the first seat answering anyway is exactly the
    // regression the runtime fix closed, and the popup writing the wrong handle would reopen it.
    const options = await globalThis.$$(MENTION_OPTION);
    const target = [];
    for (const option of options) {
      if ((await option.getText()).includes(handles[1])) target.push(option);
    }
    assert.equal(target.length, 1, `no single popup row matches ${handles[1]}`);
    await target[0].click();
    await globalThis.browser.waitUntil(
      async () => (await (await globalThis.$(INPUT)).getValue()).startsWith(`@${handles[1]}`),
      { timeout: 10_000, timeoutMsg: "picking a seat did not write its handle into the draft" },
    );

    await input.addValue(" 请回复一个字：好");
    await (await globalThis.$(SEND_ICON)).waitForExist({ timeout: 10_000 });
    await (await globalThis.$(ACTION)).click();

    // The routing verdict, read where a user reads it: the reply bubble's speaker label names the
    // picked seat's role and paints its colour dot. The runtime writes the attributed row before
    // the provider answers, so this needs the dispatch, not the model's full reply.
    const speaker = await globalThis.$(SPEAKER);
    await speaker.waitForExist({
      timeout: 120_000,
      timeoutMsg: "no attributed reply bubble ever rendered",
    });
    await globalThis.browser.waitUntil(
      async () => (await textsOf(SPEAKER)).some((label) => label.includes(handles[1])),
      {
        timeout: 30_000,
        timeoutMsg: `the reply bubble is not labelled with ${handles[1]}`,
      },
    );
    assert.ok(
      await countOf(`${SPEAKER} ${SPEAKER_COLOR}`) >= 1,
      "the speaker label renders no role colour dot",
    );
  });

  globalThis.after(async () => {
    if (session) {
      await invoke(({ core }, id) => core.invoke("stop_generation", { sessionId: id }), session.id)
        .catch(() => {});
      if (globalThis.process?.env?.VANEHUB_DESKTOP_KEEP_SESSIONS !== "1") {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), session.id)
          .catch(() => {});
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
