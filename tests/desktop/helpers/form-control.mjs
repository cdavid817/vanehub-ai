import assert from "node:assert/strict";

/**
 * Types `value` into a controlled input, clearing it first.
 *
 * Not `setValue` for the final value: that issues WebDriver's Element Clear, which assigns
 * `value` directly. React's controlled-input value tracker can swallow the event that follows,
 * leaving component state on the old text while the box on screen looks empty -- the submit then
 * sends the stale value and the test "passes" against a field it never actually changed.
 *
 * Not keyboard-driven clearing either, not anymore. One Backspace per character is off by one
 * whenever a keystroke lands before the caret settles from the click, and the residue rides into
 * the typed value: clearing a 12-character field with 12 Backspaces produced
 * "Ivanehub-ui-settings-e2e". Ctrl+A was meant to fix that by not depending on caret position or
 * keystroke count, but on WebKitGTK it silently failed to select anything often enough that two
 * `fill()` calls in a row produced a search box reading "primary-tokennomatch-token" -- the
 * Backspace after it deleted nothing, and `addValue` appended after the leftover text instead of
 * replacing it. A bounded End+Backspace fallback was tried next and failed the same way on a
 * second field (a derived session title read back as "derived-nameuiflow-primary-x": the retry
 * loop's own `browser.keys()` calls did not reach the field either). Every failure in this history
 * has one thing in common -- synthetic key events dispatched through the driver, not delivered by
 * a real keyboard. So clearing now bypasses it entirely, the same way `selectOption` below already
 * has to for the same underlying reason: assign through the native value setter in the page and
 * dispatch a bubbling `input` (what React's controlled-input listener actually reads), which lands
 * synchronously and needs no keyboard focus at all.
 */
export async function fill(description, element, value) {
  await element.waitForClickable({ timeout: 15_000 });
  await element.click();
  await globalThis.browser.execute((input, next) => {
    const proto = input instanceof globalThis.HTMLTextAreaElement
      ? globalThis.HTMLTextAreaElement.prototype
      : globalThis.HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(proto, "value")?.set;
    setter?.call(input, next);
    input.dispatchEvent(new globalThis.Event("input", { bubbles: true }));
  }, element, value);
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
