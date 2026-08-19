import assert from "node:assert/strict";

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
  const options = await element.$$("option");
  const values = await Promise.all(options.map((option) => option.getAttribute("value")));
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
