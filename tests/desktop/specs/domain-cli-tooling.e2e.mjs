import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// src-tauri/src/contexts/tooling/cli_parameters.rs:25 (MANAGED_CLI_AGENT_IDS). `list_profiles`
// (same file:770) maps over the constant in order, so the response order is itself the contract.
const MANAGED_CLI_AGENT_IDS = ["claude-code", "codex-cli", "gemini-cli", "opencode", "antigravity-cli"];
// src-tauri/src/contexts/tooling/cli_parameters.rs:33-54 -- the serde renames on the control
// (kebab-case), risk (lowercase) and launch-scope (lowercase) enums.
const PARAMETER_CONTROLS = ["enum", "boolean", "multi-enum", "custom-text"];
const PARAMETER_RISKS = ["normal", "warning"];
const PARAMETER_LAUNCH_SCOPES = ["interactive", "chat"];
// opencode's editable catalogue. `agent` and `autoApprove` are policy-governed
// (src-tauri/src/contexts/tooling/cli_parameters.rs:403-419), so they are filtered out of both the
// definitions and the selections a profile exposes; they belong to the policy template layer.
const OPENCODE_PARAMETER_IDS = ["model", "variant", "thinking"];
const OPENCODE_PARAMETER_FLAGS = ["--model", "--variant", "--thinking"];
const OPENCODE_DEFAULT_SELECTIONS = { model: "default", variant: "default", thinking: false };

// src-tauri/src/contexts/tooling/cli_config/domain/mod.rs:8 (SUPPORTED_AGENT_IDS) -- deliberately a
// different order from the CLI parameter constant above.
const CLI_CONFIG_AGENT_IDS = ["claude-code", "opencode", "codex-cli", "antigravity-cli", "gemini-cli"];
// src-tauri/src/contexts/tooling/cli_config/infrastructure/live_config.rs:60-80 (`primary_path`).
const PRIMARY_CONFIG_SUFFIX = {
  "claude-code": ".claude/settings.json",
  opencode: ".config/opencode/opencode.json",
  "codex-cli": ".codex/config.toml",
  "antigravity-cli": ".gemini/antigravity-cli/settings.json",
  "gemini-cli": ".gemini/.env",
};
// src-tauri/src/contexts/tooling/cli_config/domain/mod.rs:130-174 -- the `kind` tag is the
// kebab-cased variant name, which for Antigravity is *not* its Agent id.
const CONFIG_PAYLOAD_KIND = {
  "claude-code": "claude-code",
  opencode: "opencode",
  "codex-cli": "codex-cli",
  "antigravity-cli": "antigravity",
  "gemini-cli": "gemini-cli",
};
// src-tauri/src/contexts/tooling/cli_config/domain/mod.rs:26-61 -- drift, validation, applied and
// startup-sync states, all kebab-case.
const DRIFT_STATES = ["detached", "applied", "drifted", "malformed", "missing"];
const VALIDATION_STATES = ["valid", "needs-credential", "invalid"];
const APPLIED_STATES = ["saved", "applied", "drifted"];
const STARTUP_SYNC_STATES = ["pending", "imported", "updated", "unchanged", "skipped", "warning", "unavailable"];
// src-tauri/src/contexts/tooling/plugin_integrations/domain/lifecycle.rs:12-28 -- status wire
// values and the reason key each one is reported with.
const READINESS_REASON_KEY = {
  configured: "plugins.statusReason.configured",
  "not-configured": "plugins.statusReason.notConfigured",
  "missing-cli": "plugins.statusReason.missingCli",
  error: "plugins.statusReason.error",
};

async function readParameterProfiles() {
  // src-tauri/src/commands/tooling/cli_parameters/list_cli_parameter_profiles.rs:6 -- no arguments.
  return invoke(({ core }) => core.invoke("list_cli_parameter_profiles"));
}

async function readParameterProfile(agentId) {
  const profile = (await readParameterProfiles()).find((entry) => entry.agentId === agentId);
  assert.ok(profile, `${agentId} is missing from the CLI parameter catalogue`);
  return profile;
}

