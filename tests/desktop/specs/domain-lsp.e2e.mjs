import assert from "node:assert/strict";
import { mkdir, mkdtemp, realpath } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";

import { comparableFilesystemPath } from "../helpers/filesystem-path.mjs";

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
    // Deliberately omits `descriptors` and `startupArguments`. This spec reaches the command
    // through `core.invoke` rather than the frontend adapter, so it is the only layer that proves
    // a caller does not have to restate the descriptor list the backend itself authored. Making
    // `descriptors` a required input broke exactly this and nothing else.
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

    // Descriptors come back even though they were never sent: the registry is the authority on
    // which languages exist, and the reply is where the frontend learns it.
    assert.ok(Array.isArray(read.descriptors) && read.descriptors.length > 0, "no descriptors returned");
    for (const descriptor of read.descriptors) {
      assert.match(descriptor.language, /^[a-z0-9_]{1,64}$/);
      assert.match(descriptor.server, /^[a-z0-9_]{1,64}$/);
      assert.equal(typeof descriptor.supportedOnHost, "boolean");
      assert.ok(Array.isArray(descriptor.defaultStartupArguments));
    }
    // Every configured language must be one the same reply describes.
    const declared = new Set(read.descriptors.map((entry) => entry.language));
    for (const entry of read.languages) {
      assert.ok(declared.has(entry.language), `${entry.language} has no descriptor`);
    }
  });

  globalThis.it("keeps unset startup arguments distinct from an explicit empty list", async () => {
    // NULL means "use the registry default"; [] means the user chose to pass none. Collapsing the
    // two would silently restore `--stdio` for the TypeScript server after someone cleared it, and
    // only a real round trip through SQLite proves the column keeps them apart.
    const save = (languages) => invoke(
      ({ core }, config) => core.invoke("save_lsp_configuration", { configuration: config }),
      { enabled: true, languages },
    );
    const read = () => invoke(({ core }) => core.invoke("get_lsp_configuration"));
    const languageOf = (configuration, id) => configuration.languages.find(
      (entry) => entry.language === id,
    );

    await save([
      {
        language: "rust",
        enabled: false,
        executableOverride: null,
        startupArguments: ["--log-file", "trace.log"],
        initializationOptions: {},
      },
      {
        language: "typescript_javascript",
        enabled: false,
        executableOverride: null,
        startupArguments: [],
        initializationOptions: {},
      },
    ]);

    const stored = await read();
    assert.deepEqual(languageOf(stored, "rust")?.startupArguments, ["--log-file", "trace.log"]);
    assert.deepEqual(languageOf(stored, "typescript_javascript")?.startupArguments, []);

    await save([
      {
        language: "rust",
        enabled: false,
        executableOverride: null,
        startupArguments: null,
        initializationOptions: {},
      },
    ]);
    assert.equal(languageOf(await read(), "rust")?.startupArguments, null);
  });

  globalThis.it("discovers this host's LSP servers and reports a reason for each unavailable one", async () => {
    // lsp.ts:20-22,81-88 -- discovery does not spawn anything; it only reports whether a server
    // binary can be resolved (`executablePath`) and, when it cannot, one of the documented safe
    // reason codes rather than a raw OS error.
    const { descriptors } = await invoke(({ core }) => core.invoke("get_lsp_configuration"));
    const languages = new Set(descriptors.map((entry) => entry.language));
    const servers = new Set(descriptors.map((entry) => entry.server));

    const discoveries = await invoke(({ core }) => core.invoke("discover_lsp_servers"));
    assert.ok(Array.isArray(discoveries) && discoveries.length > 0, "discovery reported no servers at all");
    // Checked against the registry the same build reports rather than a list pinned here, so
    // registering a language does not make this spec wrong.
    for (const discovery of discoveries) {
      assert.ok(languages.has(discovery.language), `${discovery.language} is not a registered language`);
      assert.ok(servers.has(discovery.server), `${discovery.server} is not a registered server`);
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
  });

  globalThis.it("trusts and revokes a real workspace root", async () => {
    await mkdir(fixtureRoot, { recursive: true });
    const workspace = await realpath(await mkdtemp(join(fixtureRoot, "lsp-workspace-")));

    const trusted = await invoke(
      ({ core }, update) => core.invoke("update_lsp_workspace_trust", { update }),
      { canonicalRoot: workspace, trusted: true },
    );
    assert.equal(trusted.trusted, true);
    assert.equal(comparableFilesystemPath(trusted.canonicalRoot), comparableFilesystemPath(workspace));
    const firstRevision = trusted.revision;

    const list = await invoke(({ core }) => core.invoke("list_lsp_workspace_trust"));
    assert.ok(
      list.some((entry) => (
        comparableFilesystemPath(entry.canonicalRoot) === comparableFilesystemPath(workspace)
        && entry.trusted
      )),
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
