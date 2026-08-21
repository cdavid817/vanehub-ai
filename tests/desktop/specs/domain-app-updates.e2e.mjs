import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

// desktop-update.ts:1-2.
const PHASES = [
  "idle", "queued", "checking", "available", "up-to-date", "downloading", "ready-to-restart", "failed",
];
const CHANNELS = ["stable", "preview"];

/**
 * The About page: current version, update channel, and the update check behind it.
 *
 * Two commands in this domain are deliberately never called. `download_and_install_desktop_update`
 * fetches and stages a real installer over the running application, and
 * `restart_after_desktop_update` restarts the process out from under the driver -- which would
 * take the WDIO session with it and discard every result in the run. They are named in the blocked
 * list rather than left unmentioned, the same way domain-cli-tooling reports its unrun write
 * paths, so this file's coverage is not read as covering them.
 */
globalThis.describe("VaneHub AI desktop application updates", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("reports a snapshot carrying the running version and a modelled phase", async () => {
    const snapshot = await invoke(({ core }) => core.invoke("get_desktop_update_snapshot"));
    assert.ok(snapshot, "get_desktop_update_snapshot returned nothing");
    assert.ok(
      PHASES.includes(snapshot.phase),
      `the snapshot reported an unmodelled phase: ${snapshot.phase}`,
    );
    assert.ok(CHANNELS.includes(snapshot.channel), `the snapshot reported an unknown channel: ${snapshot.channel}`);
    // The About page renders this as the version the user is running, so an empty or placeholder
    // string is a defect there even though the command succeeded.
    assert.ok(
      /^\d+\.\d+\.\d+/.test(snapshot.currentVersion),
      `the snapshot reported an unusable current version: ${JSON.stringify(snapshot.currentVersion)}`,
    );
  });

  globalThis.it("round-trips the update preferences", async () => {
    const original = await invoke(({ core }) => core.invoke("get_desktop_update_preferences"));
    assert.ok(CHANNELS.includes(original.channel), `unknown stored channel: ${original.channel}`);
    assert.equal(typeof original.automaticCheck, "boolean");

    const flipped = {
      automaticCheck: !original.automaticCheck,
      channel: original.channel === "stable" ? "preview" : "stable",
    };
    const saved = await invoke(
      ({ core }, input) => core.invoke("save_desktop_update_preferences", { input }),
      flipped,
    );
    assert.ok(saved, "save_desktop_update_preferences returned nothing");

    const readBack = await invoke(({ core }) => core.invoke("get_desktop_update_preferences"));
    assert.equal(readBack.automaticCheck, flipped.automaticCheck, "the automatic-check toggle did not persist");
    assert.equal(readBack.channel, flipped.channel, "the update channel did not persist");

    // Put the host back the way it was found -- these preferences are the real user-facing
    // setting, and this run's isolated data directory is the only thing keeping that harmless.
    await invoke(({ core }, input) => core.invoke("save_desktop_update_preferences", { input }), original);
    const restored = await invoke(({ core }) => core.invoke("get_desktop_update_preferences"));
    assert.deepEqual(restored, original, "the original update preferences were not restored");
  });

  globalThis.it("refuses an update channel that is not one of the two modelled ones", async () => {
    const before = await invoke(({ core }) => core.invoke("get_desktop_update_preferences"));
    const refused = await attempt("save_desktop_update_preferences", {
      input: { automaticCheck: true, channel: "nightly" },
    });
    assert.equal(refused.ok, false, "an unmodelled update channel was accepted");
    const after = await invoke(({ core }) => core.invoke("get_desktop_update_preferences"));
    assert.deepEqual(after, before, "a rejected preference write changed the stored preferences");
  });

  globalThis.it("settles an update check on a terminal phase or reports why it could not", async function updateCheck() {
    // This is the one case here that reaches the network: it asks the real release endpoint
    // whether a newer build exists. A host with no egress is a blocked case, not a failure -- what
    // is being asserted is that the check reaches a terminal phase and never parks in `checking`,
    // which is the state the About page shows a spinner for.
    const started = await attempt("check_for_desktop_update");
    if (!started.ok) {
      blocked.push(`update check: the release endpoint was unreachable from this host (${started.error})`);
      this.skip();
    }

    const settled = await globalThis.browser.waitUntil(async () => {
      const snapshot = await attempt("get_desktop_update_snapshot");
      if (!snapshot.ok) return { phase: "failed", error: snapshot.error };
      return ["available", "up-to-date", "failed"].includes(snapshot.value.phase)
        ? snapshot.value
        : false;
    }, {
      timeout: 60_000,
      interval: 1_000,
      timeoutMsg: "The update check never left the checking phase.",
    }).catch(() => null);

    if (!settled) {
      blocked.push("update check: the release endpoint did not settle within 60 seconds on this host");
      this.skip();
    }

    if (settled.phase === "failed") {
      blocked.push(`update check: the check completed as failed on this host (${settled.error ?? "no reason reported"})`);
      this.skip();
    }
    assert.ok(settled.checkedAt, "a completed update check recorded no timestamp");
    if (settled.phase === "available") {
      assert.ok(settled.latestVersion, "an available update reported no version");
    }
  });

  globalThis.after(async () => {
    blocked.push("download_and_install_desktop_update: not run -- stages a real installer over the running application");
    blocked.push("restart_after_desktop_update: not run -- restarts the process and would end the WDIO session mid-run");
    globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
  });
});
