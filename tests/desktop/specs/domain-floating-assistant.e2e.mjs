import assert from "node:assert/strict";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

/**
 * `wdio-tauri-service`'s IPC bridge only ever rejects with a bare string
 * (node_modules/@wdio/tauri-service/dist/esm/index.js:3141), so a structured `CommandError`
 * reaches the spec with its `code`/`field` lost -- catching in the page keeps the message intact,
 * which is all the assertions below need.
 */
async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

globalThis.describe("VaneHub AI desktop floating assistant domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("reports this host's floating-assistant platform support", async () => {
    // floating_assistant.rs:6-10,17-21 -- the domain only knows two platforms, and every runner
    // this suite has ever seen but Windows falls into the same `Unsupported` bucket rather than a
    // per-OS one.
    const info = await invoke(({ core }) => core.invoke("get_floating_assistant_runtime_info"));
    if (process.platform === "win32") {
      assert.equal(info.platform, "windows");
      assert.equal(info.nativeAvailable, true);
    } else {
      assert.equal(info.platform, "unsupported");
      assert.equal(info.nativeAvailable, false);
    }
  });

  globalThis.it("refuses to enable the assistant on a platform without native support", async function refuseEnable() {
    const info = await invoke(({ core }) => core.invoke("get_floating_assistant_runtime_info"));
    if (info.nativeAvailable) {
      blocked.push("enable refusal: this host reports native support, so enabling is not expected to fail");
      this.skip();
    }
    // floating_assistant.rs:24-32 -- `validate_enablement` only rejects turning it *on*; staying
    // disabled is always allowed regardless of platform, which the next assertion checks.
    const refused = await attempt("set_floating_assistant_enabled", { enabled: true });
    assert.equal(refused.ok, false, "enabling the floating assistant on an unsupported host did not fail");
    assert.match(
      String(refused.error),
      /floating assistant is currently available on Windows only/,
    );

    const stillDisabled = await attempt("set_floating_assistant_enabled", { enabled: false });
    assert.equal(stillDisabled.ok, true, "disabling (a no-op here) must succeed on every platform");
    assert.equal(stillDisabled.value.enabled, false);
  });

  globalThis.it("round-trips a saved anchor through get_floating_assistant_config", async () => {
    // floating_assistant.rs:100-142 -- the anchor is the assistant's own bottom-right corner, not
    // a top-left origin, so the two coordinates below are unrelated to any monitor's real size;
    // `save_floating_assistant_anchor` only validates finiteness and range (line 144-146).
    const anchor = { x: 1234.5, y: 678.25, monitorName: "DISPLAY-TEST" };
    const saved = await invoke(
      ({ core }, input) => core.invoke("save_floating_assistant_anchor", { anchor: input }),
      anchor,
    );
    assert.equal(saved.anchor?.x, anchor.x);
    assert.equal(saved.anchor?.y, anchor.y);
    assert.equal(saved.anchor?.monitorName, anchor.monitorName);

    const read = await invoke(({ core }) => core.invoke("get_floating_assistant_config"));
    assert.equal(read.anchor?.x, anchor.x);
    assert.equal(read.anchor?.y, anchor.y);
    assert.equal(read.anchor?.monitorName, anchor.monitorName);
  });

  globalThis.it("normalizes a non-finite or out-of-range anchor to no anchor, rather than rejecting it", async () => {
    // floating_assistant.rs:3,108-109,144-146 -- `MAX_COORDINATE` is 10_000_000; the domain
    // constructor returns `None` for anything past it or non-finite, and `save_anchor`
    // (application/floating_assistant/service.rs:57-65) passes that `None` straight through to
    // `with_anchor` rather than rejecting the call. This is documented, tested product behavior
    // (application/floating_assistant/tests.rs's
    // `anchor_updates_normalize_invalid_coordinates_and_keep_enablement`), not a bug -- a caller
    // that hands over garbage coordinates gets "no remembered anchor" back, not an error, and this
    // is the one place in the file that asserts it end to end through the command rather than the
    // service directly.
    const result = await invoke(
      ({ core }, input) => core.invoke("save_floating_assistant_anchor", { anchor: input }),
      { x: Number.MAX_SAFE_INTEGER, y: 0, monitorName: null },
    );
    assert.equal(result.anchor, null, "an out-of-range anchor was stored instead of normalized away");

    const after = await invoke(({ core }) => core.invoke("get_floating_assistant_config"));
    assert.equal(after.anchor, null);
  });

  globalThis.it("rejects an unrecognized main-window action", async () => {
    // floating_assistant.rs:49-56 -- the parser only accepts the three documented actions; this
    // command has no return value on success, so a rejection is the only observable outcome here.
    const refused = await attempt("show_main_window", { action: "not-a-real-action" });
    assert.equal(refused.ok, false, "an invalid main-window action was accepted");
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
