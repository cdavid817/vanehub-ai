import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error: String(error) }),
  ), { command, args: args ?? {} });
}

/**
 * Install-path coverage for every managed CLI, without installing anything.
 *
 * `native-flows` drives one real install, and only for opencode, deliberately: it reinstalls the
 * version already on disk so the host ends where it started, and doing the same to claude-code or
 * codex-cli would mutate the developer's working tools. That left the install command's validation
 * untested for four of the five Agents.
 *
 * Everything here goes through refusal paths. `prepare_install` resolves the definition, validates
 * the target version, and validates the package state -- all three before it acquires the mutation
 * lock or starts an operation (application/service.rs:182-192), so a rejected call cannot have
 * touched anything. That is what makes it safe to run against claude-code and codex-cli, and the
 * assertions below check exactly that nothing moved.
 */
const AGENT_IDS = ["claude-code", "codex-cli", "gemini-cli", "opencode", "antigravity-cli"];

/** Records what the catalogue says, so an unchanged catalogue can be asserted afterwards. */
async function catalogueSnapshot() {
  const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
  return new Map(tools.map((tool) => [
    tool.agentId,
    JSON.stringify({
      currentVersion: tool.currentVersion ?? null,
      installed: tool.installed ?? null,
      activeInstallationPath: tool.activeInstallationPath ?? null,
    }),
  ]));
}

globalThis.describe("VaneHub AI desktop CLI install validation", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );

    // Detection has to have settled before anything here compares two snapshots. On a fresh data
    // directory it runs in the background at startup, so a baseline taken too early holds nulls
    // and fills in on its own a moment later -- which reads as "the catalogue changed" and would
    // have this file reporting a mutation that never happened.
    const refresh = await invoke(({ core }) => core.invoke("refresh_cli_detections"));
    if (refresh?.id) {
      await globalThis.browser.waitUntil(async () => {
        const status = await invoke(
          ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
          refresh.id,
        );
        return ["succeeded", "failed", "cancelled"].includes(status.status);
      }, { timeout: 120_000, interval: 2_000, timeoutMsg: "CLI detection refresh never settled." });
    }
  });

  globalThis.it("publishes an install lifecycle for every managed CLI", async () => {
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    const byId = new Map(tools.map((tool) => [tool.agentId, tool]));

    const missing = AGENT_IDS.filter((id) => !byId.has(id));
    assert.deepEqual(missing, [], `managed CLIs absent from the catalogue: ${missing}`);

    // `lifecycle_eligibility` is what decides whether the page offers an install button at all,
    // and which mechanism it would use (application/service.rs:471-475). Every managed CLI has to
    // report one, or its card renders a control backed by nothing.
    for (const id of AGENT_IDS) {
      const tool = byId.get(id);
      assert.ok(
        tool.lifecycleEligibility,
        `${id} reported no install lifecycle: ${JSON.stringify(tool)}`,
      );
    }

    // antigravity-cli is the one with no npm package (domain/mod.rs:80-89), which is why the guide
    // says the CLI Management page offers it no npm install action. Asserted because it is the
    // difference that would silently disappear if the catalogue started treating them uniformly.
    const antigravity = byId.get("antigravity-cli");
    assert.equal(
      antigravity.packageName ?? null,
      null,
      "antigravity-cli reported an npm package, but it ships as a script install only",
    );
    for (const id of ["claude-code", "codex-cli", "gemini-cli", "opencode"]) {
      assert.ok(byId.get(id).packageName, `${id} reported no npm package`);
    }
  });

  globalThis.it("refuses a malformed target version for every CLI without touching it", async () => {
    const before = await catalogueSnapshot();

    // application/service.rs:461-468 -- only "latest" or a stable semantic version is accepted.
    // A pre-release is the interesting case: it parses as a version and is still refused, so this
    // is not just testing that garbage is rejected.
    const rejected = ["", "  ", "not-a-version", "1.2.3-beta.1", "v1.2.3-rc1", "../../etc/passwd"];
    const accepted = [];
    for (const agentId of AGENT_IDS) {
      for (const targetVersion of rejected) {
        const result = await attempt("install_cli_version", {
          agentId,
          targetVersion,
          confirmedActivePath: null,
        });
        if (result.ok) accepted.push(`${agentId} <- ${JSON.stringify(targetVersion)}`);
      }
    }
    assert.deepEqual(accepted, [], `install accepted a malformed target version:\n  ${accepted.join("\n  ")}`);

    const after = await catalogueSnapshot();
    for (const agentId of AGENT_IDS) {
      assert.equal(
        after.get(agentId),
        before.get(agentId),
        `${agentId} changed while only refused installs were attempted`,
      );
    }
  });

  globalThis.it("refuses an install for an Agent that is not a managed CLI", async () => {
    const before = await catalogueSnapshot();
    for (const agentId of ["onepiece", "no-such-agent", ""]) {
      // onepiece is the interesting one: it is a real, registered Agent with no CLI to install,
      // so this is the case where a valid Agent id must still be refused by the CLI surface.
      const result = await attempt("install_cli_version", {
        agentId,
        targetVersion: "latest",
        confirmedActivePath: null,
      });
      assert.equal(result.ok, false, `install accepted ${JSON.stringify(agentId)} as a managed CLI`);
    }
    assert.deepEqual(
      [...(await catalogueSnapshot()).entries()],
      [...before.entries()],
      "the catalogue changed while only refused installs were attempted",
    );
  });

  globalThis.it("requires the active path to be confirmed when a CLI is installed more than once", async function confirmedPath() {
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    // The guard is conditional, and that condition is the whole point of it: confirmation is only
    // demanded when more than one installation was found
    // (infrastructure/package_adapter.rs:109-122), because that is the only time there is anything
    // to disambiguate. This is the guide's "installation conflict -- upgrade only the active copy".
    //
    // With a single installation the parameter is ignored by design, so there is no refusal to
    // assert and calling anyway starts a real install. An earlier version of this case did exactly
    // that against claude-code before the condition was understood: harmless in outcome, because
    // `latest` was already the installed version and the symlink never moved, but a real vendor
    // installer run against a working tool -- which is the one thing this file exists to avoid.
    const conflicted = tools.find((tool) => (tool.installations?.length ?? 0) > 1);
    if (!conflicted) {
      blocked.push(
        "confirmed-path guard: no CLI on this host has more than one installation, and the "
        + "confirmation is only required to disambiguate one -- exercising it would mean starting "
        + "a real install",
      );
      this.skip();
    }

    const before = await catalogueSnapshot();
    const result = await attempt("install_cli_version", {
      agentId: conflicted.agentId,
      targetVersion: "latest",
      confirmedActivePath: "/nonexistent/path/that/is/not/active",
    });
    assert.equal(
      result.ok,
      false,
      `${conflicted.agentId} has ${conflicted.installations.length} installations and accepted an `
      + "install that named none of them as active",
    );
    assert.deepEqual(
      [...(await catalogueSnapshot()).entries()],
      [...before.entries()],
      "the catalogue changed after an install that was supposed to be refused",
    );
  });

  globalThis.after(async () => {
    blocked.push(
      "install_cli_version success path: exercised for opencode only, in native-flows, by "
      + "reinstalling the version already on disk -- installing anything for claude-code, "
      + "codex-cli, gemini-cli or antigravity-cli would mutate the developer's working tools",
    );
    blocked.push(
      "upgrade_all_cli_versions: not run -- it upgrades every eligible CLI on the host at once",
    );
    globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
  });
});
