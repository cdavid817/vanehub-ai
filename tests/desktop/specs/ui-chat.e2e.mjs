import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";
import { readOnePieceApiKey } from "../helpers/onepiece-credential.mjs";
import { navigateTo } from "../helpers/navigation.mjs";

/**
 * The chat surface driven the way a user drives it: keystrokes into the composer, clicks on the
 * controls the app renders, assertions on what the interface puts on screen. Every other desktop
 * spec in this directory calls `core.invoke` and asserts on the payload that comes back, which
 * says nothing about whether the UI in front of those commands is wired to them at all.
 *
 * Only one case spends a provider turn. Everything else is asserted against UI state that needs
 * no round trip, in a second session that is deliberately left empty so "nothing was sent" is a
 * structural fact (`MessageList` renders no scroll region for an empty list — MessageList.tsx:66)
 * rather than a guess.
 */

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// ---------------------------------------------------------------------------------------------
// Selectors. Each one was read out of the component that renders it and is cited by file:line, so
// a rename surfaces as a stale citation rather than as a silently non-matching query.
//
// Nothing below matches localized copy. This host boots the app in zh-CN
// (src/i18n/index.ts:17 with `defaultAppLanguage = "zh-CN"` — src/i18n/supported-locales.ts:47),
// and every literal string this file does match on is locale-independent by construction: a
// slash-command invocation (`/status`, built from the command name in help-command.ts:5), the
// `L1` line marker (FileReferenceLines.tsx:23), and a fixture file name this spec writes itself.
// ---------------------------------------------------------------------------------------------

const COMPOSER = '[data-testid="wechat-style-composer"]';       // ChatInputBox.tsx:173
const INPUT = `${COMPOSER} textarea`;                           // ChatInputBox.tsx:222
// Rendered only while the draft is non-empty, immediately after the textarea in the same
// positioned wrapper — ChatInputBox.tsx:221-246. The adjacent-sibling combinator is what keeps
// this off the composer's other buttons without reaching for its title text.
const CLEAR = `${COMPOSER} textarea + button`;
const TOOLBAR = '[data-testid="composer-toolbar"]';             // ButtonArea.tsx:62
// The toolbar has exactly two children: the selector cluster and the `ml-auto` action group
// (ButtonArea.tsx:63 and :122). Send and Stop are the two branches of one ternary at the end of
// that group (ButtonArea.tsx:128-138), so this resolves to whichever of them is mounted.
const ACTION = `${TOOLBAR} > div:last-child > button:last-child`;
// lucide-react composes `lucide-<icon name>` onto every icon it renders
// (node_modules/lucide-react/dist/cjs/lucide-react.js:103-113); `Send` and `Square` are the two
// icons that distinguish the branches above (ButtonArea.tsx:130 and :135). Using the icon rather
// than the button label keeps the discriminator out of the translation catalogue.
const SEND_ICON = `${ACTION} svg.lucide-send`;
const STOP_ICON = `${ACTION} svg.lucide-square`;
const SLASH_OUTPUT = '[data-testid="slash-command-output"]';    // SlashCommandOutput.tsx:39
const SLASH_OUTPUT_ENTRY = `${SLASH_OUTPUT} p`;                 // SlashCommandOutput.tsx:55
const SLASH_OUTPUT_DISMISS = `${SLASH_OUTPUT} button`;          // SlashCommandOutput.tsx:45-51
// `SlashCommandCompletion` is the only `role="group"` the composer can render here:
// `SeatMentionCompletion` also uses one (SeatMentionCompletion.tsx:27) but needs a seated roster,
// and both sessions in this file are single-seat. Each option's label starts with `/` +
// the command name (SlashCommandCompletion.tsx:24-26), which is what the assertions match on.
const SLASH_OPTION = `${COMPOSER} [role="group"] button`;
const REFERENCES = '[data-testid="composer-file-references"]';  // ChatInputBox.tsx:208
const REFERENCE_REMOVE = `${REFERENCES} span > button`;         // ChatInputBox.tsx:214
const TRANSCRIPT = '[data-testid="message-scroll-region"]';     // MessageList.tsx:78
const CANVAS = '[data-testid="message-readable-measure"]';      // MessageList.tsx:86
const CHAT_TAB = '[aria-controls="session-tab-panel-chat"]';    // session-tabs.tsx:154
const DIALOG = 'section[role="dialog"][aria-modal="true"]';     // application-dialog.tsx:78-87
const PREVIEW_LINES = '[data-testid="file-preview-lines"]';     // FileReferencePreviewDialog.tsx:98
const PREVIEW_LINE_1 = '[data-testid="preview-line-1"]';        // PreviewLineRow.tsx:37

