import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { basename, dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

import { fill, selectOption } from "../helpers/form-control.mjs";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

/**
 * Interactive coverage of the workspace: every step here goes through a control a user can reach.
 *
 * The `domain-*` specs drive `core.invoke` and prove the backend answers. That leaves the gap this
 * file covers -- a command can be perfect while nothing on screen is wired to it, and no amount of
 * IPC coverage notices. IPC is used here only to decide what to click (which Agents are installed)
 * and to undo what the run created; every assertion reads the rendered DOM.
 *
 * Selectors are structural on purpose. The app's default language on this host is zh-CN
 * (src/i18n/supported-locales.ts:47), so any selector spelled as visible text is one translation
 * away from silently matching nothing. Where a text anchor was unavoidable it is a string this
 * file typed in itself, never a localized one.
 */
const blocked = [];
const stamp = Date.now().toString(36);
const primaryTitle = `uiflow-primary-${stamp}`;
const secondaryTitle = `uiflow-secondary-${stamp}`;
const workItemTitle = `uiflow-work-item-${stamp}`;
// Matches only the primary session. `search_sessions` also matches `sessions.project_path`
// (src-tauri/src/contexts/sessions/infrastructure/sqlite_repository.rs:1172), so a token borrowed
// from the shared fixture path would match both sessions and prove nothing about narrowing.
const primaryToken = `primary-${stamp}`;
// The order the board lays its columns out in, which is what turns a column index back into a
// stage without reading a translated column heading. src/types/work-board.ts:1.
const workItemStages = ["inbox", "planned", "in_progress", "review", "done"];

let repository = null;
let primarySessionId = null;
let secondarySessionId = null;
let workItemId = null;
const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

/** The run-context disposer removes this fixture after the final app process has exited. */
async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "vanehub-uiflow-"));
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
  assert.equal(
    await root.getAttribute("data-vanehub-fatal-error"),
    null,
    "the workspace tripped its fatal error boundary",
  );
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

async function waitForUrl(fragment, timeoutMsg, timeout = 30_000) {
  await globalThis.browser.waitUntil(
    async () => (await globalThis.browser.getUrl()).includes(fragment),
    { timeout, timeoutMsg },
  );
}

async function settleOperation(operation, timeoutMsg) {
  const settled = await globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 60_000, timeoutMsg });
  assert.equal(settled.status, "succeeded", settled.error ?? timeoutMsg);
  return settled;
}

/** Session cards carry their id, so the sidebar can be counted without reading any title. */
function sessionCards() {
  return globalThis.$$('[data-testid="session-sidebar"] [data-session-id]');
}

/** The tab bar only exists while the workspace tabs are shown, so its count is the assertion. */
function workspaceTabLists() {
  return globalThis.$$('[data-testid="session-workspace"] [role="tablist"]');
}