async function saveParameterProfile(agentId, selections) {
  // src-tauri/src/commands/tooling/cli_parameters/save_cli_parameter_profile.rs:8 takes `input`;
  // `SaveCliParameterProfileInput` (src-tauri/src/contexts/tooling/cli_parameters.rs:88-93,
  // camelCase) is `{ agentId, selections }`.
  return invoke(({ core }, input) => core.invoke("save_cli_parameter_profile", { input }), {
    agentId,
    selections,
  });
}

async function resetParameterProfile(agentId) {
  // src-tauri/src/commands/tooling/cli_parameters/reset_cli_parameter_profile.rs:6 -- `agent_id`
  // crosses the boundary camelCased (src/services/tauri-agent-client.ts:554).
  return invoke(({ core }, id) => core.invoke("reset_cli_parameter_profile", { agentId: id }), agentId);
}

async function readConfigStatus(agentId) {
  // src-tauri/src/commands/tooling/cli_config/get_cli_config_status.rs:6, camelCased at the
  // boundary as src/services/tauri-agent-client.ts:566 shows.
  return invoke(({ core }, id) => core.invoke("get_cli_config_status", { agentId: id }), agentId);
}

// Content hashes rather than mtimes: a rewrite is what the read-only constraint forbids, and an
// absent file has to stay absent so a read path cannot conjure one into existence.
async function fingerprintFiles(paths) {
  const entries = await Promise.all([...new Set(paths)].map(async (path) => {
    try {
      return [path, createHash("sha256").update(await readFile(path)).digest("hex")];
    } catch {
      return [path, "absent"];
    }
  }));
  return Object.fromEntries(entries);
}

