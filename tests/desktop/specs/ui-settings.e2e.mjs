import assert from "node:assert/strict";
import { fill, selectOption } from "../helpers/form-control.mjs";

/**
 * Interactive coverage of the Settings surface: real clicks on real widgets.
 *
 * The `domain-*` specs in this directory drive `core.invoke` and prove the commands work. That
 * leaves a gap this file exists to close -- a command can be perfect while no button is wired to
 * it, while a dialog silently swallows its submit, or while the "cancel" path writes anyway. So
 * every assertion here is against rendered state, and every mutation goes through the widget a
 * user would touch.
 *
 * Selector policy: structural attributes only. The app is localized and its default language on
 * this host is zh-CN, so a text selector would be a locale assertion in disguise. Where no
 * `data-testid` exists the anchors are `role`, `data-dialog-autofocus`, `data-skill-id`, the
 * `hidden` attribute the Settings shell uses to park inactive pages, and the `lucide-*` class
 * lucide-react stamps on every icon (`node_modules/lucide-react/dist/cjs/lucide-react.js:103-110`
 * builds it from the icon name, so it is a stable, translation-free handle on the button that
 * carries that icon). Each one cites the file:line it was read from.
 */

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

/**
 * The Settings shell keeps every *visited* page mounted and hides the inactive ones with the
 * `hidden` attribute (settings-shell.tsx:45-61). "What is on screen" is therefore the single
 * un-hidden panel, not "the last thing that rendered" -- scoping every query to it is what stops
 * a selector from matching the identical widget on a page an earlier test left in the DOM.
 * `main > div` is unambiguous because the top bar is a `<header>` (settings-topbar.tsx:17).
 */
const PANEL = "main > div > section > div:not([hidden])";
/** Every dialog in the app funnels through this one primitive (application-dialog.tsx:78-88). */
const DIALOG = 'section[role="dialog"][aria-modal="true"]';

/** types/settings.ts:8 -- the font-size select's option values, and its locale-free fingerprint. */
const FONT_SIZES = ["12px", "14px", "16px", "18px"];

// Names carry the spec so a leaked record is attributable. The MCP name also has to satisfy the
// kebab-case rule at mcp/mcp-server-validation.ts:28, and the hook id the 3..=64 lowercase-ASCII
// rule at prompt_hooks/domain/identity.rs:9-16.
const MCP_SERVER = "vanehub-ui-settings-e2e";
const MCP_ABANDONED_SERVER = "vanehub-ui-settings-escape-e2e";
const HOOK_ID = "vanehub-ui-settings-e2e";
const HOOK_NAME = "UI Settings E2E Hook";
const SKILL_ID = "vanehub-ui-settings-e2e";
const SETTINGS_NAVIGATION_ICONS = {
  basic: "settings",
  mcp: "boxes",
  "prompt-hooks": "workflow",
  skills: "puzzle",
};

const blocked = [];

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

async function waitForBootstrap() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
  assert.equal(
    await root.getAttribute("data-vanehub-fatal-error"),
    null,
    "the app came up inside its fatal error boundary",
  );
}

/**
 * Routes to a Settings page and waits until `probe` recognises the un-hidden panel as that page.
 *
 * Waiting on the anchor alone is not enough: react-router commits history-driven navigations
 * inside `startTransition`, so for a beat the *previous* page is still the un-hidden one. A probe
 * that the previous page also satisfies would return a handle to the wrong screen, and every
 * assertion after it would describe a page the test never opened.
 */
async function openSettings(section, description, probe, ...args) {
  await navigate(`/settings?section=${section}`);
  const navigationIcon = SETTINGS_NAVIGATION_ICONS[section];
  assert.ok(navigationIcon, `No Settings navigation icon is registered for ${section}.`);
  const navigationIconElement = await globalThis.$(`aside nav button svg.lucide-${navigationIcon}`);
  await navigationIconElement.waitForExist({
    timeout: 60_000,
    timeoutMsg: `The ${description} Settings navigation button never mounted.`,
  });
  const navigationButton = await pick(
    `the ${description} Settings navigation button`,
    "aside nav button",
    iconButtonLocator,
    `svg.lucide-${navigationIcon}`,
  );
  await navigationButton.click();
  await globalThis.browser.waitUntil(async () => {
    const panels = await globalThis.$$(PANEL);
    if (panels.length !== 1) return false;
    return (await globalThis.browser.execute(probe, PANEL, ...args)) === true;
  }, {
    // Generous: `LazyFeature` fetches the page chunk on first visit (settings-shell.tsx:57).
    timeout: 60_000,
    timeoutMsg: `The ${description} Settings page never became the visible panel.`,
  });
}