globalThis.describe("VaneHub AI desktop workspace UI flows", () => {
  globalThis.before(async () => {
    // Never inherit the previous spec's page. Route changes commit inside `startTransition`, so a
    // page left mid-flight keeps rendering the old DOM with no error to notice.
    await globalThis.browser.refresh();
    await bootstrapReady("React bootstrap did not become ready.");
    repository = await createRepository();
  });

  globalThis.it("creates a session from the sidebar dialog and opens its workspace", async function createThroughDialog() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const usableIds = new Set(agents
      .filter((agent) => agent.availabilityState === "available"
        && agent.supportedInteractionModes.includes("cli"))
      .map((agent) => agent.id));
    if (usableIds.size === 0) {
      blocked.push("session creation: no installed CLI Agent, and the dialog refuses to submit an unselectable one");
      this.skip();
    }

    await navigate("/workspace/sessions");
    const sidebar = await globalThis.$('[data-testid="session-sidebar"]');
    await sidebar.waitForExist({ timeout: 30_000 });

    // session-sidebar.tsx:263 -- the New button shares the sidebar's header row with the overflow
    // trigger, and the trigger is the one carrying aria-haspopup. Excluding it picks New out
    // without reading either label.
    const newButton = await globalThis.$(
      '[data-testid="session-sidebar"] > div:first-child button:not([aria-haspopup="menu"])',
    );
    await newButton.waitForClickable({ timeout: 30_000 });
    await newButton.click();
    await waitForUrl("/workspace/sessions/new", "The New button did not open the create-session route.");

    const dialog = await globalThis.$('[role="dialog"][aria-modal="true"]');
    await dialog.waitForExist({ timeout: 20_000 });
    // create-session-workspace-sections.tsx:90 -- a literal sample path rather than a translated
    // string, which is what makes it usable as an anchor in any locale.
    const projectInput = await dialog.$('input[placeholder*="code"][placeholder*="project"]');
    await projectInput.waitForExist({ timeout: 20_000 });

    // create-session-dialog-content.tsx:151 -- the footer is the last thing in the dialog and
    // Create is the last thing in the footer. That it is disabled before a project path is entered
    // (create-session-dialog-utils.ts:98) both proves this is the right button and exercises the
    // validation gate.
    const dialogButtons = await dialog.$$("button");
    const createButton = dialogButtons[dialogButtons.length - 1];
    assert.equal(
      await createButton.isEnabled(),
      false,
      "the create-session dialog offered to submit before a project folder was chosen",
    );

    const agentIds = agents.map((agent) => agent.id);
    const cards = [];
    for (const button of await dialog.$$("button[aria-pressed]")) {
      // The Agent cards render `agent.id` verbatim in a span of its own
      // (create-session-agent-section.tsx:62), which is the one label in this dialog that is an
      // identifier rather than a translation. The single/multi mode buttons also carry
      // aria-pressed, and requiring a span whose whole text *is* the id is what tells them apart.
      //
      // Read from the DOM rather than from `getText()`: these spans are laid out with no line box
      // between them, so getText collapses the card to "Claude CodeClaude Codeclaude-code" and a
      // line-wise match finds nothing. That reported "the dialog offered no installed Agent" over
      // a screenshot showing all five of them.
      const agentId = await globalThis.browser.execute((card, ids) => {
        const spans = Array.from(card.querySelectorAll("span"));
        return ids.find((id) => spans.some((span) => (span.textContent ?? "").trim() === id)) ?? null;
      }, button, agentIds);
      if (agentId) cards.push({ agentId, button, pressed: (await button.getAttribute("aria-pressed")) === "true" });
    }
    const offered = cards.filter((card) => usableIds.has(card.agentId));
    if (offered.length === 0) {
      blocked.push(`session creation: the dialog offered no installed Agent (saw ${cards.map((card) => card.agentId).join(", ") || "none"})`);
      this.skip();
    }
    // Prefer one the dialog has not already preselected so the click has to change something.
    const target = offered.find((card) => !card.pressed) ?? offered[0];
    await target.button.click();
    await globalThis.browser.waitUntil(
      async () => (await target.button.getAttribute("aria-pressed")) === "true",
      { timeout: 10_000, timeoutMsg: `Clicking the ${target.agentId} card did not select it.` },
    );
    const pressedAfter = [];
    for (const card of cards) {
      if ((await card.button.getAttribute("aria-pressed")) === "true") pressedAfter.push(card.agentId);
    }
    assert.deepEqual(
      pressedAfter,
      [target.agentId],
      "the Agent cards did not settle on a single exclusive selection",
    );

    await fill("The project folder field", projectInput, repository);
    // create-session-workspace-sections.tsx:88 -- the path is inspected on blur, so the field has
    // to lose focus for anything to happen. Neither `Tab` nor a click on the dialog heading moved
    // it here: both runs left the field still ringed in the failure screenshot, with neither the
    // "Git project" line nor an error under it, so `inspectPath` had not run at all. Blurring the
    // element directly is what actually reaches the app's own handler. It does not prove the
    // dialog is keyboard-traversable; nothing in this suite does.
    await globalThis.browser.execute((input) => input.blur(), projectInput);
    // create-session-workspace-sections.tsx:147 -- the worktree checkbox stays disabled until an
    // inspection comes back reporting a Git project, so it becoming enabled is the dialog telling
    // us it read the path rather than just storing the characters.
    const worktreeCheckbox = await dialog.$('input[type="checkbox"]');
    await globalThis.browser.waitUntil(
      async () => worktreeCheckbox.isEnabled(),
      { timeout: 30_000, timeoutMsg: "The dialog never recognised the fixture as a Git project." },
    );

    const inputs = await dialog.$$("input");
    const titleInput = inputs[inputs.length - 1];
    // create-session-dialog.tsx:165 derives the title from the folder name until the user edits it
    // (lib/session-path.ts:20). A value in exactly that shape is proof this is the session-name
    // field, which is otherwise only identifiable by a translated label.
    const derivedTitle = String((await titleInput.getProperty("value")) ?? "");
    assert.ok(
      derivedTitle.startsWith(`${basename(repository)}-`) && /-\d{8}-\d{6}$/.test(derivedTitle),
      `the last dialog input held ${JSON.stringify(derivedTitle)}, not the derived session name`,
    );
    await fill("The session name field", titleInput, primaryTitle);

    await globalThis.browser.waitUntil(
      async () => createButton.isEnabled(),
      { timeout: 15_000, timeoutMsg: "Create stayed disabled after the dialog was fully filled in." },
    );
    await createButton.click();

    // Settles on either outcome rather than only on success. create-session-dialog-content.tsx:138
    // renders the reason a create was refused, and that reason is the only actionable part of the
    // failure -- letting the wait run its full course instead would report a timeout for what is
    // really a rejection. It is read out here rather than thrown from inside the condition because
    // `waitUntil` swallows a throwing condition and reports its own timeout message instead.
    let refusal = "";
    await globalThis.browser.waitUntil(async () => {
      const open = await globalThis.$$('[role="dialog"][aria-modal="true"]');
      if (open.length === 0) return true;
      refusal = (await (await open[0].$('[role="alert"]')).getText()).trim();
      return refusal.length > 0;
    }, { timeout: 120_000, interval: 1_000, timeoutMsg: "The create-session dialog neither closed nor reported a reason." });
    assert.equal(refusal, "", `the create-session dialog refused the submission: ${refusal}`);

    primarySessionId = await globalThis.browser.waitUntil(async () => {
      const current = await globalThis.browser.getUrl();
      const segment = decodeURIComponent((current.split("/workspace/sessions/")[1] ?? "").split(/[?#]/)[0]);
      // `new` is the create route's own segment (workbench-route.ts:22), so it is the one value that
      // means the dialog closed without the workspace ever moving on to a session.
      return segment && segment !== "new" ? segment : false;
    }, { timeout: 30_000, timeoutMsg: "The dialog closed without routing to the new session." });

    const card = await globalThis.$(`[data-session-id="${primarySessionId}"]`);
    await card.waitForExist({ timeout: 30_000 });
    await globalThis.browser.waitUntil(
      async () => (await card.getAttribute("aria-pressed")) === "true",
      { timeout: 30_000, timeoutMsg: "The new session was listed but never shown as the active one." },
    );
    assert.ok(
      (await card.getText()).includes(primaryTitle),
      "the sidebar card for the new session did not show the title that was typed",
    );

    const workspace = await globalThis.$('[data-testid="session-workspace"]');
    await workspace.waitForDisplayed({ timeout: 30_000 });
    const heading = await globalThis.$('[data-testid="session-conversation-header"] h2');
    await globalThis.browser.waitUntil(
      async () => (await heading.getText()).trim() === primaryTitle,
      { timeout: 30_000, timeoutMsg: "The conversation header never took the new session's title." },
    );
    const chatTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
    await chatTab.waitForExist({ timeout: 30_000 });
    assert.equal(
      await chatTab.getAttribute("aria-selected"),
      "true",
      "the new session's workspace did not open on its chat tab",
    );
    assert.ok(
      await (await globalThis.$("#session-tab-panel-chat")).isDisplayed(),
      "the chat tab was selected but its panel was not on screen",
    );
  });

  globalThis.it("switches destinations from the activity bar, toggles panels, and keeps the destination across a reload", async () => {
    await navigate("/workspace/sessions");
    const grid = await globalThis.$('[data-testid="sessions-destination-layout"]');
    await grid.waitForDisplayed({ timeout: 30_000 });

    // workspace-activity-bar.tsx:90-99 -- Projects/Runs/Plan/Quality all share
    // aria-controls="workbench-route-outlet" now that main-layout.tsx routes them through one
    // shared outlet, so position within the primary button group is what picks Plan out (it is
    // fourth: sessions, projects, runs, plan, quality).
    const boardButton = await globalThis.$('nav.ucd-activity-bar div[data-activity-group="primary"] button:nth-child(4)');
    await boardButton.waitForClickable({ timeout: 20_000 });
    await boardButton.click();
    // #todo-board is WorkBoard's own root (work-board.tsx:80); the Plan destination that mounts it
    // carries no id of its own now that per-destination wrapper ids are gone.
    await (await globalThis.$("#todo-board")).waitForDisplayed({ timeout: 30_000 });
    await globalThis.browser.waitUntil(
      async () => !(await grid.isDisplayed()),
      { timeout: 20_000, timeoutMsg: "The sessions grid stayed on screen after switching destination." },
    );
    await waitForUrl("/workspace/plan/board", "The activity bar did not change the workspace route.");

    await globalThis.browser.refresh();
    await bootstrapReady("React bootstrap did not become ready after the reload.");
    // Either route back counts: the WebView may reload the deep path directly, or land on the
    // launch route and be sent here by the remembered location (workbench-route.ts). What the
    // user is promised is the destination, not the mechanism.
    await waitForUrl("/workspace/plan/board", "The chosen destination did not survive a reload.");
    await (await globalThis.$("#todo-board")).waitForDisplayed({ timeout: 30_000 });

    const sessionsButton = await globalThis.$('nav.ucd-activity-bar button[aria-controls="workspace-session-sidebar"]');
    await sessionsButton.waitForClickable({ timeout: 20_000 });
    await sessionsButton.click();
    const sessionsGrid = await globalThis.$('[data-testid="sessions-destination-layout"]');
    await sessionsGrid.waitForDisplayed({ timeout: 30_000 });

    const sessionSidebar = await globalThis.$("#workspace-session-sidebar");
    const conversation = await globalThis.$('[data-testid="session-conversation-shell"]');
    assert.equal(await sessionsButton.getAttribute("aria-expanded"), "true");
    assert.equal(await sessionsGrid.getAttribute("data-session-collapsed"), "false");
    const expandedWidth = (await sessionSidebar.getSize()).width;
    assert.ok(expandedWidth > 0, "the session sidebar reported no width while expanded");
    const conversationWidthExpanded = (await conversation.getSize()).width;

    // On the sessions destination the same button collapses the sidebar (main-layout.tsx). Closed
    // now unmounts the region entirely (destination-layout/DestinationLayoutBody.tsx) instead of
    // hiding it in place, so absence from the DOM is the assertion, backed by the conversation
    // column actually growing into the freed space.
    await sessionsButton.click();
    await globalThis.browser.waitUntil(
      async () => (await sessionsGrid.getAttribute("data-session-collapsed")) === "true",
      { timeout: 20_000, timeoutMsg: "The activity bar did not collapse the session sidebar." },
    );
    assert.equal(await sessionsButton.getAttribute("aria-expanded"), "false");
    assert.equal(
      await (await globalThis.$("#workspace-session-sidebar")).isExisting(),
      false,
      "the collapsed sidebar was still mounted in the DOM",
    );
    const conversationWidthCollapsed = (await conversation.getSize()).width;
    assert.ok(
      conversationWidthCollapsed > conversationWidthExpanded + 150,
      `the conversation column measured ${conversationWidthExpanded}px expanded and ${conversationWidthCollapsed}px collapsed; collapsing the sidebar did not free its space`,
    );
    await sessionsButton.click();
    await globalThis.browser.waitUntil(
      async () => (await sessionsGrid.getAttribute("data-session-collapsed")) === "false",
      { timeout: 20_000, timeoutMsg: "The activity bar did not restore the session sidebar." },
    );
    const restoredSidebar = await globalThis.$("#workspace-session-sidebar");
    assert.ok(
      Math.abs((await restoredSidebar.getSize()).width - expandedWidth) <= 1,
      "the restored sidebar came back a different width",
    );

    const overflow = await globalThis.$('[data-testid="conversation-overflow-trigger"]');
    assert.equal((await workspaceTabLists()).length, 1, "the workspace did not start with its tab bar showing");
    await overflow.click();
    const tabsToggle = await globalThis.$('[data-testid="toggle-workspace-tabs"]');
    await tabsToggle.waitForDisplayed({ timeout: 10_000 });
    assert.equal(await tabsToggle.getAttribute("aria-checked"), "true");
    await tabsToggle.click();
    await globalThis.browser.waitUntil(
      async () => (await workspaceTabLists()).length === 0,
      { timeout: 20_000, timeoutMsg: "The menu entry did not hide the workspace tab bar." },
    );
    await overflow.click();
    const tabsToggleBack = await globalThis.$('[data-testid="toggle-workspace-tabs"]');
    await tabsToggleBack.waitForDisplayed({ timeout: 10_000 });
    assert.equal(
      await tabsToggleBack.getAttribute("aria-checked"),
      "false",
      "the menu still reported the tab bar as showing after it was hidden",
    );
    await tabsToggleBack.click();
    await globalThis.browser.waitUntil(
      async () => (await workspaceTabLists()).length === 1,
      { timeout: 20_000, timeoutMsg: "The menu entry did not bring the workspace tab bar back." },
    );

    // The info panel opens by default at every window size now (main-layout.tsx's
    // `readInitialSessionsLayout`); the toggle's direction is still read off the layout rather
    // than assumed, since an earlier step in this same spec may have left it closed. Closed now
    // unmounts the panel entirely (destination-layout/DestinationLayoutBody.tsx) instead of
    // hiding it in place, so its presence in the DOM is the assertion.
    const infoCollapsedAtStart = (await sessionsGrid.getAttribute("data-info-collapsed")) === "true";
    assert.equal(
      await (await globalThis.$('[data-testid="session-info-panel"]')).isExisting(),
      !infoCollapsedAtStart,
      "the info panel's presence in the DOM disagreed with the layout's collapsed flag",
    );
    await overflow.click();
    const infoToggle = await globalThis.$('[data-testid="toggle-info-panel"]');
    await infoToggle.waitForDisplayed({ timeout: 10_000 });
    assert.equal(
      await infoToggle.getAttribute("aria-checked"),
      infoCollapsedAtStart ? "false" : "true",
      "the menu reported the info panel in a state the layout disagreed with",
    );
    await infoToggle.click();
    await globalThis.browser.waitUntil(
      async () => (await sessionsGrid.getAttribute("data-info-collapsed")) === String(!infoCollapsedAtStart),
      { timeout: 20_000, timeoutMsg: "The menu entry did not change the info panel's state." },
    );
    assert.equal(
      await (await globalThis.$('[data-testid="session-info-panel"]')).isExisting(),
      infoCollapsedAtStart,
      "toggling the info panel did not change what is mounted on screen",
    );
    await overflow.click();
    const infoToggleBack = await globalThis.$('[data-testid="toggle-info-panel"]');
    await infoToggleBack.waitForDisplayed({ timeout: 10_000 });
    await infoToggleBack.click();
    await globalThis.browser.waitUntil(
      async () => (await sessionsGrid.getAttribute("data-info-collapsed")) === String(infoCollapsedAtStart),
      { timeout: 20_000, timeoutMsg: "The menu entry did not restore the info panel." },
    );
    assert.equal(
      await (await globalThis.$('[data-testid="session-info-panel"]')).isExisting(),
      !infoCollapsedAtStart,
      "the info panel did not come back after being restored",
    );
  });

  globalThis.it("moves the session workspace between tabs on click", async function switchTabs() {
    if (!primarySessionId) {
      blocked.push("session tabs: no session was created through the dialog, so there is no workspace to drive");
      this.skip();
    }
    await navigate(`/workspace/sessions/${encodeURIComponent(primarySessionId)}`);
    const chatTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
    await chatTab.waitForDisplayed({ timeout: 30_000 });

    // Deliberately not `terminal` or `shell`. Both leave a live PTY behind once mounted, and this
    // file has no `exit_application` to reap them with.
    let current = "chat";
    for (const tab of ["files", "logs", "report", "chat"]) {
      const button = await globalThis.$(`[aria-controls="session-tab-panel-${tab}"]`);
      await button.waitForClickable({ timeout: 20_000 });
      await button.click();
      await globalThis.browser.waitUntil(
        async () => (await button.getAttribute("aria-selected")) === "true",
        { timeout: 20_000, timeoutMsg: `The ${tab} tab never reported itself as selected.` },
      );
      const panel = await globalThis.$(`#session-tab-panel-${tab}`);
      await panel.waitForDisplayed({ timeout: 30_000 });
      // A panel that mounted as an empty shell would still pass a visibility check, so require it
      // to have put something on screen.
      await globalThis.browser.waitUntil(
        async () => (await panel.getText()).trim().length > 0,
        { timeout: 30_000, timeoutMsg: `The ${tab} panel became visible but rendered nothing.` },
      );

      if (tab !== current) {
        const previousButton = await globalThis.$(`[aria-controls="session-tab-panel-${current}"]`);
        assert.equal(
          await previousButton.getAttribute("aria-selected"),
          "false",
          `both ${current} and ${tab} claimed to be the selected tab`,
        );
        assert.equal(
          await (await globalThis.$(`#session-tab-panel-${current}`)).isDisplayed(),
          false,
          `the ${current} panel stayed on screen after ${tab} was selected`,
        );
      }
      current = tab;
    }
  });

  globalThis.it("creates a work item on the board and moves it between stages", async function driveWorkBoard() {
    // See the destination-switching spec above for why this is a position-based selector.
    const boardButton = await globalThis.$('nav.ucd-activity-bar div[data-activity-group="primary"] button:nth-child(4)');
    await boardButton.waitForClickable({ timeout: 20_000 });
    await boardButton.click();
    const board = await globalThis.$("#todo-board");
    await board.waitForDisplayed({ timeout: 60_000 });

    // work-board.tsx:80 -- one section per stage, in `workItemStages` order, with no per-stage
    // attribute to key off. The index is therefore the stage, and a column count that is not five
    // means the board collapsed to its narrow single-column layout (work-board.tsx:64), where
    // "move between columns" is not a thing the interface offers.
    const columns = () => globalThis.$$("#todo-board > div > section");
    await globalThis.browser.waitUntil(
      async () => (await columns()).length > 0,
      { timeout: 60_000, timeoutMsg: "The work board never rendered its columns." },
    );
    const columnCount = (await columns()).length;
    if (columnCount !== workItemStages.length) {
      blocked.push(`work board: rendered ${columnCount} columns, so this window is on the compact single-stage layout`);
      this.skip();
    }

    // work-board.tsx:67 -- the header's first row holds the archived toggle and then New.
    const headerButtons = await globalThis.$$("#todo-board > header > div:first-child button");
    assert.equal(headerButtons.length, 2, "the work board header did not offer its two actions");
    await headerButtons[headerButtons.length - 1].click();
    const form = await globalThis.$("#todo-board form");
    await form.waitForDisplayed({ timeout: 20_000 });

    // work-board.tsx:89 -- the form fields are named for the FormData keys it reads, which is a
    // stable hook that owes nothing to the locale.
    await fill("The work item title field", await form.$('input[name="title"]'), workItemTitle);
    await (await form.$('button[type="submit"]')).click();
    await globalThis.browser.waitUntil(
      async () => (await globalThis.$$("#todo-board form")).length === 0,
      { timeout: 30_000, timeoutMsg: "The work item form stayed open after submitting." },
    );

    // Resolve the durable id first, then require the UI to render the card with that exact id.
    // Reading every heading through WebDriver is both slower and sensitive to a card rerender
    // between locating the article and locating its h3.
    workItemId = await globalThis.browser.waitUntil(async () => {
      const items = await invoke(({ core }) => core.invoke("list_work_items", { filters: { archived: false } }));
      return items.find((item) => item.title === workItemTitle)?.id ?? false;
    }, {
      timeout: 30_000,
      timeoutMsg: "The work item form closed without persisting the submitted item.",
    });
    await (await globalThis.$(`[data-testid="work-item-${workItemId}"]`)).waitForDisplayed({
      timeout: 30_000,
      timeoutMsg: "The persisted work item never appeared on the board.",
    });

    const stageOf = async () => {
      const sections = await columns();
      for (let index = 0; index < sections.length; index += 1) {
        if ((await sections[index].$$(`[data-testid="work-item-${workItemId}"]`)).length > 0) return index;
      }
      return -1;
    };
    assert.equal(
      await stageOf(),
      workItemStages.indexOf("inbox"),
      "a newly created work item did not land in the first column",
    );

    // work-board-card.tsx:33 -- the stage picker's option values are the raw stage ids, so driving
    // the real control needs no translated option text.
    const picker = await globalThis.$(`[data-testid="work-item-${workItemId}"] select`);
    await selectOption("The work item's stage picker", picker, "planned");
    await globalThis.browser.waitUntil(
      async () => (await stageOf()) === workItemStages.indexOf("planned"),
      { timeout: 30_000, timeoutMsg: "Choosing a stage did not move the card into that column." },
    );

    // work-board-card.tsx:32-36 -- the card's action row is [previous, next, edit, archive]; the
    // stage picker between the first two is a select, not a button.
    const actions = await globalThis.$$(`[data-testid="work-item-${workItemId}"] button`);
    assert.equal(actions.length, 4, "the work item card did not offer its four actions");
    await actions[1].click();
    await globalThis.browser.waitUntil(
      async () => (await stageOf()) === workItemStages.indexOf("in_progress"),
      { timeout: 30_000, timeoutMsg: "The next-stage control did not advance the card a column." },
    );
    assert.equal(
      (await (await columns())[workItemStages.indexOf("planned")].$$(`[data-testid="work-item-${workItemId}"]`)).length,
      0,
      "the card was rendered in two columns at once",
    );
  });

  globalThis.it("narrows the session list as the sidebar filter is typed", async function filterSessions() {
    if (!primarySessionId) {
      blocked.push("session filter: no session was created through the dialog, so there is nothing distinctive to filter for");
      this.skip();
    }

    const primary = (await invoke(({ core }) => core.invoke("list_sessions")))
      .find((session) => session.id === primarySessionId);
    assert.ok(primary, "the session created through the dialog is no longer listed");
    const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
      agentId: primary.agentId,
      interactionMode: "cli",
      title: secondaryTitle,
      folder: repository,
      projectPath: repository,
      remoteWorkspace: null,
      worktree: null,
    });
    await settleOperation(operation, "Creating the second session for the filter never settled.");
    const created = await globalThis.browser.waitUntil(async () => {
      const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
      return sessions.find((session) => session.title === secondaryTitle) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The second session was not created." });
    secondarySessionId = created.id;
    // `create_session` always activates what it creates (commands/sessions/mapper.rs:77). Handing
    // the active seat back keeps the reload below on the primary session, so the second one is a
    // list entry rather than another workspace with another PTY behind it.
    await invoke(({ core }, sessionId) => core.invoke("switch_session", { sessionId }), primarySessionId);

    // A session created over IPC does not exist as far as a page that already loaded its list is
    // concerned; without this reload the filter would be narrowing a list of one.
    await globalThis.browser.refresh();
    await bootstrapReady("React bootstrap did not become ready after seeding the second session.");
    const sessionsButton = await globalThis.$('nav.ucd-activity-bar button[aria-controls="workspace-session-sidebar"]');
    await sessionsButton.waitForClickable({ timeout: 20_000 });
    await sessionsButton.click();
    await waitForUrl("/workspace/sessions", "The activity bar did not return to the sessions destination.");
    await (await globalThis.$('[data-testid="session-sidebar"]')).waitForDisplayed({ timeout: 30_000 });

    // session-sidebar.tsx:268 -- the view switcher is [list, category, project], and the choice is
    // restored from the WebView's localStorage (session-sidebar.tsx:25). The grouped views render
    // their sections collapsed, so inheriting one would leave the sidebar showing no cards at all
    // and every count below would read as a filter that had already hidden everything.
    const views = await globalThis.$$('[data-testid="session-sidebar"] div.ucd-segmented button');
    assert.equal(views.length, 3, "the session sidebar did not offer its three list views");
    await views[0].click();

    await globalThis.browser.waitUntil(async () => {
      const ids = [];
      for (const card of await sessionCards()) ids.push(await card.getAttribute("data-session-id"));
      return ids.includes(primarySessionId) && ids.includes(secondarySessionId);
    }, { timeout: 60_000, timeoutMsg: "The sidebar never listed both sessions." });

    const unfiltered = (await sessionCards()).length;
    assert.ok(unfiltered >= 2, `the filter needs at least two listed sessions, saw ${unfiltered}`);

    // session-sidebar.tsx:266.
    const search = await globalThis.$("#workspace-session-search");
    await fill("The session filter", search, primaryToken);
    await globalThis.browser.waitUntil(async () => {
      const cards = await sessionCards();
      if (cards.length !== 1) return false;
      return (await cards[0].getAttribute("data-session-id")) === primarySessionId;
    }, {
      timeout: 30_000,
      timeoutMsg: `Filtering on ${primaryToken} did not narrow the list to the one matching session.`,
    });

    await fill("The session filter", search, `nomatch-${stamp}`);
    await globalThis.browser.waitUntil(
      async () => (await sessionCards()).length === 0,
      { timeout: 30_000, timeoutMsg: "A query matching nothing still left session cards on screen." },
    );

    await fill("The session filter", search, "");
    await globalThis.browser.waitUntil(
      async () => (await sessionCards()).length === unfiltered,
      { timeout: 30_000, timeoutMsg: "Clearing the filter did not restore the full session list." },
    );
  });

  globalThis.after(async () => {
    // Every step is tolerant: a throwing after hook marks the whole file failed and buries whatever
    // the tests actually found.
    const attempt = async (label, action) => {
      try {
        await action();
      } catch (reason) {
        globalThis.console.warn(`Cleanup step "${label}" failed: ${reason instanceof Error ? reason.message : String(reason)}`);
      }
    };

    // agent-terminal-tab.tsx:226 -- the trailing group in the terminal header is [clear, stop], and
    // stop is the only control that ends the CLI this file started. Without it the PTY outlives the
    // spec; the fixture lives outside the harness run root so that is survivable, but leaving a
    // child process running is not the state to hand the next run.
    if (primarySessionId) {
      await attempt("stop the agent terminal", async () => {
        await navigate(`/workspace/sessions/${encodeURIComponent(primarySessionId)}`);
        const chatTab = await globalThis.$('[aria-controls="session-tab-panel-chat"]');
        await chatTab.waitForClickable({ timeout: 20_000 });
        await chatTab.click();
        const controls = await globalThis.$$("#session-tab-panel-chat div.ml-auto button");
        if (controls.length === 2 && await controls[1].isEnabled()) await controls[1].click();
      });
    }
    if (workItemId) {
      await attempt("archive and delete the work item", async () => {
        await invoke(({ core }, id) => core.invoke("archive_work_item", { workItemId: id }), workItemId);
        await invoke(({ core }, id) => core.invoke("delete_work_item", { workItemId: id }), workItemId);
      });
    }
    for (const sessionId of [primarySessionId, secondarySessionId].filter(Boolean)) {
      await attempt(`delete session ${sessionId}`, () => invoke(
        ({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId,
      ));
    }
    // Reload before parking: the page still holds the deleted sessions in its query cache, and the
    // route reconciler would bounce straight back to one of them (use-workspace-session-route.ts:52).
    await attempt("return to the session list", async () => {
      await globalThis.browser.refresh();
      await bootstrapReady("React bootstrap did not become ready during cleanup.");
      await navigate("/workspace/sessions");
    });
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