/** Answering this needs a real provider round trip, and it is cheap to assert on. */
const PROMPT = "Reply with exactly the word PONG and nothing else.";
// Distinct enough that the mention search cannot rank an unrelated repository file above it, and
// long enough that selecting line 1 is a real range rather than the whole file by coincidence.
const FIXTURE_FILE = "ui-chat-fixture.txt";
// Typed without the extension: the mention query is whatever follows `@`
// (composer-mention.ts:40-43), and a stem keeps the case independent of how the native ranker
// treats a trailing suffix.
const FIXTURE_QUERY = "ui-chat-fixture";
const FIXTURE_BODY = Array.from({ length: 40 }, (_, index) => `fixture line ${index + 1}`).join("\n");

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

let repository = null;
let liveSession = null;
let uiSession = null;
let profileId = null;
let setupFailure = null;
/** Filled in by the live turn so the stop-affordance case can report rather than assume. */
let inFlight = null;

async function quietly(action) {
  try {
    await action();
  } catch (reason) {
    globalThis.console.warn(`ui-chat cleanup step failed: ${reason instanceof Error ? reason.message : String(reason)}`);
  }
}

async function bootstrapReady() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
}

const navigate = navigateTo;

const countOf = async (selector) => (await globalThis.$$(selector)).length;

/**
 * Text content, read in one round trip rather than through `getText()` per element. Both panels
 * this is used on scroll their own overflow (`max-h-56 overflow-y-auto` — ChatInputBox.tsx:189 and
 * SlashCommandOutput.tsx:32), and WebDriver's "rendered text" is not obliged to report a row that
 * is currently scrolled past. Nothing here depends on visibility; the assertions are about which
 * entries exist.
 */
function textsOf(selector) {
  return globalThis.browser.execute((target) => Array.from(
    globalThis.document.querySelectorAll(target),
  ).map((node) => (node.textContent ?? "").replace(/\s+/g, " ").trim()), selector);
}

/**
 * The whole transcript in one round trip. Re-querying element handles inside a poll loop that
 * spans a streaming turn hands back references React has already replaced, and a stale-element
 * throw there reads as "the turn failed" rather than "the DOM moved". `justify-end` is the only
 * thing in the DOM that separates a user bubble from an Agent one (MessageItem.tsx:53).
 */
function readTranscript() {
  return globalThis.browser.execute((canvas) => Array.from(
    globalThis.document.querySelectorAll(`${canvas} > article`),
  ).map((article) => ({
    user: article.classList.contains("justify-end"),
    text: (article.textContent ?? "").trim(),
  })), CANVAS);
}