/**
 * WDIO handle for the one element a page-side predicate picks out of `selector`'s matches.
 *
 * `script` receives `selector` and returns the *indices* it matched; WebDriver and
 * `querySelectorAll` both walk the document in order, so an index transfers between them. The
 * exactly-one assertion is the point: an index-only locator would keep working after a button was
 * added next to the one being targeted, and would then click the wrong thing without a word.
 */
async function pick(description, selector, script, ...args) {
  const matched = await globalThis.browser.execute(script, selector, ...args);
  assert.ok(Array.isArray(matched), `the locator for ${description} returned no match list`);
  assert.equal(matched.length, 1, `expected exactly one ${description}, matched ${matched.length}`);
  const elements = await globalThis.$$(selector);
  const element = elements[matched[0]];
  assert.ok(element, `${description} disappeared between being located and being used`);
  return element;
}

/**
 * Locator for the button among `selector`'s matches that carries `icon`, e.g. "svg.lucide-plus".
 *
 * `icon` arrives as an argument rather than a closed-over constant because `pick` ships this
 * function to the page as source: nothing outside its own parameters exists on the other side.
 */
const iconButtonLocator = (selector, icon) => Array.from(globalThis.document.querySelectorAll(selector))
  .flatMap((button, index) => (button.querySelector(icon) ? [index] : []));

// Only dialogs that are on screen count. The Settings shell parks the pages you are not on with
// the `hidden` attribute rather than unmounting them, so a dialog left open by a failing case
// stays in the document and makes the next case's count read 2 -- reporting "the dialog did not
// open" about a dialog that is plainly open in the failure screenshot.
const openDialogCount = async () => globalThis.browser.execute((selector) => (
  Array.from(globalThis.document.querySelectorAll(selector))
    .filter((dialog) => dialog.checkVisibility()).length
), DIALOG);

async function waitForDialog(description, expected) {
  await globalThis.browser.waitUntil(async () => (await openDialogCount()) === expected, {
    timeout: 20_000,
    timeoutMsg: `Expected ${expected} open dialog(s) for ${description}.`,
  });
}

/** Whatever error banner the open dialog is currently showing, or "" for none. */
async function dialogBanner() {
  return globalThis.browser.execute((dialogSelector) => {
    const dialog = globalThis.document.querySelector(dialogSelector);
    // mcp/mcp-server-form.tsx:150 and prompt-hooks/prompt-hook-dialogs.tsx:113-118 use the
    // `ucd-status-danger` utility (styles.css:454); skills/skill-dialogs.tsx:147 uses role=alert.
    const error = dialog?.querySelector('[class*="ucd-status-danger"], [role="alert"]');
    return error ? (error.textContent ?? "").trim() : "";
  }, DIALOG);
}

/**
 * Waits for the dialog a submit was just made in to close, reporting the message it is showing
 * instead of a bare timeout when it refuses to. That distinction is the whole point of this file:
 * "the dialog is still open and says the id is taken" and "the save button is not wired to
 * anything" both look like a hung test otherwise.
 *
 * `stale` is the banner the dialog was already displaying before the click. Both forms clear their
 * error only once the next submit begins, so without it a rejected *earlier* attempt gets reported
 * as this one's failure.
 */
async function awaitDialogClose(description, stale = "") {
  const outcome = await globalThis.browser.waitUntil(async () => {
    if ((await openDialogCount()) === 0) return "closed";
    const banner = await dialogBanner();
    return banner.length > 0 && banner !== stale ? banner : false;
  }, { timeout: 30_000, timeoutMsg: `${description} neither closed nor reported a new error.` });
  assert.equal(outcome, "closed", `${description} was refused: ${outcome}`);
}