globalThis.describe("VaneHub AI desktop CLI tooling domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("reports the CLI parameter catalogue for every managed CLI Agent", async () => {
    const profiles = await readParameterProfiles();
    assert.deepEqual(
      profiles.map((profile) => profile.agentId),
      MANAGED_CLI_AGENT_IDS,
      "the catalogue no longer covers the managed CLI Agents in registry order",
    );

    for (const profile of profiles) {
      assert.ok(profile.definitions.length > 0, `${profile.agentId} exposes no editable parameters`);
      // Selections and definitions are derived from the same filtered catalogue
      // (cli_parameters.rs:682-700), so a key on one side and not the other is a real defect: the
      // UI would render a control with no value, or hold a value with no control.
      assert.deepEqual(
        Object.keys(profile.selections).sort(),
        profile.definitions.map((definition) => definition.id).sort(),
        `${profile.agentId} selections and definitions disagree`,
      );
      for (const definition of profile.definitions) {
        assert.equal(definition.agentId, profile.agentId);
        assert.ok(PARAMETER_CONTROLS.includes(definition.control), `unexpected control ${definition.control}`);
        assert.ok(PARAMETER_RISKS.includes(definition.risk), `unexpected risk ${definition.risk}`);
        assert.ok(definition.launchScopes.length > 0, `${definition.id} declares no launch scope`);
        assert.ok(
          definition.launchScopes.every((scope) => PARAMETER_LAUNCH_SCOPES.includes(scope)),
          `${definition.id} declares an unexpected launch scope`,
        );
        // The catalogue is the surface a user can flip from the UI, so bypass flags must never
        // reach it (asserted natively at cli_parameters.rs:1076, restated here end to end).
        assert.equal(definition.flag.includes("dangerously"), false, `${definition.id} exposes a bypass flag`);
      }
      assert.ok(Array.isArray(profile.previewArgs), `${profile.agentId} rendered no preview arguments`);
    }

    const opencode = profiles.find((profile) => profile.agentId === "opencode");
    assert.deepEqual(opencode.definitions.map((definition) => definition.id), OPENCODE_PARAMETER_IDS);
    assert.deepEqual(opencode.definitions.map((definition) => definition.flag), OPENCODE_PARAMETER_FLAGS);
    for (const governed of ["agent", "autoApprove"]) {
      assert.equal(
        Object.hasOwn(opencode.selections, governed),
        false,
        `${governed} is policy-governed and must not be editable as a launch parameter`,
      );
    }
  });

  globalThis.it("saves, reads back and resets the opencode parameter selection", async () => {
    const before = await readParameterProfiles();
    const original = before.find((profile) => profile.agentId === "opencode");
    const untouched = before.filter((profile) => ["claude-code", "codex-cli"].includes(profile.agentId));

    try {
      // Scoped to opencode by explicit instruction: claude-code and codex-cli keep whatever
      // configuration this host already has.
      const saved = await saveParameterProfile("opencode", { variant: "high", thinking: true });
      assert.equal(saved.agentId, "opencode");
      // `model` is absent from the request and is backfilled from its default
      // (cli_parameters.rs:504-521), so a stored selection is always complete.
      assert.deepEqual(saved.selections, { model: "default", variant: "high", thinking: true });
      // Rendered for the chat launch scope in definition order (cli_parameters.rs:689-693):
      // `model` stays at "default" and contributes nothing, `variant` renders flag plus value, and
      // the boolean renders its flag alone.
      assert.deepEqual(saved.previewArgs, ["--variant", "high", "--thinking"]);

      const reread = await readParameterProfile("opencode");
      assert.deepEqual(reread.selections, saved.selections, "the saved selection did not survive a read");
      assert.deepEqual(reread.previewArgs, saved.previewArgs);

      const others = (await readParameterProfiles())
        .filter((profile) => ["claude-code", "codex-cli"].includes(profile.agentId));
      assert.deepEqual(
        others.map((profile) => profile.selections),
        untouched.map((profile) => profile.selections),
        "saving one Agent's parameters changed another Agent's",
      );

      // A policy-governed parameter is not merely ignored on save, it is rejected: dropping it
      // silently would let the launch surface disagree with the applied policy template.
      await assert.rejects(() => saveParameterProfile("opencode", { autoApprove: true }));
      assert.deepEqual(
        (await readParameterProfile("opencode")).selections,
        saved.selections,
        "the rejected save still mutated the stored profile",
      );
      // A boolean control must refuse a string, and must do so before anything is written.
      await assert.rejects(() => saveParameterProfile("opencode", { thinking: "yes" }));
      assert.deepEqual((await readParameterProfile("opencode")).selections, saved.selections);

      const reset = await resetParameterProfile("opencode");
      assert.deepEqual(reset.selections, OPENCODE_DEFAULT_SELECTIONS, "reset did not restore the defaults");
      assert.deepEqual(reset.previewArgs, [], "a default profile still contributes launch arguments");
      assert.deepEqual((await readParameterProfile("opencode")).selections, OPENCODE_DEFAULT_SELECTIONS);
    } finally {
      // Spec files in one run share a single data directory. Reset again defensively -- a failure
      // before the reset above would otherwise hand this sweep's selection to the next spec -- then
      // put back whatever this host started with.
      await resetParameterProfile("opencode").catch(() => {});
      if (JSON.stringify(original.selections) !== JSON.stringify(OPENCODE_DEFAULT_SELECTIONS)) {
        await saveParameterProfile("opencode", original.selections).catch(() => {});
      }
    }
  });

  globalThis.it("reports the built-in plugin integration catalogue and its native environment", async () => {
    // src-tauri/src/commands/tooling/plugin_integrations/get_plugin_integration_overview.rs:7 --
    // no arguments. The definition is a compile-time constant
    // (src-tauri/src/contexts/tooling/plugin_integrations/domain/catalog.rs:62-71), so it is
    // asserted whole rather than sampled.
    const overview = await invoke(({ core }) => core.invoke("get_plugin_integration_overview"));
    assert.deepEqual(overview.definitions, [{
      id: "github",
      nameKey: "plugins.github.name",
      descriptionKey: "plugins.github.description",
      version: "1.0.0",
      provider: "GitHub",
      icon: "github",
      docsUrl: "https://cli.github.com/manual/gh_auth_login",
      setupSteps: [
        { id: "install", labelKey: "plugins.github.setup.install" },
        { id: "auth", labelKey: "plugins.github.setup.auth" },
      ],
    }]);
    // The one assertion the web/mock adapter cannot satisfy: this is the native runtime, so a
    // readiness check really does shell out (domain/lifecycle.rs:67-73).
    assert.deepEqual(overview.environment, { runtime: "tauri", nativeChecksAvailable: true, reasonKey: null });
    assert.deepEqual(overview.states, [{
      integrationId: "github",
      status: "not-configured",
      configured: false,
      canTest: true,
      lastCheckedAt: null,
      statusReasonKey: "plugins.statusReason.notChecked",
      message: null,
    }]);

    // src-tauri/src/contexts/tooling/plugin_integrations/application/service.rs:42-44 -- refresh is
    // overview; the command exists for the UI's explicit refresh affordance, not for new data.
    const refreshed = await invoke(({ core }) => core.invoke("refresh_plugin_integrations"));
    assert.deepEqual(refreshed, overview, "refresh returned something other than the overview");
  });

  globalThis.it("runs the real GitHub readiness check and leaves the catalogue stateless", async () => {
    // src-tauri/src/commands/tooling/plugin_integrations/test_plugin_integration.rs:7 takes
    // `request`; `PluginIntegrationRequest` (../plugin_integrations/dto.rs:67-71) is
    // `{ integrationId }` and `PluginIntegrationId` is kebab-case (dto.rs:3-7), so "github".
    // This really executes `gh auth status` on a 10s budget (domain/catalog.rs:73-78): it reads
    // the developer's gh state and writes nothing.
    const result = await invoke(
      ({ core }, request) => core.invoke("test_plugin_integration", { request }),
      { integrationId: "github" },
    );
    assert.equal(result.integrationId, "github");
    assert.ok(
      Object.hasOwn(READINESS_REASON_KEY, result.status),
      `unexpected readiness status: ${result.status}`,
    );
    assert.equal(result.configured, result.status === "configured");
    assert.equal(result.message, READINESS_REASON_KEY[result.status], "status and reason key disagree");
    assert.ok(Number.isFinite(Date.parse(result.checkedAt)), `checkedAt is not a timestamp: ${result.checkedAt}`);
    if (result.status !== "configured") {
      // The check ran and answered; `gh` simply is not installed or not authenticated here. That is
      // a host fact, recorded rather than asserted away.
      blocked.push(`github readiness: ${result.status}`);
    }

    // The overview is recomputed from the catalogue on every call, so a completed readiness check
    // is deliberately not remembered. Pinned because it is surprising: the UI has to keep the
    // result it was handed rather than re-reading it.
    const after = await invoke(({ core }) => core.invoke("get_plugin_integration_overview"));
    assert.equal(after.states[0].lastCheckedAt, null, "the overview started remembering readiness results");
    assert.equal(after.states[0].status, "not-configured");

    // Unknown ids never reach the domain: the DTO enum has a single variant, so deserialization
    // rejects them at the command boundary.
    await assert.rejects(() => invoke(
      ({ core }, request) => core.invoke("test_plugin_integration", { request }),
      { integrationId: "gitlab" },
    ));
  });

  globalThis.it("reads CLI global configuration without touching the developer's real files", async () => {
    const statuses = new Map();
    for (const agentId of CLI_CONFIG_AGENT_IDS) {
      statuses.set(agentId, await readConfigStatus(agentId));
    }
    // The real, unredirected targets under `dirs::home_dir()`
    // (src-tauri/src/contexts/tooling/cli_config/infrastructure/live_config.rs:41). Everything
    // below this line is a read, and the closing assertion is what proves it.
    const realPaths = [...statuses.values()].flatMap((status) => status.resolvedPaths);
    const fingerprintBefore = await fingerprintFiles(realPaths);

    for (const agentId of CLI_CONFIG_AGENT_IDS) {
      const status = await readConfigStatus(agentId);
      assert.equal(status.agentId, agentId);
      assert.ok(DRIFT_STATES.includes(status.driftState), `${agentId} reported drift state ${status.driftState}`);
      // The native adapter hardcodes this false (api.rs:411); true would mean the mock answered.
      assert.equal(status.simulated, false, `${agentId} status came from a simulated adapter`);
      // live_config.rs:94-101 -- codex-cli is the only Agent whose auth.json is a second target.
      assert.equal(status.resolvedPaths.length, agentId === "codex-cli" ? 2 : 1);
      assert.ok(
        status.resolvedPaths[0].replaceAll("\\", "/").endsWith(PRIMARY_CONFIG_SUFFIX[agentId]),
        `${agentId} resolved an unexpected primary path`,
      );
      if (agentId === "codex-cli") {
        assert.ok(status.resolvedPaths[1].replaceAll("\\", "/").endsWith(".codex/auth.json"));
      }
      // src-tauri/src/bootstrap/runtime.rs:150 runs the startup sync synchronously during setup, so
      // by the time any spec runs the per-Agent result must have left "pending".
      assert.equal(status.startupSync.agentId, agentId);
      assert.ok(
        STARTUP_SYNC_STATES.includes(status.startupSync.state),
        `unexpected sync state ${status.startupSync.state}`,
      );
      assert.notEqual(status.startupSync.state, "pending", `${agentId} never ran its startup sync`);
      assert.ok(Array.isArray(status.startupSync.warnings));
      assert.equal(status.startupSync.simulated, false);

      // src/services/tauri-agent-client.ts:562 -- `{ agentId }`.
      const profiles = await invoke(
        ({ core }, id) => core.invoke("list_cli_config_profiles", { agentId: id }),
        agentId,
      );
      for (const profile of profiles) {
        assert.equal(profile.agentId, agentId);
        assert.equal(profile.payloadVersion, 1);
        assert.equal(profile.payload.kind, CONFIG_PAYLOAD_KIND[agentId], `${agentId} payload tag drifted`);
        assert.equal(typeof profile.credentialConfigured, "boolean");
        assert.ok(VALIDATION_STATES.includes(profile.validationState));
        assert.ok(APPLIED_STATES.includes(profile.appliedState));
        // The secret lives in the OS keyring and must never ride back out on the read path; only
        // the boolean above may cross (api.rs:930-968).
        assert.equal(Object.hasOwn(profile, "credential"), false, "a profile echoed its credential");
      }

      if (agentId === "antigravity-cli") {
        // `discover_current` has no antigravity arm once the settings file exists
        // (live_config.rs:183-202), so whether this call answers is a property of the host rather
        // than of the product: it succeeds with nothing to report while the file is absent and is
        // rejected outright once it exists. Probed and reported as a source-level gap instead of
        // failing a run over a hole this spec did not create.
        const probe = await invoke(
          ({ core }, id) => core.invoke("discover_cli_config_profiles", { agentId: id }),
          agentId,
        ).catch(() => null);
        if (probe === null) {
          blocked.push("cli_config discovery: antigravity-cli is rejected by discover_current once its settings file exists");
        } else {
          assert.equal(probe.agentId, agentId);
          assert.equal(probe.simulated, false);
          assert.deepEqual(probe.candidates, [], "antigravity discovery answered with candidates it has no arm to build");
        }
        continue;
      }

      const discovery = await invoke(
        ({ core }, id) => core.invoke("discover_cli_config_profiles", { agentId: id }),
        agentId,
      );
      assert.equal(discovery.agentId, agentId);
      assert.ok(["available", "parse-error"].includes(discovery.state), `${agentId} discovery state ${discovery.state}`);
      assert.equal(discovery.simulated, false);
      assert.ok(Array.isArray(discovery.candidates) && Array.isArray(discovery.warnings));
      for (const candidate of discovery.candidates) {
        assert.equal(typeof candidate.candidateKey, "string");
        assert.ok(candidate.suggestedName.length > 0, "a discovered candidate has no suggested name");
        assert.equal(typeof candidate.credentialDetected, "boolean");
        assert.equal(typeof candidate.isDefault, "boolean");
        assert.ok(Array.isArray(candidate.resolvedPaths));
        // Discovery reads the live file, credential included; only the fact of one may cross the
        // IPC boundary (api.rs:649-658).
        assert.equal(Object.hasOwn(candidate, "credential"), false, "discovery leaked a live credential");
      }
    }

    assert.deepEqual(
      await fingerprintFiles(realPaths),
      fingerprintBefore,
      "a CLI configuration read path rewrote a real file under the developer's home directory",
    );
  });

  globalThis.it("validates a credential-free CLI configuration without a network call", async () => {
    // src-tauri/src/commands/tooling/cli_config/validate_cli_config_credential.rs:7 takes `input`;
    // `ValidateCliConfigCredentialInput` (../cli_config/domain/mod.rs:214-222) is
    // `{ agentId, profileId, payload, sourcePresetId, credential }`. The Antigravity payload
    // (domain/mod.rs:161-167, tag "antigravity", `AntigravityToolPermission` kebab-case at :115-122)
    // authenticates through the OS keyring, so `requires_credential()` is false and the command
    // short-circuits at api.rs:354 -- no probe, no keyring read, and nothing written anywhere.
    const payload = {
      kind: "antigravity",
      toolPermission: "request-review",
      enableTerminalSandbox: false,
      verbosity: "high",
      model: "gemini-3-pro",
      advancedSettings: {},
    };
    const result = await invoke(({ core }, input) => core.invoke("validate_cli_config_credential", { input }), {
      agentId: "antigravity-cli",
      profileId: null,
      payload,
      sourcePresetId: null,
      credential: null,
    });
    // platform/network/provider_credential_probe.rs:55-61, serialized kebab-case/camelCase at :27-45.
    assert.deepEqual(result, { status: "unsupported", latencyMs: 0, httpStatus: null });

    // The tagged payload has to match the Agent it is submitted under (api.rs:348-352), or a
    // mis-wired caller could validate one CLI's endpoint against another's credential.
    await assert.rejects(() => invoke(({ core }, input) => core.invoke("validate_cli_config_credential", { input }), {
      agentId: "claude-code",
      profileId: null,
      payload,
      sourcePresetId: null,
      credential: null,
    }));
  });

  globalThis.it("leaves every CLI global configuration write path unrun", async function writePathsAreOffLimits() {
    // Not skipped for want of a fixture. `NativeCliGlobalConfigAdapter` resolves its targets
    // through `dirs::home_dir()` (live_config.rs:41) and credentials go to the real OS keyring
    // (cli_config/infrastructure/credential_adapter.rs:16); VANEHUB_APP_DATA_DIR redirects neither.
    // Running any of these would edit this developer's own ~/.claude/settings.json,
    // ~/.codex/config.toml, ~/.codex/auth.json, ~/.config/opencode/opencode.json, ~/.gemini/.env or
    // ~/.gemini/antigravity-cli/settings.json, or plant a secret in their credential manager.
    for (const [command, effect] of [
      ["apply_cli_config_profile", "projects a profile onto the live config files and rewrites auth.json"],
      ["save_cli_config_profile", "stores a profile and writes its credential to the real OS keyring"],
      ["import_cli_config_profile", "imports the live config and copies its credential into the keyring"],
      ["import_discovered_cli_config_profiles", "the same, in bulk, for every selected discovered candidate"],
      ["duplicate_cli_config_profile", "duplicates the stored credential under a second keyring account"],
      ["delete_cli_config_profile", "deletes the keyring credential and can detach the applied live config"],
    ]) {
      blocked.push(`cli_config ${command}: not run -- ${effect}`);
    }
    // Reported as skipped rather than passed: these capabilities have no coverage on this host.
    this.skip();
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // Teardown is left to WDIO on purpose: calling `exit_application` here races `deleteSession`
    // and discards every per-test result for this file.
  });
});
