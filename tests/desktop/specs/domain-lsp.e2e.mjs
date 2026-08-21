import assert from "node:assert/strict";
import { mkdir, mkdtemp, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();

globalThis.describe("VaneHub AI desktop LSP code intelligence domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("saves and reads back a per-language LSP configuration", async () => {
    // types/lsp.ts:5-8 -- exactly two languages are modeled today: rust and
    // typescript_javascript.
    const configuration = {
      enabled: true,
      languages: [
        { language: "rust", enabled: true, executableOverride: null, initializationOptions: {} },
        {
          language: "typescript_javascript",
          enabled: false,
          executableOverride: null,
          initializationOptions: { checkJs: true },
        },
      ],
    };
    await invoke(({ core }, config) => core.invoke("save_lsp_configuration", { configuration: config }), configuration);

    const read = await invoke(({ core }) => core.invoke("get_lsp_configuration"));
    assert.equal(read.enabled, true);
    const rust = read.languages.find((entry) => entry.language === "rust");
    const typescript = read.languages.find((entry) => entry.language === "typescript_javascript");
    assert.equal(rust?.enabled, true);
    assert.equal(typescript?.enabled, false);
    assert.deepEqual(typescript?.initializationOptions, { checkJs: true });
  });

  globalThis.it("discovers this host's LSP servers and reports a reason for each unavailable one", async () => {
    // lsp.ts:20-22,81-88 -- discovery does not spawn anything; it only reports whether a server
    // binary can be resolved (`executablePath`) and, when it cannot, one of the documented safe
    // reason codes rather than a raw OS error.
    const discoveries = await invoke(({ core }) => core.invoke("discover_lsp_servers"));
    assert.ok(Array.isArray(discoveries) && discoveries.length > 0, "discovery reported no servers at all");
    for (const discovery of discoveries) {
      assert.ok(["rust", "typescript_javascript"].includes(discovery.language));
      assert.ok(["rust_analyzer", "typescript_language_server"].includes(discovery.server));
      if (discovery.availability === "unavailable") {
        assert.equal(discovery.executablePath, null);
        assert.ok(discovery.reasonCode, `${discovery.server} reported unavailable with no reason code`);
      } else {
        assert.ok(discovery.executablePath, `${discovery.server} reported available with no executable path`);
      }
    }
  });

  globalThis.it("reports an empty live server process list when nothing has been started", async () => {
    // api.rs:358-366 -- `list_lsp_server_status` snapshots the live process registry, not a
    // static list of configured languages (that is what `get_lsp_configuration` and
    // `discover_lsp_servers` are for). Nothing in this run has started a server, so an empty
    // array is the correct answer, not a missing one.
    const statuses = await invoke(({ core }) => core.invoke("list_lsp_server_status"));
    assert.ok(Array.isArray(statuses), "list_lsp_server_status did not return an array");
    assert.equal(statuses.length, 0, "a fresh run already had a live LSP server process tracked");
    for (const status of statuses) {
      assert.ok(["rust", "typescript_javascript"].includes(status.language));
    }
  });

  globalThis.it("trusts and revokes a real workspace root", async () => {
    await mkdir(fixtureRoot, { recursive: true });
    const workspace = await realpath(await mkdtemp(join(fixtureRoot, "lsp-workspace-")));

    const trusted = await invoke(
      ({ core }, update) => core.invoke("update_lsp_workspace_trust", { update }),
      { canonicalRoot: workspace, trusted: true },
    );
    assert.equal(trusted.trusted, true);
    assert.equal(trusted.canonicalRoot, workspace);
    const firstRevision = trusted.revision;

    const list = await invoke(({ core }) => core.invoke("list_lsp_workspace_trust"));
    assert.ok(
      list.some((entry) => entry.canonicalRoot === workspace && entry.trusted),
      "the trusted workspace did not appear in list_lsp_workspace_trust",
    );

    // Revoking is the path that also tears down any live server process for the workspace
    // (code_intelligence/api.rs:154-158) -- there is none here, so this only exercises the state
    // transition and revision bump, not the process teardown itself.
    const revoked = await invoke(
      ({ core }, update) => core.invoke("update_lsp_workspace_trust", { update }),
      { canonicalRoot: workspace, trusted: false },
    );
    assert.equal(revoked.trusted, false);
    assert.ok(revoked.revision > firstRevision, "revoking trust did not bump the revision");
  });

  globalThis.it("runs a real server-test round trip and reports which phase it reached", async () => {
    // lsp.ts:25-27,89-98 -- discovery/spawn/initialize/cleanup, in order; a host with no server
    // binaries installed stops at "discovery" rather than failing the whole command, which is the
    // one thing this test can assert without a real rust-analyzer/typescript-language-server on
    // PATH.
    const result = await invoke(
      ({ core }, input) => core.invoke("test_lsp_server", { input }),
      { language: "rust" },
    );
    assert.equal(result.server, "rust_analyzer");
    assert.ok(Array.isArray(result.phases) && result.phases.length > 0, "the server test reported no phases");
    const discoveryPhase = result.phases.find((phase) => phase.phase === "discovery");
    assert.ok(discoveryPhase, "the server test never reached the discovery phase");
    if (discoveryPhase.status !== "succeeded") {
      blocked.push(
        `LSP server test: rust_analyzer discovery ${discoveryPhase.status} on this host (${discoveryPhase.reasonCode ?? "no reason code"}) -- negotiatedCapabilities not exercised`,
      );
    }
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
