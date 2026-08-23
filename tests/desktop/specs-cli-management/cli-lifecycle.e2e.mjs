import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import process from "node:process";
import { auditCliSideEffects, describeCliSideEffects } from "../../../scripts/desktop/cli-side-effect-guard.mjs";
import { FIXTURE_MARKER, INITIAL_VERSIONS, UPGRADE_TARGET } from "../cli-management-fixture.mjs";
import {
  assertPathsAreFixtureOwned,
  awaitOperation,
  installationOf,
  invoke,
  readFixture,
  readInvocations,
  refreshEnvironments,
  snapshotOf,
} from "../helpers/cli-management.mjs";
import { assertNoFatalError, bootDesktopUi } from "../helpers/native-ui.mjs";

/**
 * The CLI lifecycle, over real Tauri IPC, against a fixture PATH.
 *
 * Every command here is the one the frontend calls. The Web/mock adapter is not involved: it
 * answers from invented data, which can prove a UI renders and can prove nothing about discovery,
 * probing, planning, process execution, verification, or SQLite.
 */
globalThis.describe("VaneHub AI desktop CLI management: lifecycle", () => {
  let fixture;
  let planId;
  let planRevision;

  globalThis.before(async () => {
    fixture = await readFixture();
  });

  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("discovers the fixture installations and nothing from the host", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();

    const snapshots = await refreshEnvironments();
    assert.deepEqual(
      snapshots.map((snapshot) => snapshot.agentId),
      ["claude-code", "codex-cli", "gemini-cli", "opencode", "antigravity-cli"],
    );

    const claude = snapshotOf(snapshots, "claude-code");
    assertPathsAreFixtureOwned(claude, fixture.root);
    assert.equal(claude.discovery, "found-one");
    assert.equal(claude.executable, "healthy");
    // The version came from running the binary, not from a stored guess.
    const recommended = installationOf(claude, claude.recommendedInstallationId);
    assert.equal(recommended.reportedVersion, INITIAL_VERSIONS.claude);
    assert.equal(recommended.environmentOrigin, "path");

    await assertNoFatalError(root);
  });

  globalThis.it("groups one npm install's launchers into a single installation", async function () {
    this.timeout(300000);
    const snapshots = await invoke("list_cli_environments");
    const claude = snapshotOf(snapshots, "claude-code");

    // On Windows the fixture writes `claude`, `claude.cmd`, and `claude.ps1` side by side, exactly
    // as one npm global install does. Three installations here would be three competing copies of
    // something that is one thing.
    assert.equal(claude.installations.length, 1, JSON.stringify(claude.installations, null, 2));
    if (process.platform === "win32") {
      assert.ok(claude.installations[0].aliasPaths.length >= 1, "the launcher family was not folded in");
    }
  });

  globalThis.it("reports a broken launcher ahead of a healthy one as a conflict", async function () {
    this.timeout(300000);
    const snapshots = await invoke("list_cli_environments");
    const codex = snapshotOf(snapshots, "codex-cli");

    assertPathsAreFixtureOwned(codex, fixture.root);
    assert.ok(codex.installations.length >= 2, JSON.stringify(codex.installations, null, 2));
    // PATH reaches the broken launcher first; the backend recommends the runnable one and says so.
    assert.notEqual(codex.pathSelectedInstallationId, codex.recommendedInstallationId);
    assert.ok(codex.conflicts.length > 0, "a shadowed broken launcher produced no conflict");
    assert.ok(
      codex.conflicts.some((conflict) => conflict.reasonCode.length > 0 && conflict.severity.length > 0),
      "a conflict arrived without a stable reason code",
    );
  });

  globalThis.it("prepares a plan that names the exact selected version", async function () {
    this.timeout(300000);
    const snapshots = await invoke("list_cli_environments");
    const claude = snapshotOf(snapshots, "claude-code");
    const source = claude.sources.find((entry) => entry.sourceId === "npm");
    assert.ok(source, "the npm source was not summarized");

    const handle = await invoke("prepare_cli_action", {
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: UPGRADE_TARGET,
      channel: null,
    });
    const prepared = await awaitOperation(handle.operationId);
    assert.equal(prepared.status, "succeeded", prepared.error ?? "");
    planId = prepared.result.planId;
    assert.ok(planId, `no plan id on ${JSON.stringify(prepared.result)}`);

    const plan = await invoke("get_cli_action_plan", { planId });
    planRevision = plan.revision;
    assert.equal(plan.targetVersion, UPGRADE_TARGET);
    assert.equal(plan.currentVersion, INITIAL_VERSIONS.claude);
    assert.equal(plan.sourceId, "npm");
    assert.equal(plan.state, "draft");
    // Structured argv carrying the chosen version, not a shell string and not `latest`.
    assert.ok(plan.commandPreview.program.startsWith("npm"), plan.commandPreview.program);
    assert.ok(
      plan.commandPreview.args.some((argument) => argument.endsWith(`@${UPGRADE_TARGET}`)),
      JSON.stringify(plan.commandPreview),
    );
  });

  globalThis.it("executes the reviewed plan, changes the host, and verifies it", async function () {
    this.timeout(300000);
    const handle = await invoke("execute_cli_action", { planId, expectedRevision: planRevision });
    const executed = await awaitOperation(handle.operationId);

    assert.equal(executed.status, "succeeded", executed.error ?? "");
    assert.equal(executed.result.outcome, "verified", JSON.stringify(executed.result));
    assert.equal(executed.result.targetVersion, UPGRADE_TARGET);
    assert.equal(executed.result.observedVersion, UPGRADE_TARGET);
    assert.equal(executed.result.termination, "exited");

    // The fake package manager actually rewrote what the fake CLI reports, so verification was an
    // observation rather than an assumption.
    assert.equal((await readFile(fixture.versionFiles.claude, "utf8")).trim(), UPGRADE_TARGET);
    const invocations = await readInvocations(fixture);
    const install = invocations.find((entry) => entry.tool === "npm" && entry.argv.includes("install"));
    assert.ok(install, "the package manager was never invoked");
    assert.ok(
      install.argv.some((argument) => argument.endsWith(`@${UPGRADE_TARGET}`)),
      JSON.stringify(install.argv),
    );
  });

  globalThis.it("refuses to run the same plan twice", async function () {
    this.timeout(300000);
    // Single use is the property that makes "the version reviewed is the version that runs" hold
    // across a retry: a consumed plan cannot be replayed after the environment moved.
    const consumed = await invoke("get_cli_action_plan", { planId });
    assert.notEqual(consumed.state, "draft", JSON.stringify(consumed));

    let refused = null;
    try {
      await invoke("execute_cli_action", { planId, expectedRevision: planRevision });
    } catch (error) {
      refused = error;
    }
    assert.ok(refused, "a consumed plan was accepted for execution");
  });

  globalThis.it("reports a failing command as failed without claiming a rollback", async function () {
    this.timeout(300000);
    // The fixture's package manager refuses this version, so the command fails after the host was
    // already left where the previous run put it.
    const handle = await invoke("prepare_cli_action", {
      agentId: "claude-code",
      action: null,
      sourceId: "npm",
      targetVersion: "9.9.9-fails",
      channel: null,
    });
    const prepared = await awaitOperation(handle.operationId);
    if (prepared.status !== "succeeded") {
      // A source that refuses the version at planning time is also a correct answer; what must
      // never happen is a plan that runs and reports success.
      assert.ok(prepared.error, "planning failed without an error");
      return;
    }
    const failing = prepared.result.planId;
    const plan = await invoke("get_cli_action_plan", { planId: failing });
    const executed = await awaitOperation(
      (await invoke("execute_cli_action", { planId: failing, expectedRevision: plan.revision })).operationId,
    );

    assert.equal(executed.status, "failed");
    assert.ok(
      ["no-change-failed", "changed-but-failed"].includes(executed.result.outcome),
      JSON.stringify(executed.result),
    );
    // The previous upgrade stands. Nothing restored it, and nothing claimed to.
    assert.equal((await readFile(fixture.versionFiles.claude, "utf8")).trim(), UPGRADE_TARGET);
  });

  globalThis.it("previews a batch with a reason for everything it will not run", async function () {
    this.timeout(300000);
    const handle = await invoke("prepare_cli_bulk_action", {
      agentIds: ["claude-code", "codex-cli", "gemini-cli", "opencode", "antigravity-cli"],
    });
    const prepared = await awaitOperation(handle.operationId);
    assert.equal(prepared.status, "succeeded", prepared.error ?? "");

    const bulk = await invoke("get_cli_bulk_action_plan", { planId: prepared.result.planId });
    assert.equal(bulk.items.length + bulk.skipped.length, 5, JSON.stringify(bulk, null, 2));
    for (const skip of bulk.skipped) {
      assert.ok(skip.reason.length > 0, `${skip.agentId} was skipped without a reason`);
    }
  });

  globalThis.it("cancels a refresh through the operation service", async function () {
    this.timeout(300000);
    const handle = await invoke("refresh_cli_environment", { agentIds: [], forceCatalog: true });
    await invoke("cancel_operation", { operationId: handle.operationId });
    const settled = await awaitOperation(handle.operationId);

    assert.ok(["cancelled", "succeeded", "failed"].includes(settled.status), settled.status);
    // Cancelling never claims an already-applied external effect was undone.
    if (settled.result?.outcome) {
      assert.notEqual(settled.result.outcome, "verified");
    }
  });

  globalThis.it("touched nothing real", async function () {
    this.timeout(120000);
    const invocations = await readInvocations(fixture);
    const snapshots = await invoke("list_cli_environments");
    const previews = [];
    for (const snapshot of snapshots) {
      for (const conflict of snapshot.conflicts) assert.ok(conflict.reasonCode);
    }
    const violations = auditCliSideEffects({
      marker: FIXTURE_MARKER,
      invocations,
      commandPreviews: previews,
      fixtureRoot: fixture.root,
      dataDir: process.env.VANEHUB_APP_DATA_DIR,
      userDataDir: process.env.APPDATA ? `${process.env.APPDATA}\\ai.vanehub.app` : null,
      environment: process.env,
    });
    assert.equal(describeCliSideEffects(violations), null);
  });
});
