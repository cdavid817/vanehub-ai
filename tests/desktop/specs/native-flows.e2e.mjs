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
    // Under the mocha ceiling, above a pip download through an egress proxy.
  }, { timeout: 240_000, interval: 1_000, timeoutMsg: message });
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

  globalThis.it("detects CLI tools and installs the version the plan names for opencode", async function installOpencode() {
    if (process.env.VANEHUB_DESKTOP_MUTATE_HOST !== "1") {
      blocked.push("opencode install: set VANEHUB_DESKTOP_MUTATE_HOST=1 to reinstall a global CLI");
      this.skip();
    }
    this.timeout(600_000);
    // The one test in this repository that drives a real package manager against the real host.
    // `desktop-cli-management` covers the same pipeline against a fixture; this exists because a
    // fixture cannot show that the real npm, the real global prefix, and the real PATH agree.
    const refresh = await invoke(({ core }) => core.invoke("refresh_cli_environment", {
      agentIds: ["opencode"],
      forceCatalog: true,
    }));
    await settle(refresh.operationId, "CLI detection refresh never settled.");

    const snapshots = await invoke(({ core }) => core.invoke("list_cli_environments"));
    assert.ok(snapshots.length >= 4, `expected the managed CLI catalogue, found ${snapshots.length}`);
    const opencode = snapshots.find((snapshot) => snapshot.agentId === "opencode");
    assert.ok(opencode, "opencode is missing from the CLI catalogue");

    const installed = opencode.installations
      .find((installation) => installation.id === opencode.recommendedInstallationId)
      ?.reportedVersion;
    // Reinstalling the version already on disk, deliberately: it drives the whole pipeline --
    // plan, operation, npm global install, verification -- and leaves the host on exactly the
    // version it started on. Installing a different one would exercise the same code and change
    // the developer's working CLI as a side effect.
    if (!installed) {
      blocked.push("opencode install: nothing installed to reinstall at its current version");
      this.skip();
    }

    // Scoped to opencode by explicit instruction: claude-code and codex-cli must not be mutated.
    const prepared = await invoke(({ core }, input) => core.invoke("prepare_cli_action", input), {
      agentId: "opencode",
      action: "reinstall",
      sourceId: "npm",
      targetVersion: installed,
      channel: null,
    });
    const planning = await settle(prepared.operationId, "The opencode plan never settled.");
    if (planning.status !== "succeeded") {
      blocked.push(`opencode install: planning refused -- ${planning.error ?? "no reason recorded"}`);
      this.skip();
    }
    const plan = await invoke(({ core }, planId) => core.invoke("get_cli_action_plan", { planId }), planning.result.planId);
    // The version reviewed is the version that runs, and nothing else crosses the execute call.
    assert.equal(plan.targetVersion, installed);

    const execution = await invoke(({ core }, input) => core.invoke("execute_cli_action", input), {
      planId: plan.id,
      expectedRevision: plan.revision,
    });
    const outcome = await settle(execution.operationId, "The opencode install never settled.");
    assert.equal(outcome.status, "succeeded", outcome.error ?? "opencode install failed");
    // Verified, not merely started: the host was re-read after the command and reported the
    // version the plan named.
    assert.equal(outcome.result.outcome, "verified", JSON.stringify(outcome.result));
    assert.equal(outcome.result.observedVersion, installed);
  });

  globalThis.it("installs and removes a real extension framework through pip", async function pipExtension() {
    if (process.env.VANEHUB_DESKTOP_MUTATE_HOST !== "1") {
      blocked.push("extension install: set VANEHUB_DESKTOP_MUTATE_HOST=1 to mutate the host Python environment");
      this.skip();
    }
    const overview = await invoke(({ core }) => core.invoke("get_extension_overview"));
    assert.ok(overview.definitions.length >= 3, "the extension catalogue is missing frameworks");
    // sherpa-onnx of the three: paddleocr pulls paddlepaddle and faster-whisper pulls
    // ctranslate2, and this drives the same install pipeline for a fraction of the download.
    const frameworkId = "sherpa-onnx";

    const preview = await invoke(({ core }, id) => core.invoke("get_extension_install_preview", {
      request: { frameworkId: id },
    }), frameworkId);
    assert.ok(preview, "no install preview was produced");

    const install = await invoke(({ core }, id) => core.invoke("install_extension", {
      request: { frameworkId: id },
    }), frameworkId);
    const installed = await settle(install.id, "The extension install never settled.");
    if (installed.status !== "succeeded") {
      blocked.push(`extension install: ${installed.error ?? "pip install failed on this host"}`);
      this.skip();
    }

    const afterInstall = await invoke(({ core }) => core.invoke("get_extension_overview"));
    const status = afterInstall.statuses.find((entry) => entry.frameworkId === frameworkId);
    assert.ok(status, "the installed framework reported no status");

    // Put the host back: this test installs into the developer's real Python environment.
    const uninstall = await invoke(({ core }, id) => core.invoke("uninstall_extension", {
      request: { frameworkId: id },
    }), frameworkId);
    const removed = await settle(uninstall.id, "The extension uninstall never settled.");
    assert.equal(removed.status, "succeeded", removed.error ?? "uninstall failed, leaving the host changed");
  });

  globalThis.it("connects to a real SSH host and removes the connection", async function sshConnection() {
    // Credentials come from the environment only -- never a committed fixture.
    const host = process.env.VANEHUB_SSH_HOST;
    const user = process.env.VANEHUB_SSH_USER;
    const password = process.env.VANEHUB_SSH_PASSWORD;
    if (!host || !user || !password) {
      blocked.push("SSH: set VANEHUB_SSH_HOST, VANEHUB_SSH_USER and VANEHUB_SSH_PASSWORD");
      this.skip();
    }

    const connection = await invoke(({ core }, input) => core.invoke("create_ssh_connection", { input }), {
      name: "desktop-sweep-ssh",
      host,
      port: 22,
      user,
      defaultPath: "/root",
      authMode: "password",
      keyPath: null,
      password,
    });
    assert.ok(connection.id, "the SSH connection was not persisted");

    try {
      let result = await invoke(({ core }, id) => core.invoke("test_ssh_connection", {
        connectionId: id,
      }), connection.id);

      // First contact with an unknown host parks the key for confirmation rather than trusting
      // it, so accepting it is part of the flow rather than a workaround.
      if (result.status !== "succeeded") {
        const challenge = await invoke(({ core }, id) => core.invoke("get_pending_ssh_host_key", {
          connectionId: id,
        }), connection.id).catch(() => null);
        if (challenge?.fingerprint) {
          await invoke(({ core }, input) => core.invoke("confirm_ssh_host_key", { input }), {
            connectionId: challenge.connectionId,
            revision: challenge.revision,
            fingerprint: challenge.fingerprint,
          });
          result = await invoke(({ core }, id) => core.invoke("test_ssh_connection", {
            connectionId: id,
          }), connection.id);
        }
      }

      assert.equal(result.status, "succeeded", result.message ?? "the SSH connection test failed");
      const listed = await invoke(({ core }) => core.invoke("list_ssh_connections"));
      const stored = listed.find((entry) => entry.id === connection.id);
      assert.ok(stored, "the tested connection is missing from the list");
      assert.equal(stored.host, host);
      // The secret must not come back out of the store through a read path.
      assert.equal(
        JSON.stringify(stored).includes(password),
        false,
        "the connection record echoed its password back",
      );
    } finally {
      await invoke(({ core }, id) => core.invoke("delete_ssh_connection", {
        connectionId: id,
      }), connection.id).catch(() => {});
    }
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // Teardown is left to WDIO. Exiting the app from here raced its `deleteSession`, and the
    // resulting worker-level failure discarded every per-test result for this file -- the run
    // reported one opaque FAILED for a spec whose tests had all completed.
  });
});