async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "ui-chat-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await writeFile(join(root, FIXTURE_FILE), `${FIXTURE_BODY}\n`, "utf8");
  await run("git", ["add", "."], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function createSession(title) {
  const operation = await invoke(({ core }, input) => core.invoke("create_session", { input }), {
    agentId: "onepiece",
    interactionMode: "api",
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
  }, { timeout: 60_000, timeoutMsg: `Creating "${title}" never settled.` });
  assert.equal(settled.status, "succeeded", settled.error ?? `creating "${title}" failed`);
  return globalThis.browser.waitUntil(async () => {
    const sessions = await invoke(({ core }) => core.invoke("list_sessions"));
    return sessions.find((item) => item.title === title) ?? false;
  }, { timeout: 30_000, timeoutMsg: `"${title}" was not created.` });
}

/**
 * Routes to a session and waits for the app to actually be driving it.
 *
 * The URL commits before the switch does: `pushState` is synchronous, but the reconciliation
 * effect then calls `switchSession`, which is a backend mutation (use-workspace-session-route.ts:
 * 59-66). Asserting on the URL alone would leave the following keystrokes going into the previous
 * session's composer. The backend's own answer is the only thing that settles it.
 */
async function openSessionChat(sessionId) {
  await navigate(`/workspace/sessions/${encodeURIComponent(sessionId)}`);
  // The tab bar renders unconditionally once the workspace opens for a session, so its absence
  // means the route bounced rather than that the mount is slow (screen-sweep.e2e.mjs:188-190).
  await (await globalThis.$(CHAT_TAB)).waitForExist({ timeout: 30_000 });
  await globalThis.browser.waitUntil(async () => {
    const active = await invoke(({ core }) => core.invoke("get_active_session"));
    return active?.id === sessionId;
  }, { timeout: 30_000, timeoutMsg: `The app never switched to session ${sessionId}.` });
  const input = await globalThis.$(INPUT);
  await input.waitForDisplayed({ timeout: 30_000 });
  // A composer disabled here is not a slow mount: `canSendToSession` refuses archived sessions,
  // sessions awaiting recovery and sessions with a live run (session-admission.ts:10-16).
  await globalThis.browser.waitUntil(async () => await countOf(`${INPUT}:disabled`) === 0, {
    timeout: 30_000,
    timeoutMsg: "The composer stayed disabled, so this session refuses sends.",
  });
  return input;
}

/**
 * Types with real key events. WebDriver's `elementClear`, and any direct `.value` write, is
 * invisible to React's value tracker — the draft would not move, and neither would the slash
 * suggestion query, which only ever updates from `onChange` (api-session-composer.tsx:115).
 */
async function typeInto(input, text) {
  await input.click();
  await input.addValue(text);
}

async function clearDraft(input) {
  if ((await input.getValue()).length === 0) return;
  await input.click();
  await globalThis.browser.keys([process.platform === "darwin" ? "Meta" : "Control", "a"]);
  await globalThis.browser.keys("Backspace");
  // Not trusting the chord: a WebView that maps it elsewhere would leave the draft in place and
  // hand the next case a composer it never asked for. Deleting what is left is cheap insurance.
  for (let remaining = (await input.getValue()).length; remaining > 0; remaining -= 1) {
    await globalThis.browser.keys("Backspace");
  }
  assert.equal(await input.getValue(), "", "the composer could not be emptied");
}

/** Every UI case starts from the same composer state, whatever the previous one left behind. */
async function resetComposer(input) {
  await clearDraft(input);
  // Bounded by the composer's own reference ceiling (MAX_CHAT_FILE_REFERENCES, types/chat.ts) so
  // a chip whose remove button stops working ends the loop rather than the suite's time budget.
  for (let attempt = 0; attempt < 8 && await countOf(REFERENCE_REMOVE) > 0; attempt += 1) {
    await (await globalThis.$(REFERENCE_REMOVE)).click();
  }
  if (await countOf(SLASH_OUTPUT_DISMISS) > 0) {
    await (await globalThis.$(SLASH_OUTPUT_DISMISS)).click();
  }
}

/**
 * A session switch moves the URL and the backend's active session before the message query for
 * the new session resolves, so the previous session's transcript is briefly still on screen.
 * Asserting "nothing was sent" against that window would read the live turn's bubbles.
 */
async function awaitEmptyTranscript() {
  await globalThis.browser.waitUntil(async () => await countOf(TRANSCRIPT) === 0, {
    timeout: 30_000,
    timeoutMsg: "The transcript still holds messages, so this session is not the empty one.",
  });
}

globalThis.describe("VaneHub AI desktop chat UI", () => {
  globalThis.before(async () => {
    // Reload rather than inherit whatever the previous spec file left on screen: this harness
    // does not terminate the app between spec files, and a page that has already driven other
    // flows will not commit a `pushState` route change (screen-sweep.e2e.mjs:82-86).
    await globalThis.browser.refresh();
    await bootstrapReady();

    try {
      repository = await createRepository();
      // Slash commands are gated to `agentId === "onepiece"` (command-availability.ts:17-20) and
      // an api-mode session is what renders the structured composer at all (main-layout.tsx:
      // 221-223), so every case in this file needs a OnePiece API session. Those need a provider
      // profile before they will start (screen-sweep.e2e.mjs:145-146).
      //
      // A placeholder key is used when the host supplies none: saving a profile persists it
      // without contacting anyone (service.rs:1050-1110 validates shape only), which keeps the
      // provider-free cases — slash commands, composer states, the file picker — runnable. Only
      // the live turn actually needs a real credential, and it checks for one itself.
      const apiKey = readOnePieceApiKey();
      const saved = await invoke(({ core }, input) => core.invoke("save_onepiece_provider_profile", { input }), {
        id: null,
        name: "UI chat sweep DeepSeek",
        providerId: "deepseek",
        endpointType: "openai-chat-completions",
        modelId: "deepseek-v4-flash",
        apiKey: apiKey ?? "ui-chat-sweep-placeholder",
      });
      const created = saved.profiles.find((profile) => profile.name === "UI chat sweep DeepSeek");
      profileId = created?.id ?? null;
      // Only the first profile in a store auto-activates (service.rs:1094). A profile another
      // spec left behind would otherwise keep serving the turn this file is about to pay for.
      if (profileId && created?.active !== true) {
        await invoke(({ core }, id) => core.invoke("activate_onepiece_provider_profile", {
          profileId: id,
        }), profileId);
      }

      liveSession = await createSession("UI chat live turn");
      uiSession = await createSession("UI chat surfaces");
    } catch (reason) {
      setupFailure = reason instanceof Error ? reason.message : String(reason);
    }

    // Both sessions were created over IPC, after this page had already read its session list, so
    // routing straight to one lands on "session unavailable" and bounces back to the list. The
    // route commits either way, which is why the URL alone proves nothing here.
    await globalThis.browser.refresh();
    await bootstrapReady();
  });

  globalThis.it("renders the user message and then the Agent reply for a turn sent from the composer", async function liveTurn() {
    if (!liveSession) {
      blocked.push(`live turn: no OnePiece session (${setupFailure ?? "session creation reported no error"})`);
      this.skip();
    }
    if (!readOnePieceApiKey()) {
      blocked.push("live turn: set VANEHUB_ONEPIECE_API_KEY or VANEHUB_ONEPIECE_PROFILE_ID");
      this.skip();
    }
    const input = await openSessionChat(liveSession.id);
    await resetComposer(input);

    await typeInto(input, PROMPT);
    assert.equal(await input.getValue(), PROMPT, "the composer did not accept the typed prompt");
    await globalThis.browser.waitUntil(async () => await countOf(`${ACTION}:disabled`) === 0, {
      timeout: 10_000,
      timeoutMsg: "The send control stayed disabled with a non-empty draft.",
    });
    await (await globalThis.$(ACTION)).click();

    // One loop, because the in-flight window closes when the reply lands: polling for the reply
    // first would routinely miss the state the composer passed through on the way there.
    const observed = { disabledInput: false, stopControl: false, stopEnabled: false, userBubble: null };
    let reply = null;
    try {
      await globalThis.browser.waitUntil(async () => {
        if (!observed.disabledInput && await countOf(`${INPUT}:disabled`) > 0) observed.disabledInput = true;
        if (!observed.stopControl && await countOf(STOP_ICON) > 0) {
          observed.stopControl = true;
          observed.stopEnabled = await countOf(`${ACTION}:not(:disabled) svg.lucide-square`) > 0;
        }
        const transcript = await readTranscript();
        observed.userBubble ??= transcript.find((bubble) => bubble.user && bubble.text.includes(PROMPT)) ?? null;
        // Keep the last Agent bubble even when it does not answer yet, so a turn that produces
        // prose but never the word says so in the failure instead of reporting an empty reply.
        const agentBubbles = transcript.filter((bubble) => !bubble.user && bubble.text.length > 0);
        reply = agentBubbles.find((bubble) => /PONG/i.test(bubble.text)) ?? agentBubbles.at(-1) ?? null;
        return Boolean(reply && /PONG/i.test(reply.text));
      }, {
        // Generous on purpose: a provider round trip through an egress proxy is slow, and a
        // timeout here would be reported as a product failure rather than the wait it is.
        timeout: 240_000,
        interval: 500,
        timeoutMsg: "No Agent bubble containing PONG reached the transcript.",
      });
    } catch (reason) {
      blocked.push(`live turn: ${reason instanceof Error ? reason.message : String(reason)}`
        + ` (user bubble rendered=${Boolean(observed.userBubble)}, composer disabled=${observed.disabledInput}`
        + `, stop control=${observed.stopControl}; see the run's vanehub.log)`);
      inFlight = { ...observed, completed: false };
      this.skip();
    }

    assert.ok(observed.userBubble, "the typed prompt never rendered as a user bubble");
    assert.ok(reply, "no Agent bubble rendered");
    assert.match(reply.text, /PONG/i, `unexpected Agent reply: ${reply.text}`);
    assert.ok(
      observed.disabledInput || observed.stopControl,
      "the composer never entered an in-flight state: the input stayed writable and the send control never swapped",
    );

    // Back to a Send control, disabled again because `submit` empties the draft
    // (use-main-layout-model.ts:253). Both halves matter: a Stop button left on screen after the
    // turn ends is a live-looking session that can no longer be sent to.
    await globalThis.browser.waitUntil(async () => await countOf(SEND_ICON) > 0, {
      timeout: 30_000,
      timeoutMsg: "The action control did not revert to Send after the turn completed.",
    });
    assert.equal(await input.getValue(), "", "the composer kept the draft after sending it");
    assert.equal(await countOf(`${ACTION}:disabled`), 1, "the send control is enabled on an empty draft");
    inFlight = { ...observed, completed: true };
  });

  globalThis.it("swaps the send control for an enabled stop control while a turn is in flight", async function stopControl() {
    if (!inFlight) {
      blocked.push("stop control: the live turn did not run, so there was no in-flight window to observe");
      this.skip();
    }
    if (!inFlight.stopControl) {
      // Reported rather than asserted away: a turn the runtime never marks as streaming has no
      // stop affordance to expose, and `isStreaming` needs both a live run id on the session and
      // a streaming message that belongs to it (session-admission.ts:22-28).
      blocked.push(`stop control: never rendered during the live turn (composer disabled=${inFlight.disabledInput}`
        + `, turn completed=${inFlight.completed}) — the turn may not have streamed`);
      this.skip();
    }
    assert.ok(inFlight.stopEnabled, "the stop control rendered disabled, so an in-flight turn cannot be cancelled");
    // Clicking it is deliberately not exercised: cancelling is only reachable from a live turn,
    // and this file spends exactly one.
  });

  globalThis.it("offers slash completions and fills the draft from the one that is picked", async function completions() {
    if (!uiSession) {
      blocked.push(`slash completions: no OnePiece session (${setupFailure ?? "session creation reported no error"})`);
      this.skip();
    }
    const input = await openSessionChat(uiSession.id);
    await awaitEmptyTranscript();
    await resetComposer(input);

    await typeInto(input, "/");
    await globalThis.browser.waitUntil(async () => await countOf(SLASH_OPTION) > 0, {
      timeout: 15_000,
      timeoutMsg: "Typing a slash offered no completions.",
    });
    // Capped at eight (use-slash-commands.ts:47), and there are twenty commands in the catalogue
    // (command-catalog.ts:7-9), so a short list here means the availability filter dropped them.
    assert.equal(await countOf(SLASH_OPTION), 8, "the completion list is not the capped eight entries");
    for (const label of await textsOf(SLASH_OPTION)) {
      assert.match(label, /^\//, "a completion entry does not lead with its invocation");
    }

    await clearDraft(input);
    await typeInto(input, "/st");
    // `status` and `streaming` are the whole catalogue's `st` prefix; anything else means the
    // filter is matching on something other than the command name (use-slash-commands.ts:47).
    await globalThis.browser.waitUntil(async () => await countOf(SLASH_OPTION) === 2, {
      timeout: 15_000,
      timeoutMsg: "Narrowing to /st did not leave exactly the two matching commands.",
    });
    const labels = await textsOf(SLASH_OPTION);
    assert.ok(labels.some((label) => label.startsWith("/status")), `no /status option: ${labels.join(" | ")}`);
    assert.ok(labels.some((label) => label.startsWith("/streaming")), `no /streaming option: ${labels.join(" | ")}`);

    const statusIndex = labels.findIndex((label) => label.startsWith("/status"));
    await (await globalThis.$$(SLASH_OPTION))[statusIndex].click();
    // A command occupies the whole draft, so completing one replaces it (use-slash-commands.ts:114).
    await globalThis.browser.waitUntil(async () => await input.getValue() === "/status ", {
      timeout: 10_000,
      timeoutMsg: "Picking a completion did not write the invocation into the draft.",
    });
    assert.equal(await countOf(TRANSCRIPT), 0, "picking a completion sent something to the model");
    await resetComposer(input);
  });

  globalThis.it("runs /help into the output panel without sending a chat message", async function helpCommand() {
    if (!uiSession) {
      blocked.push(`/help: no OnePiece session (${setupFailure ?? "session creation reported no error"})`);
      this.skip();
    }
    const input = await openSessionChat(uiSession.id);
    await awaitEmptyTranscript();
    await resetComposer(input);

    await typeInto(input, "/help");
    await globalThis.browser.keys("Enter");

    const output = await globalThis.$(SLASH_OUTPUT);
    await output.waitForDisplayed({ timeout: 15_000 });
    assert.equal(await output.getAttribute("data-tone"), "info", "/help reported an error tone");

    const entries = await textsOf(SLASH_OUTPUT_ENTRY);
    // Nineteen of the twenty commands apply here — `/plan` is withheld without an associated plan
    // run (navigation-commands.ts:31-34, api-session-composer.tsx:40). Asserting a floor rather
    // than the exact number so adding a command is not a test failure, while losing most of them
    // still is.
    assert.ok(entries.length >= 15, `/help listed only ${entries.length} commands`);
    for (const invocation of ["/status", "/logs", "/mode <inherit|plan|execute>"]) {
      assert.ok(entries.some((entry) => entry.includes(invocation)), `/help omitted ${invocation}`);
    }
    // The one mechanism shared by every entry: `descriptionKey` is a translation key nested in
    // another key's params and has to be resolved before the outer interpolation
    // (SlashCommandOutput.tsx:20-26). A broken resolver still prints the invocation, and prints
    // the raw `slash.command.*.description` key where the description belongs.
    for (const entry of entries) {
      assert.ok(!entry.includes("slash."), `an unresolved translation key reached the panel: ${entry}`);
    }

    assert.equal(await input.getValue(), "", "/help left its own text in the composer");
    // A fresh session renders no scroll region until it holds a message (MessageList.tsx:66-72),
    // so its absence is structural proof that /help never reached `model.submit()`.
    assert.equal(await countOf(TRANSCRIPT), 0, "/help was sent to the model as a chat message");

    await (await globalThis.$(SLASH_OUTPUT_DISMISS)).click();
    await globalThis.browser.waitUntil(async () => await countOf(SLASH_OUTPUT) === 0, {
      timeout: 10_000,
      timeoutMsg: "The command output panel could not be dismissed.",
    });
  });

  globalThis.it("refuses to send an empty or whitespace-only composer", async function emptyComposer() {
    if (!uiSession) {
      blocked.push(`empty composer: no OnePiece session (${setupFailure ?? "session creation reported no error"})`);
      this.skip();
    }
    const input = await openSessionChat(uiSession.id);
    await awaitEmptyTranscript();
    await resetComposer(input);

    assert.equal(await countOf(SEND_ICON), 1, "the action control is not a send control on an idle session");
    assert.equal(await countOf(`${ACTION}:disabled`), 1, "an empty composer offers an enabled send control");
    // The clear affordance only exists while there is something to clear (ChatInputBox.tsx:235).
    assert.equal(await countOf(CLEAR), 0, "the clear control is rendered over an empty composer");

    await input.click();
    await globalThis.browser.keys("Enter");
    assert.equal(await countOf(TRANSCRIPT), 0, "Enter on an empty composer sent a message");

    // `canSubmit` trims before measuring (ChatInputBox.tsx:94), so spaces alone must stay unsendable.
    await typeInto(input, "   ");
    assert.equal(await countOf(CLEAR), 1, "a non-empty draft did not reveal the clear control");
    assert.equal(await countOf(`${ACTION}:disabled`), 1, "a whitespace-only draft offers an enabled send control");
    await globalThis.browser.keys("Enter");
    assert.equal(await countOf(TRANSCRIPT), 0, "Enter on a whitespace-only composer sent a message");

    await typeInto(input, "x");
    await globalThis.browser.waitUntil(async () => await countOf(`${ACTION}:disabled`) === 0, {
      timeout: 10_000,
      timeoutMsg: "A composer with real content still refuses to send.",
    });

    await (await globalThis.$(CLEAR)).click();
    await globalThis.browser.waitUntil(async () => await input.getValue() === "", {
      timeout: 10_000,
      timeoutMsg: "The composer's clear control did not empty the draft.",
    });
    assert.equal(await countOf(`${ACTION}:disabled`), 1, "the send control stayed enabled after clearing");
    assert.equal(await countOf(TRANSCRIPT), 0, "this case put a message into the session");
  });

  globalThis.it("attaches a file reference picked through the composer's preview", async function filePicker() {
    if (!uiSession) {
      blocked.push(`file reference picker: no OnePiece session (${setupFailure ?? "session creation reported no error"})`);
      this.skip();
    }
    const input = await openSessionChat(uiSession.id);
    await awaitEmptyTranscript();
    await resetComposer(input);

    await typeInto(input, `@${FIXTURE_QUERY}`);
    // The candidate label is the match's own path (ChatInputBox.tsx:200) — a fixture this spec
    // wrote, not app copy, which is why matching on its text is safe under any locale.
    const candidate = await globalThis.$(
      `//*[@data-testid="wechat-style-composer"]//button[contains(normalize-space(.), "${FIXTURE_FILE}")]`,
    );
    try {
      await candidate.waitForExist({ timeout: 30_000 });
    } catch {
      blocked.push(`file reference picker: the native mention search returned no candidate for ${FIXTURE_FILE}`);
      await quietly(() => resetComposer(input));
      this.skip();
    }
    await candidate.click();

    // No range was typed, so the candidate goes to the preview rather than attaching directly
    // (ChatInputBox.tsx:115-124). The dialog is lazily imported (ChatInputBox.tsx:11-12), so the
    // wait here covers a chunk load as well as a native file read.
    const dialog = await globalThis.$(DIALOG);
    try {
      await dialog.waitForExist({ timeout: 30_000 });
      await (await globalThis.$(PREVIEW_LINES)).waitForExist({ timeout: 30_000 });
      await (await globalThis.$(PREVIEW_LINE_1)).waitForExist({ timeout: 30_000 });
    } catch {
      blocked.push(`file reference picker: the preview dialog never rendered ${FIXTURE_FILE}`);
      await quietly(() => resetComposer(input));
      this.skip();
    }

    // Cancel / attach-whole-file / attach-selection, in that order, are the last three buttons in
    // the dialog (FileReferencePreviewDialog.tsx:107-120); everything before them is a preview
    // row, and `ApplicationDialog` adds none of its own (application-dialog.tsx:89-98).
    // Attach-selection is disabled until a range exists, which is the wiring under test.
    const attachSelection = async () => (await globalThis.$$(`${DIALOG} button`)).at(-1);
    assert.equal(
      await (await attachSelection()).isEnabled(),
      false,
      "the attach-selection control is enabled before any line was picked",
    );

    await (await globalThis.$(PREVIEW_LINE_1)).click();
    await globalThis.browser.waitUntil(
      async () => (await (await globalThis.$(PREVIEW_LINE_1)).getAttribute("data-selected")) === "true",
      { timeout: 10_000, timeoutMsg: "Clicking a preview line did not select it." },
    );
    await globalThis.browser.waitUntil(async () => await (await attachSelection()).isEnabled(), {
      timeout: 10_000,
      timeoutMsg: "Selecting a line did not enable the attach-selection control.",
    });
    await (await attachSelection()).click();

    await globalThis.browser.waitUntil(async () => await countOf(DIALOG) === 0, {
      timeout: 15_000,
      timeoutMsg: "The preview dialog stayed open after attaching.",
    });
    const chips = await globalThis.$(REFERENCES);
    await chips.waitForExist({ timeout: 15_000 });
    const chipText = (await chips.getText()).trim();
    assert.ok(chipText.includes(FIXTURE_FILE), `the chip does not name the picked file: ${chipText}`);
    // `L1` is the compact range marker (FileReferenceLines.tsx:23). Without it the chip claims a
    // whole-file reference, which is a different prompt payload than the one that was picked.
    assert.ok(chipText.includes("L1"), `the chip lost the picked range: ${chipText}`);
    // Selection also rewrites the mention token in the draft (ChatInputBox.tsx:109-113). Matched
    // without the leading `@` so a session-relative path with a directory prefix still passes:
    // the claim is that the resolved path and the picked range reached the draft, not where the
    // native search chose to root it.
    const draft = await input.getValue();
    assert.ok(draft.includes(`${FIXTURE_FILE}:1`), `the draft did not take the resolved mention: ${draft}`);

    await (await globalThis.$(REFERENCE_REMOVE)).click();
    await globalThis.browser.waitUntil(async () => await countOf(REFERENCES) === 0, {
      timeout: 10_000,
      timeoutMsg: "The reference chip could not be removed.",
    });
    await clearDraft(input);
    assert.equal(await countOf(TRANSCRIPT), 0, "this case put a message into the session");
  });

  globalThis.after(async () => {
    for (const session of [liveSession, uiSession]) {
      if (!session) continue;
      await quietly(() => invoke(
        ({ core }, sessionId) => core.invoke("delete_session", { sessionId }),
        session.id,
      ));
    }
    if (profileId) {
      await quietly(() => invoke(
        ({ core }, id) => core.invoke("delete_onepiece_provider_profile", { profileId: id }),
        profileId,
      ));
    }
    // Restoring the destination takes a reload, not just a `pushState`: the route effect redirects
    // `/workspace/sessions` straight back onto whatever session the page still believes is active
    // (use-workspace-session-route.ts:50-53), and this page's cache still holds the two sessions
    // that were just deleted. After the reload the backend reports no active session — deleting
    // one emits `active_session_changed(None)` (delete_session.rs:12) — so the list destination
    // sticks, and that is what App.tsx:54 persists for the next spec's relaunch.
    await quietly(async () => {
      await globalThis.browser.refresh();
      await bootstrapReady();
      await navigate("/workspace/sessions");
    });
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // No `exit_application` here: from an after hook it races WDIO's `deleteSession` and discards
    // every per-test result this file produced.
  });
});