/**
 * Locator for the MCP form's text inputs. With the stdio transport selected those are, in document
 * order, name (mcp/mcp-server-form.tsx:88), description (:114) and command (:121); the enabled
 * checkbox (:107) is the only other `input` and is filtered out by type.
 */
const MCP_TEXT_FIELD = { name: 0, description: 1, command: 2 };
const mcpTextInputLocator = (selector, position) => {
  const all = Array.from(globalThis.document.querySelectorAll(selector));
  const text = all.filter((input) => input.type === "text");
  return text.length === 3 ? [all.indexOf(text[position])] : [];
};

/**
 * The validation message rendered against each MCP text field, in `MCP_TEXT_FIELD` order, with an
 * empty string for a field that is not currently failing.
 *
 * mcp/mcp-server-form.tsx:86-148 gives every field a `<label>` holding a caption `<span>`, its
 * control, and a *second* `<span>` that exists only while that field failed validation. Counting
 * the spans is the structural test for "this field is showing an error" -- no dependency on the
 * message text, which is translated, or on the Tailwind colour class, which is not a contract.
 */
async function mcpFieldErrors() {
  return globalThis.browser.execute((dialogSelector) => {
    const dialog = globalThis.document.querySelector(dialogSelector);
    if (!dialog) return null;
    return Array.from(dialog.querySelectorAll("input"))
      .filter((input) => input.type === "text")
      .map((input) => {
        const label = input.closest("label");
        const spans = label ? Array.from(label.children).filter((child) => child.tagName === "SPAN") : [];
        return spans.length > 1 ? (spans[spans.length - 1].textContent ?? "").trim() : "";
      });
  }, DIALOG);
}

/** mcp/mcp-server-card.tsx:36-41 -- one `section.ucd-panel` per server, `h3` holding its name. */
async function mcpCardNames() {
  return globalThis.browser.execute(
    (scope) => Array.from(globalThis.document.querySelectorAll(`${scope} section.ucd-panel h3`))
      .map((heading) => (heading.textContent ?? "").trim()),
    PANEL,
  );
}

/**
 * prompt-hooks/prompt-hook-card-list.tsx renders one `article[data-hook-id]` per hook. The list
 * virtualizes only above 500 hooks, so every compact row is really in the DOM here.
 */
async function promptHookCardIds() {
  return globalThis.browser.execute(
    (scope) => Array.from(globalThis.document.querySelectorAll(`${scope} article[data-hook-id]`))
      .map((row) => row.getAttribute("data-hook-id") ?? ""),
    PANEL,
  );
}

const listMcpServers = () => invoke(({ core }) => core.invoke("list_mcp_servers"));
const listPromptHooks = () => invoke(({ core }) => core.invoke("list_prompt_hooks"));
const globalSkillScope = { scope: "global", workspacePath: null };
const listGlobalSkills = () => invoke(
  ({ core }, input) => core.invoke("list_skills", { input }),
  globalSkillScope,
);

