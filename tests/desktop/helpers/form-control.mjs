import assert from "node:assert/strict";
import { Key } from "webdriverio";

/**
 * Types `value` into a controlled input, clearing it with the keyboard first.
 *
 * Not `setValue`: that issues WebDriver's Element Clear, which assigns `value` directly. React's
 * controlled-input value tracker can swallow the event that follows, leaving component state on
 * the old text while the box on screen looks empty -- the submit then sends the stale value and
 * the test "passes" against a field it never actually changed.
 *
 * Not one Backspace per character either. A counted run is off by one whenever a keystroke lands
 * before the caret settles from the click, and the residue rides into the typed value: clearing a
 * 12-character field with 12 Backspaces produced "Ivanehub-ui-settings-e2e", which reads as the
 * app mangling input rather than as the test miscounting. Ctrl+A does not depend on the caret
 * position or on how many keys arrive.
 */
export async function fill(description, element, value) {
  await element.waitForClickable({ timeout: 15_000 });
  await element.click();
  const current = String((await element.getProperty("value")) ?? "");
  if (current.length > 0) {
    await globalThis.browser.keys([Key.Ctrl, "a"]);
    await globalThis.browser.keys(["Backspace"]);
  }
  if (value.length > 0) await element.addValue(value);
  await globalThis.browser.waitUntil(
    async () => (await element.getProperty("value")) === value,
    { timeout: 15_000, timeoutMsg: `${description} did not accept the typed value.` },
  );
}


/**
 * Moves a native `<select>` to `value` and waits for the app to hold it.
 *
 * A native select cannot be driven through the driver in this runtime, and two ways of trying were
 * spent finding that out. `selectByAttribute` finds the matching `<option>` and clicks it, but
 * WebView2 draws a closed select's options through the OS rather than painting them into the page,
 * so the click reports success and the value never moves -- `elementClick` returning null with the
 * select still reading `14px` for the whole 30s poll after it. Focusing the select and sending
 * ArrowUp/ArrowDown did not move it either, though the same `browser.keys` drives the chat
 * composer in the spec next door.
 *
 * So the change is dispatched in the page instead: assign through the prototype's value setter,
 * which is what bypasses React's controlled-input tracker, then fire a bubbling `change`. Be clear
 * about what that buys. It exercises the app end to end from the change handler on -- the handler,
 * the command, the persisted value, the re-render -- which is the wiring these specs exist to
 * prove. It does not exercise the browser's own select widget. A regression that broke only the
 * widget's OS-level interaction would pass here, and nothing in this suite would catch it.
 */
export async function selectOption(description, element, value) {
  // Read the option values in the page rather than as WDIO element handles: `$$` resolves to a
  // chainable array-like whose `map` is not the array method, so mapping it and awaiting the
  // result throws "object is not iterable" from inside this helper -- an error about the helper,
  // reported against the widget.
  const values = await globalThis.browser.execute(
    (select) => Array.from(select.options).map((option) => option.value),
    element,
  );
  assert.ok(
    values.includes(value),
    `${description} has no ${value} option; it offers ${values.join(", ")}`,
  );

  // A page that saves on change disables its controls while the save is in flight, and a change
  // dispatched then is dropped by the handler it reaches.
  await element.waitForEnabled({ timeout: 30_000 });
  await globalThis.browser.execute((select, next) => {
    const setter = Object.getOwnPropertyDescriptor(
      globalThis.HTMLSelectElement.prototype,
      "value",
    )?.set;
    setter?.call(select, next);
    select.dispatchEvent(new globalThis.Event("change", { bubbles: true }));
  }, element, value);

  // Settles either by holding the new value or by going away. A control whose change relocates its
  // own container -- the work board picker moves its card to another column -- is unmounted and a
  // fresh one renders elsewhere, so this handle is left pointing at a detached element that reads
  // the old value forever. Waiting only for the value would report "the picker never settled",
  // about a picker that did exactly what it was supposed to. Every caller asserts the outcome it
  // actually cares about right after this returns; this wait only rules out a no-op.
  // `isConnected` rather than a stale-element error, because this driver answers `getProperty` on a
  // detached node instead of raising, so a caught exception never arrives and the old value polls
  // forever. The work board's picker moves its own card to another column and is unmounted by its
  // own success; waiting only for the value reported "the picker never settled" over a screenshot
  // of the card sitting in the column it was asked to move to.
  await globalThis.browser.waitUntil(async () => {
    try {
      return await globalThis.browser.execute(
        (select, want) => !select.isConnected || select.value === want,
        element,
        value,
      );
    } catch {
      return true;
    }
  }, { timeout: 30_000, timeoutMsg: `${description} never settled on ${value}.` });
}
