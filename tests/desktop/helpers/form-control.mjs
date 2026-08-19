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
 * Moves a native `<select>` to `value` with the keyboard and waits for the app to hold it.
 *
 * Not `selectByAttribute`. That command finds the matching `<option>` and clicks it, and in the
 * WebView2 runtime the options of a *closed* select are drawn by the OS rather than painted into
 * the page, so the synthesized click lands on nothing. It is the worst kind of failure to read: the
 * driver reports the click succeeded, the value never moves, and the test times out pointing at the
 * widget as though the product were broken. The run that found this shows `elementClick` returning
 * null at 19:18:01.937 and the select still reading `14px` for the full 30s poll that followed.
 *
 * Arrow keys on a focused select are handled inside the engine and commit immediately, which is
 * both reliable here and what a keyboard user actually does. Each step is committed on its own
 * because a page that saves on change disables its controls while the save is in flight, and a key
 * that arrives during one is dropped.
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
  const target = values.indexOf(value);
  assert.ok(target >= 0, `${description} has no ${value} option; it offers ${values.join(", ")}`);

  // Bounded by the option count: one step per option is enough to cross the list from either end,
  // and a select that stops responding then fails on the assertion below rather than spinning.
  for (let step = 0; step < values.length; step += 1) {
    const current = values.indexOf(String(await element.getProperty("value")));
    if (current === target) break;
    await element.waitForEnabled({ timeout: 30_000 });
    await globalThis.browser.execute((select) => select.focus(), element);
    await globalThis.browser.keys([current < target ? "ArrowDown" : "ArrowUp"]);
  }

  await globalThis.browser.waitUntil(
    async () => (await element.getProperty("value")) === value,
    { timeout: 30_000, timeoutMsg: `${description} never settled on ${value}.` },
  );
}