globalThis.describe("VaneHub AI desktop Settings interactions", () => {
  globalThis.before(async () => {
    // Reload rather than inherit. Spec files in one run share this WebView, and the previous one
    // can leave the page mid-flow, on any route, with dialogs open. Without this the first
    // `pushState` route change lands on a page that is already showing something else.
    await globalThis.browser.refresh();
    await waitForBootstrap();
  });

  globalThis.it("persists a Basic setting changed through its control across a reload", async function basicSetting() {
    let settings;
    try {
      settings = await invoke(({ core }) => core.invoke("get_settings"));
    } catch (error) {
      blocked.push(`Basic settings: get_settings failed (${error instanceof Error ? error.message : String(error)})`);
      this.skip();
    }
    const original = settings.fontSize;
    const target = FONT_SIZES.find((size) => size !== original);
    assert.ok(target, `no alternative font size to switch to from ${original}`);

    // The font-size select is identified by its option values, which are literal CSS lengths
    // (basic-settings-page.tsx:153-161 maps `appFontSizes` to both label and value). That is the
    // one control on this page whose identity survives a change of application language.
    const isBasicPage = (scope, sizes) => Array.from(globalThis.document.querySelectorAll(`${scope} select`))
      .some((select) => Array.from(select.options).map((option) => option.value).join() === sizes.join());
    const fontSizeLocator = (selector, sizes) => Array.from(globalThis.document.querySelectorAll(selector))
      .flatMap((select, index) => (
        Array.from(select.options).map((option) => option.value).join() === sizes.join() ? [index] : []
      ));
    const fontSizeSelect = () => pick("the Basic font-size select", `${PANEL} select`, fontSizeLocator, FONT_SIZES);

    await openSettings("basic", "Basic", isBasicPage, FONT_SIZES);
    const before = await fontSizeSelect();
    assert.equal(
      await before.getProperty("value"),
      original,
      "the select rendered a font size the backend does not hold",
    );

    // Every control on the page is disabled while a save is in flight (basic-settings-page.tsx:92),
    // so a click that lands during one is silently dropped.
    await globalThis.browser.waitUntil(async () => (await fontSizeSelect()).isEnabled(), {
      timeout: 30_000,
      timeoutMsg: "The Basic settings form never became interactive.",
    });
    // Re-queried rather than reusing the handle above: the panel re-renders while settings load,
    // and a detached handle fails as "element not interactable" rather than as a product problem.
    await selectOption("The font-size select", await fontSizeSelect(), target);
    await globalThis.browser.waitUntil(async () => (await fontSizeSelect()).isEnabled(), {
      timeout: 30_000,
      timeoutMsg: "The Basic settings form never re-enabled after the save.",
    });

    // Proves the widget reached the command rather than only re-rendering its own local state.
    const saved = await invoke(({ core }) => core.invoke("get_settings"));
    assert.equal(saved.fontSize, target, "the select changed on screen but nothing was saved");

    await globalThis.browser.refresh();
    await waitForBootstrap();
    await openSettings("basic", "Basic", isBasicPage, FONT_SIZES);
    assert.equal(
      await (await fontSizeSelect()).getProperty("value"),
      target,
      "the changed font size did not survive a reload",
    );

    // Restored through the same control, so the restore is itself covered rather than assumed.
    await globalThis.browser.waitUntil(async () => (await fontSizeSelect()).isEnabled(), {
      timeout: 30_000,
      timeoutMsg: "The Basic settings form never became interactive again.",
    });
    await selectOption("The font-size select", await fontSizeSelect(), original);
    await globalThis.browser.waitUntil(
      async () => (await (await fontSizeSelect()).getProperty("value")) === original,
      { timeout: 30_000, timeoutMsg: `The font-size select never returned to ${original}.` },
    );
    const restored = await invoke(({ core }) => core.invoke("get_settings"));
    assert.equal(restored.fontSize, original, "the font size was not restored for the next spec");
  });

  globalThis.it("adds, validates and deletes an MCP server entirely through the UI", async function mcpLifecycle() {
    let existing;
    try {
      existing = await listMcpServers();
    } catch (error) {
      blocked.push(`MCP: list_mcp_servers failed (${error instanceof Error ? error.message : String(error)})`);
      this.skip();
    }
    assert.ok(
      !existing.some((server) => server.name === MCP_SERVER),
      `${MCP_SERVER} is left over from an earlier run; the isolated data directory is dirty`,
    );

    // settings-pages.ts:163 gives the MCP page the `Boxes` icon, rendered by page-parts.tsx:23-26.
    const isMcpPage = (scope) => globalThis.document.querySelector(`${scope} svg.lucide-boxes`) !== null;
    await openSettings("mcp", "MCP", isMcpPage);

    // mcp-page.tsx:229-232 -- the only `Plus` on this page opens the add-server form.
    const addButton = await pick(
      "the MCP add-server button",
      `${PANEL} button`,
      iconButtonLocator,
      "svg.lucide-plus",
    );
    await addButton.click();
    await waitForDialog("the MCP add-server form", 1);

    // mcp/mcp-server-form.tsx:88 -- the name input is the dialog's autofocus target.
    const nameInput = () => globalThis.$(`${DIALOG} [data-dialog-autofocus]`);
    const commandInput = () => pick(
      "the MCP command input",
      `${DIALOG} input`,
      mcpTextInputLocator,
      MCP_TEXT_FIELD.command,
    );
    // mcp/mcp-server-form.tsx:154-157 -- the submit button is the one carrying the Save icon.
    const submit = () => pick("the MCP form submit button", `${DIALOG} button`, iconButtonLocator, "svg.lucide-save");

    await (await nameInput()).waitForExist({ timeout: 10_000 });
    assert.deepEqual(
      await mcpFieldErrors(),
      ["", "", ""],
      "the add-server form opened already showing validation errors",
    );

    // First invalid entry: a name that breaks the kebab-case rule (mcp-server-validation.ts:28).
    // Only the name field is asserted. The run shows the form raising the missing-command error in
    // the same pass, so the two are not mutually exclusive the way reading `superRefine` alone
    // suggests -- which is exactly why the second pass below is what proves the errors are
    // per-field rather than one banner repeated.
    await fill("the MCP server name", await nameInput(), "Invalid Name");
    await (await submit()).click();
    await globalThis.browser.waitUntil(async () => {
      const errors = await mcpFieldErrors();
      return errors !== null && errors[MCP_TEXT_FIELD.name].length > 0;
    }, { timeout: 20_000, timeoutMsg: "The invalid MCP name produced no inline validation on its field." });
    assert.equal(await openDialogCount(), 1, "the invalid submit closed the form instead of rejecting it");
    assert.ok(
      !(await listMcpServers()).some((server) => server.name === "Invalid Name"),
      "the rejected entry was written anyway",
    );

    // Second invalid entry: a valid name with no command, which the stdio transport requires
    // (mcp-server-validation.ts:40-46). This is the check that the message lands on the *right*
    // field -- the first pass cannot tell a per-field error apart from a single shared banner.
    await fill("the MCP server name", await nameInput(), MCP_SERVER);
    await (await submit()).click();
    const commandOnly = await globalThis.browser.waitUntil(async () => {
      const errors = await mcpFieldErrors();
      return errors !== null && errors[MCP_TEXT_FIELD.command].length > 0 ? errors : false;
    }, { timeout: 20_000, timeoutMsg: "The missing MCP command produced no inline validation on its field." });
    assert.equal(
      commandOnly[MCP_TEXT_FIELD.name],
      "",
      "the corrected name is still flagged, so the errors are not per-field",
    );
    assert.equal(await openDialogCount(), 1, "the incomplete submit closed the form instead of rejecting it");
    assert.ok(
      !(await listMcpServers()).some((server) => server.name === MCP_SERVER),
      "the incomplete entry was written anyway",
    );

    // Completed in place in the same dialog, so the save path is exercised on a corrected form.
    await fill("the MCP command", await commandInput(), "node");
    const staleBanner = await dialogBanner();
    await (await submit()).click();
    await awaitDialogClose("the MCP add-server form", staleBanner);
    await globalThis.browser.waitUntil(async () => (await mcpCardNames()).includes(MCP_SERVER), {
      timeout: 30_000,
      timeoutMsg: "The saved MCP server never appeared as a card in the list.",
    });
    const persisted = (await listMcpServers()).find((server) => server.name === MCP_SERVER);
    assert.ok(persisted, "the card rendered but no server was stored");
    assert.equal(persisted.command, "node", "the command typed into the form was not the one stored");

    // Delete through the card's own action, then through the confirmation it raises.
    const deleteButton = await pick(
      "the MCP delete button on the created card",
      `${PANEL} section.ucd-panel button`,
      (selector, name) => Array.from(globalThis.document.querySelectorAll(selector)).flatMap((button, index) => {
        const card = button.closest("section");
        const matches = (card?.querySelector("h3")?.textContent ?? "").trim() === name
          && button.querySelector("svg.lucide-trash-2") !== null;
        return matches ? [index] : [];
      }),
      MCP_SERVER,
    );
    await deleteButton.click();
    await waitForDialog("the MCP delete confirmation", 1);
    // use-confirmation.tsx:50-56 -- the confirm action is the dialog's autofocus target.
    const confirmDelete = await globalThis.$(`${DIALOG} button[data-dialog-autofocus]`);
    await confirmDelete.waitForClickable({ timeout: 10_000 });
    await confirmDelete.click();

    await waitForDialog("the dismissed MCP delete confirmation", 0);
    await globalThis.browser.waitUntil(async () => !(await mcpCardNames()).includes(MCP_SERVER), {
      timeout: 30_000,
      timeoutMsg: "The deleted MCP server is still rendered as a card.",
    });
    assert.ok(
      !(await listMcpServers()).some((server) => server.name === MCP_SERVER),
      "the card disappeared but the server is still stored",
    );
  });

  globalThis.it("creates and removes a Prompt Hook through its dialogs", async function promptHookLifecycle() {
    let existing;
    try {
      existing = await listPromptHooks();
    } catch (error) {
      blocked.push(`Prompt Hooks: list_prompt_hooks failed (${error instanceof Error ? error.message : String(error)})`);
      this.skip();
    }
    assert.ok(
      !existing.hooks.some((hook) => hook.id === HOOK_ID),
      `${HOOK_ID} is left over from an earlier run; the isolated data directory is dirty`,
    );

    // settings-pages.ts:190 gives the Prompt Hooks page the `Workflow` icon.
    const isPromptHooksPage = (scope) => globalThis.document.querySelector(`${scope} svg.lucide-workflow`) !== null;
    await openSettings("prompt-hooks", "Prompt Hooks", isPromptHooksPage);

    // prompt-hooks-page.tsx:180-183 -- the only `Plus` on this page opens the create dialog.
    const createButton = await pick(
      "the Prompt Hook create button",
      `${PANEL} button`,
      iconButtonLocator,
      "svg.lucide-plus",
    );
    await createButton.click();
    await waitForDialog("the Prompt Hook create dialog", 1);

    // prompt-hooks/prompt-hook-dialogs.tsx:82-83 -- the id field is rendered immediately before
    // the autofocused name field, and 82 labels it with the untranslated literal "ID".
    const idInput = await pick(
      "the Prompt Hook id input",
      `${DIALOG} input`,
      (selector) => {
        const inputs = Array.from(globalThis.document.querySelectorAll(selector));
        const name = inputs.findIndex((input) => input.hasAttribute("data-dialog-autofocus"));
        return name > 0 ? [name - 1] : [];
      },
    );
    await fill("the Prompt Hook id", idInput, HOOK_ID);
    await fill("the Prompt Hook name", await globalThis.$(`${DIALOG} [data-dialog-autofocus]`), HOOK_NAME);
    // prompt-hooks/prompt-hook-dialogs.tsx:92-95 -- one textarea, holding the template body.
    await fill("the Prompt Hook body", await globalThis.$(`${DIALOG} textarea`), "UI settings e2e template body.");

    // prompt-hooks/prompt-hook-dialogs.tsx:101-104 -- the action row is exactly cancel then save.
    const dialogActions = async () => globalThis.$$(`${DIALOG} button`);
    let actions = await dialogActions();
    assert.equal(actions.length, 2, `expected cancel and save, found ${actions.length} dialog buttons`);
    await actions[1].click();

    // The dialog's defaults are left alone deliberately: (stage, category, order) is a unique slot
    // (prompt_hooks/domain/ordering.rs:42-49) and the draft's per-turn/dynamic/500
    // (prompt-hook-dialogs.tsx:162-175) is free -- the built-in at order 500 is `navigation`
    // and the `dynamic` built-in sits at 400 (prompt_hooks/domain/catalog.rs:55-68). If that ever
    // stops being true the refusal now arrives as the dialog's own message, not as a timeout.
    await awaitDialogClose("the Prompt Hook create dialog");
    await globalThis.browser.waitUntil(async () => (await promptHookCardIds()).includes(HOOK_ID), {
      timeout: 30_000,
      timeoutMsg: "The created Prompt Hook never appeared in the list.",
    });
    const created = (await listPromptHooks()).hooks.find((hook) => hook.id === HOOK_ID);
    assert.ok(created, "the card rendered but no hook was stored");
    assert.equal(created.name, HOOK_NAME, "the name typed into the dialog was not the one stored");

    const cardDelete = async () => {
      const row = `${PANEL} article[data-hook-id="${HOOK_ID}"]`;
      const summary = await (await globalThis.$(row)).$("summary");
      assert.ok(summary, "the Prompt Hook overflow summary is missing");
      await summary.click();
      // The overflow's delete button carries Trash2 (prompt-hook-card-list.tsx:209-210). It used
      // to be located by the English text "Delete", which this file's own selector policy rules
      // out and which never matched: the label is localized and this host renders zh-CN.
      return pick("the Prompt Hook card delete button", `${row} button`, iconButtonLocator, "svg.lucide-trash-2");
    };

    // Cancelling the confirmation must leave the hook alone -- a delete dialog whose cancel still
    // deletes is exactly the wiring bug a command-level test cannot see.
    await (await cardDelete()).click();
    await waitForDialog("the Prompt Hook delete confirmation", 1);
    // prompt-hooks/prompt-hook-dialogs.tsx:70-73 -- cancel then delete.
    actions = await dialogActions();
    assert.equal(actions.length, 2, `expected cancel and delete, found ${actions.length} dialog buttons`);
    await actions[0].click();
    await waitForDialog("the cancelled Prompt Hook delete confirmation", 0);
    assert.ok(
      (await promptHookCardIds()).includes(HOOK_ID),
      "cancelling the delete confirmation removed the card anyway",
    );
    assert.ok(
      (await listPromptHooks()).hooks.some((hook) => hook.id === HOOK_ID),
      "cancelling the delete confirmation deleted the hook anyway",
    );

    await (await cardDelete()).click();
    await waitForDialog("the Prompt Hook delete confirmation", 1);
    actions = await dialogActions();
    assert.equal(actions.length, 2, `expected cancel and delete, found ${actions.length} dialog buttons`);
    await actions[1].click();

    await awaitDialogClose("the Prompt Hook delete confirmation");
    await globalThis.browser.waitUntil(async () => !(await promptHookCardIds()).includes(HOOK_ID), {
      timeout: 30_000,
      timeoutMsg: "The deleted Prompt Hook is still rendered in the list.",
    });
    assert.ok(
      !(await listPromptHooks()).hooks.some((hook) => hook.id === HOOK_ID),
      "the card disappeared but the hook is still stored",
    );
  });

  globalThis.it("discards an MCP entry dismissed with Escape", async function escapeDismissal() {
    try {
      await listMcpServers();
    } catch (error) {
      blocked.push(`MCP Escape: list_mcp_servers failed (${error instanceof Error ? error.message : String(error)})`);
      this.skip();
    }

    const isMcpPage = (scope) => globalThis.document.querySelector(`${scope} svg.lucide-boxes`) !== null;
    await openSettings("mcp", "MCP", isMcpPage);
    const addButton = await pick(
      "the MCP add-server button",
      `${PANEL} button`,
      iconButtonLocator,
      "svg.lucide-plus",
    );
    await addButton.click();
    await waitForDialog("the MCP add-server form", 1);

    // Filled with an entry that *would* save -- a valid kebab-case name and a command -- so the
    // assertion is about the dismissal rather than about the form having rejected the input.
    await fill(
      "the MCP server name",
      await globalThis.$(`${DIALOG} [data-dialog-autofocus]`),
      MCP_ABANDONED_SERVER,
    );
    await fill(
      "the MCP command",
      await pick("the MCP command input", `${DIALOG} input`, mcpTextInputLocator, MCP_TEXT_FIELD.command),
      "node",
    );
    assert.deepEqual(await mcpFieldErrors(), ["", "", ""], "the abandoned entry was already invalid");

    // application-dialog.tsx:40-43 -- Escape is handled on `document` for the topmost modal, so it
    // is delivered from whichever field currently holds focus.
    await globalThis.browser.keys(["Escape"]);
    await waitForDialog("the Escape-dismissed MCP form", 0);

    assert.ok(
      !(await mcpCardNames()).includes(MCP_ABANDONED_SERVER),
      "the Escape-dismissed entry was rendered as a card",
    );
    assert.ok(
      !(await listMcpServers()).some((server) => server.name === MCP_ABANDONED_SERVER),
      "the Escape-dismissed entry was written to storage",
    );
  });

  globalThis.it("discards a Skill cancelled from its dialog", async function cancelDismissal() {
    let existing;
    try {
      existing = await listGlobalSkills();
    } catch (error) {
      blocked.push(`Skills: list_skills failed (${error instanceof Error ? error.message : String(error)})`);
      this.skip();
    }
    assert.ok(
      !existing.skills.some((skill) => skill.id === SKILL_ID),
      `${SKILL_ID} is left over from an earlier run`,
    );

    // settings-pages.ts:172 gives the Skills page the `Puzzle` icon.
    const isSkillsPage = (scope) => globalThis.document.querySelector(`${scope} svg.lucide-puzzle`) !== null;
    await openSettings("skills", "Skills", isSkillsPage);

    // skills-page.tsx:73 -- of the three header actions only the create one carries `Plus`.
    const createButton = await pick(
      "the Skill create button",
      `${PANEL} button`,
      iconButtonLocator,
      "svg.lucide-plus",
    );
    await createButton.click();
    await waitForDialog("the Skill create dialog", 1);

    // skills/skill-dialogs.tsx:113-124 -- two tab buttons, then cancel, then save. The create form
    // has no autofocus target, so the fields are anchored on document order instead: 117 renders
    // the id field first and the name field second.
    const dialogButtons = () => globalThis.$$(`${DIALOG} button`);
    assert.equal(
      (await dialogButtons()).length,
      4,
      "expected two tabs plus cancel and save in the Skill create dialog",
    );
    const fields = await globalThis.$$(`${DIALOG} input`);
    assert.ok(fields.length >= 2, `expected the Skill metadata inputs, found ${fields.length}`);
    await fill("the Skill id", fields[0], SKILL_ID);
    await fill("the Skill name", fields[1], "UI Settings E2E Skill");

    const buttons = await dialogButtons();
    await buttons[buttons.length - 2].click();
    await waitForDialog("the cancelled Skill create dialog", 0);

    // skills/skill-card-list.tsx:87 and :123 -- every rendered Skill row carries `data-skill-id`.
    const rendered = await globalThis.$$(`${PANEL} article[data-skill-id="${SKILL_ID}"]`);
    assert.equal(rendered.length, 0, "the cancelled Skill was rendered in the inventory");
    // Recorded rather than asserted: if the inventory drew no rows at all the DOM check above was
    // vacuous, and the run should say so instead of counting it as coverage. The storage check
    // below is what actually carries this case either way.
    const inventoryRows = await globalThis.$$(`${PANEL} article[data-skill-id]`);
    if (existing.skills.length > 0 && inventoryRows.length === 0) {
      blocked.push("Skills: the inventory rendered no rows, so the cancelled-Skill DOM check was vacuous");
    }
    assert.ok(
      !(await listGlobalSkills()).skills.some((skill) => skill.id === SKILL_ID),
      "the cancelled Skill was written to storage",
    );
  });

  globalThis.after(async () => {
    // Safety nets, not the coverage: each record above is removed by the test that created it, and
    // these only matter when an assertion failed mid-flow. Spec files in this run share one data
    // directory, so a leaked MCP server or hook would show up as the next file's "dirty directory"
    // failure -- and a leaked global Skill would land in the real user home rather than the
    // isolated directory (skills/infrastructure/filesystem/paths.rs:31,106-111).
    for (const name of [MCP_SERVER, MCP_ABANDONED_SERVER]) {
      await invoke(({ core }, value) => core.invoke("remove_mcp_server", { name: value }), name)
        .catch(() => {});
    }
    await invoke(({ core }, hookId) => core.invoke("delete_prompt_hook", { hookId }), HOOK_ID)
      .catch(() => {});
    await invoke(
      ({ core }, args) => core.invoke("delete_skill", args),
      { skillId: SKILL_ID, input: globalSkillScope },
    ).catch(() => {});

    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }

    // The app reopens its last destination on relaunch and these specs share a data directory, so
    // parking on Settings would decide where the next file starts. `exit_application` is
    // deliberately not called here: it races WDIO's `deleteSession` and discards this file's
    // per-test results.
    await navigate("/workspace/sessions");
    await globalThis.browser.waitUntil(
      async () => (await globalThis.browser.getUrl()).includes("/workspace/sessions"),
      { timeout: 20_000, timeoutMsg: "The WebView did not return to the sessions workspace." },
    ).catch(() => {});
  });
});
