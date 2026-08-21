import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

async function attempt(command) {
  return invoke(({ core }, name) => core.invoke(name).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error: String(error) }),
  ), command);
}

/**
 * A sweep over the read commands that take no caller argument and had no coverage.
 *
 * These are cheap to call and, individually, not very interesting -- which is exactly why none of
 * them had a test. What the sweep buys is the failure mode they share. A command can be perfectly
 * implemented and still be unreachable: register it with `generate_handler!`, forget the routing
 * list beside it, and every call comes back "unknown command" while the code that would have
 * answered sits there untouched. That took out the entire Goals domain on the desktop (D-01), and
 * it was invisible until something actually called one of them across the IPC boundary.
 *
 * `supplemental_registry.rs`'s own unit test now compares those two lists at the source level,
 * which catches the drift earlier and on every platform. This covers what that cannot: a command
 * that is routed correctly and then fails anyway -- a panic on empty state, a DTO that will not
 * serialize, missing managed state. Each entry is asserted to answer with the shape its page
 * expects, so "it did not error" is never the whole assertion.
 */
const EXPECT_ARRAY = [
  "list_archived_sessions",
  "list_folder_openers",
  "list_hybrid_routing_rules",
  "list_known_projects",
  "list_known_remote_workspaces",
  "list_onepiece_provider_presets",
  "list_operations",
  "list_sdk_definitions",
  "list_session_categories",
];

// A `list_` prefix does not promise a bare array. These two answer with a wrapper
// (`OnePieceProviderProfiles`) and a map (`SdkStatusMap`) respectively, which is what their pages
// consume; grouping them by prefix rather than by return type was this file's own mistake on its
// first run, not a defect in either command.
const EXPECT_OBJECT = [
  "get_automatic_archival_settings",
  "get_data_management_info",
  "get_folder_opener_preferences",
  "get_node_info",
  "get_onepiece_provider_config",
  "get_session_details",
  "get_workflow_state",
  "list_onepiece_provider_profiles",
  "list_sdk_statuses",
];

globalThis.describe("VaneHub AI desktop argument-free read surface", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("answers every argument-free list command with an array", async () => {
    const failures = [];
    for (const command of EXPECT_ARRAY) {
      const result = await attempt(command);
      if (!result.ok) {
        failures.push(`${command}: ${result.error}`);
        continue;
      }
      if (!Array.isArray(result.value)) {
        failures.push(`${command}: answered ${typeof result.value}, not an array`);
      }
    }
    // Reported together rather than one assertion each: a registry that lost a whole domain takes
    // several of these down at once, and seeing all of them beats fixing them one run at a time.
    assert.deepEqual(failures, [], `argument-free list commands that did not answer:\n  ${failures.join("\n  ")}`);
  });

  globalThis.it("answers every argument-free get command with an object", async () => {
    const failures = [];
    for (const command of EXPECT_OBJECT) {
      const result = await attempt(command);
      if (!result.ok) {
        failures.push(`${command}: ${result.error}`);
        continue;
      }
      if (result.value === null || typeof result.value !== "object" || Array.isArray(result.value)) {
        failures.push(`${command}: answered ${JSON.stringify(result.value)}, not an object`);
      }
    }
    assert.deepEqual(failures, [], `argument-free get commands that did not answer:\n  ${failures.join("\n  ")}`);
  });

  globalThis.it("does not report a routed command as unknown", async () => {
    // The D-01 signature, asserted directly. Every command above is registered, so none of them
    // may come back as unrecognised -- an error is acceptable here (a command may legitimately
    // fail on this host), but "not found" is not, because that means nothing is wired to it.
    const unknown = [];
    for (const command of [...EXPECT_ARRAY, ...EXPECT_OBJECT]) {
      const result = await attempt(command);
      if (result.ok) continue;
      if (/not found|unknown command|Command \w+ not found/i.test(result.error)) {
        unknown.push(`${command}: ${result.error}`);
      }
    }
    assert.deepEqual(unknown, [], `registered commands reported as unknown:\n  ${unknown.join("\n  ")}`);
  });

  globalThis.it("reports the node runtime and data directories the About and Basic pages render", async () => {
    // Two of the sweep's entries carry values a page puts on screen verbatim, so their shape is
    // worth more than "an object came back".
    const node = await invoke(({ core }) => core.invoke("get_node_info"));
    assert.ok("available" in node, "get_node_info did not report availability");

    const data = await invoke(({ core }) => core.invoke("get_data_management_info"));
    const directories = Object.entries(data).filter(([, value]) => typeof value === "string" && value.length > 0);
    assert.ok(
      directories.length > 0,
      `get_data_management_info reported no directory at all: ${JSON.stringify(data)}`,
    );
  });
});
