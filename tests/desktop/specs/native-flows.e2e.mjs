import assert from "node:assert/strict";
import { join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// The repo's own MCP stdio fixture, which `mcp_fixture_contracts.rs` already certifies as a
// complete server. Pointing at it keeps the connection test real without reaching the network.
const MCP_FIXTURE = join(process.cwd(), "src-tauri", "tests", "fixtures", "mcp_stdio_server.cjs");
const MCP_NAME = "desktop-sweep-stdio";

async function settle(operationId, message) {
  return globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, id) => core.invoke("get_operation_status", { operationId: id }),
      operationId,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 180_000, interval: 1_000, timeoutMsg: message });
}

globalThis.describe("VaneHub AI desktop native flows", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("takes an MCP server through add, connect, toggle, update, and remove", async () => {
    await invoke(({ core }, config) => core.invoke("add_mcp_server", { config }), {
      name: MCP_NAME,
      transportType: "stdio",
      command: "node",
      args: [MCP_FIXTURE, "stdio"],
      env: {},
      url: null,
      headers: null,
      description: "Desktop sweep fixture",
      active: true,
      scope: "user",
      projectPath: null,
    });
    const listed = await invoke(({ core }) => core.invoke("list_mcp_servers"));
    const added = listed.find((server) => server.name === MCP_NAME);
    assert.ok(added, "the MCP server was not persisted");
    assert.equal(added.transportType, "stdio");

    // A real stdio handshake against the fixture, not a stored flag.
    const test = await invoke(({ core }, name) => core.invoke("test_mcp_connection", { name }), MCP_NAME);
    const outcome = await settle(test.id, "The MCP connection test never settled.");
    assert.equal(outcome.status, "succeeded", outcome.error ?? "MCP connection test failed");
    const status = await invoke(({ core }, name) => core.invoke("get_mcp_server_status", { name }), MCP_NAME);
    assert.ok(status, "no status was recorded for the tested server");

    await invoke(({ core }, input) => core.invoke("toggle_mcp_server", input), { name: MCP_NAME, active: false });
    const disabled = (await invoke(({ core }) => core.invoke("list_mcp_servers")))
      .find((server) => server.name === MCP_NAME);
    assert.equal(disabled.active, false, "toggling the server off did not persist");

    await invoke(({ core }, input) => core.invoke("update_mcp_server", input), {
      name: MCP_NAME,
      config: { description: "Updated by the desktop sweep" },
    });
    const updated = (await invoke(({ core }) => core.invoke("list_mcp_servers")))
      .find((server) => server.name === MCP_NAME);
    assert.equal(updated.description, "Updated by the desktop sweep");

    // Export has to reflect the live registry, since it is the migration path between machines.
    const exported = await invoke(({ core }, names) => core.invoke("export_mcp_servers", { names }), [MCP_NAME]);
    assert.ok(JSON.stringify(exported).includes(MCP_NAME), "the exported bundle omitted the server");

    await invoke(({ core }, name) => core.invoke("remove_mcp_server", { name }), MCP_NAME);
    const remaining = await invoke(({ core }) => core.invoke("list_mcp_servers"));
    assert.equal(remaining.find((server) => server.name === MCP_NAME), undefined, "removal did not persist");
  });

  globalThis.it("applies a policy template and reports it back for the Agent", async () => {
    const before = await invoke(({ core }, input) => core.invoke("get_agent_policy_principal", { input }), {
      agentId: "claude-code",
    });
    assert.ok(before.template, "no policy template was reported");

    for (const template of ["readonly", "standard"]) {
      const applied = await invoke(({ core }, input) => core.invoke("apply_policy_template", { input }), {
        agentId: "claude-code",
        template,
      });
      assert.equal(applied.template, template, `applying ${template} did not stick`);
      const readBack = await invoke(({ core }, input) => core.invoke("get_agent_policy_principal", { input }), {
        agentId: "claude-code",
      });
      assert.equal(readBack.template, template, `${template} did not survive a read`);
    }

    // Nothing was approved here, so the queue has to be empty rather than merely present.
    const pending = await invoke(({ core }) => core.invoke("list_pending_approvals"));
    assert.ok(Array.isArray(pending), "the pending approval queue is not readable");
  });

  globalThis.it("detects CLI tools and installs the version the UI offers for opencode", async function installOpencode() {
    const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
    assert.ok(tools.length >= 4, `expected the managed CLI catalogue, found ${tools.length}`);

    const refresh = await invoke(({ core }) => core.invoke("refresh_cli_detections"));
    if (refresh?.id) {
      await settle(refresh.id, "CLI detection refresh never settled.");
    }

    const refreshed = await invoke(({ core }) => core.invoke("list_cli_tools"));
    const opencode = refreshed.find((tool) => tool.agentId === "opencode");
    assert.ok(opencode, "opencode is missing from the CLI catalogue");
    // Reinstalling the version already on disk, deliberately: it drives the whole install
    // pipeline -- operation task, npm global install, detection refresh -- and leaves the host on
    // exactly the version it started on. Installing a different one would exercise the same code
    // and change the developer's working CLI as a side effect.
    const target = opencode.latestVersion ?? opencode.currentVersion;
    if (!target) {
      blocked.push("opencode install: the catalogue reported no installable version");
      this.skip();
    }

    // Scoped to opencode by explicit instruction: claude-code and codex-cli must not be mutated.
    const install = await invoke(({ core }, input) => core.invoke("install_cli_version", input), {
      agentId: "opencode",
      targetVersion: target,
      confirmedActivePath: opencode.activePath ?? null,
    });
    const outcome = await settle(install.id, "The opencode install never settled.");
    assert.equal(outcome.status, "succeeded", outcome.error ?? "opencode install failed");

    const after = await globalThis.browser.waitUntil(async () => {
      const tools = await invoke(({ core }) => core.invoke("list_cli_tools"));
      const entry = tools.find((tool) => tool.agentId === "opencode");
      return entry?.currentVersion === target ? entry : false;
    }, { timeout: 120_000, interval: 2_000, timeoutMsg: `opencode still does not report ${target}.` });
    assert.equal(after.currentVersion, target);
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    await invoke(({ core }) => core.invoke("exit_application"));
  });
});
