import assert from "node:assert/strict";
import { INITIAL_VERSIONS, UPGRADE_TARGET } from "../cli-management-fixture.mjs";
import {
  assertPathsAreFixtureOwned,
  installationOf,
  invoke,
  readFixture,
  snapshotOf,
} from "../helpers/cli-management.mjs";
import { assertNoFatalError, bootDesktopUi } from "../helpers/native-ui.mjs";

/**
 * A freshly launched application against the previous spec's data directory.
 *
 * Everything asserted here came out of SQLite, because the process that wrote it is gone. The
 * point is that the snapshot survived a real restart rather than a re-render, and that the page
 * still shows it before any refresh runs.
 */
globalThis.describe("VaneHub AI desktop CLI management: persistence", () => {
  globalThis.after(async () => {
    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("reads the previous run's snapshot back without probing again", async function () {
    this.timeout(300000);
    const root = await bootDesktopUi();
    const fixture = await readFixture();

    // No refresh: a bounded read is what the page does on open, and it has to answer from storage.
    const snapshots = await invoke("list_cli_environments");
    const claude = snapshotOf(snapshots, "claude-code");
    assertPathsAreFixtureOwned(claude, fixture.root);

    const recommended = installationOf(claude, claude.recommendedInstallationId);
    assert.equal(
      recommended.reportedVersion,
      UPGRADE_TARGET,
      "the version the previous run installed did not survive the restart",
    );
    assert.notEqual(recommended.reportedVersion, INITIAL_VERSIONS.claude);
    assert.ok(claude.checkedAt, "a restored snapshot carried no detection timestamp");

    // The mutation is on the record, so the page can explain what last happened to this tool. The
    // previous spec ran a failing command after the successful one, and the last one is what a
    // user needs to see -- a record that kept the older, happier result would be a lie about the
    // most recent thing that happened to this host.
    assert.ok(claude.lastMutation, "the completed mutation was not persisted");
    assert.equal(claude.lastMutation.outcome, "no-change-failed");
    assert.equal(claude.lastMutation.sourceId, "npm");

    await assertNoFatalError(root);
  });

  globalThis.it("keeps the conflict it found before the restart", async function () {
    this.timeout(300000);
    const snapshots = await invoke("list_cli_environments");
    const codex = snapshotOf(snapshots, "codex-cli");

    assert.ok(codex.conflicts.length > 0, "the stored snapshot lost its conflict");
    assert.notEqual(codex.pathSelectedInstallationId, codex.recommendedInstallationId);
  });

  globalThis.it("renders the restored environment on the CLI Management page", async function () {
    this.timeout(300000);
    const settings = await globalThis.$('//button[@aria-label="设置"]');
    await settings.waitForClickable({ timeout: 30000 });
    await settings.click();

    const page = await globalThis.$('//button[starts-with(normalize-space(.), "CLI 管理")]');
    await page.waitForClickable({ timeout: 30000 });
    await page.click();

    const card = await globalThis.$('[data-cli-agent="claude-code"]');
    await card.waitForExist({ timeout: 30000 });
    // The page shows what the backend restored, not a value it derived on this side.
    await globalThis.browser.waitUntil(async () => (await card.getText()).includes(UPGRADE_TARGET), {
      timeout: 30000,
      timeoutMsg: "The restored version never reached the CLI Management card.",
    });
  });
});
