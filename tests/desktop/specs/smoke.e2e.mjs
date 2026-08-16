import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { readNativeSettings } from "../helpers/native-settings.mjs";

const run = promisify(execFile);

globalThis.describe("VaneHub AI native desktop smoke", () => {
  globalThis.it("starts the real runtime, crosses IPC, and performs stable navigation", async () => {
    const repository = await mkdtemp(join(tmpdir(), "vanehub-review-e2e-"));
    await run("git", ["init"], { cwd: repository });
    await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: repository });
    await run("git", ["config", "user.name", "Desktop E2E"], { cwd: repository });
    await writeFile(join(repository, "review.txt"), "before\n", "utf8");
    await run("git", ["add", "review.txt"], { cwd: repository });
    await run("git", ["commit", "-m", "fixture"], { cwd: repository });
    await writeFile(join(repository, "review.txt"), "after\n", "utf8");
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready", {
      timeout: 120_000,
      timeoutMsg: "React bootstrap did not become ready.",
    });

    const settings = await readNativeSettings();
    assert.match(settings.applicationLanguage, /^(zh-CN|zh-TW|en|ja|ko)$/);
    const update = await globalThis.browser.tauri.execute(({ core }) => (
      core.invoke("get_desktop_update_snapshot")
    ));
    assert.equal(update.phase, "idle");
    assert.match(update.currentVersion, /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/);
    assert.match(update.channel, /^(stable|preview)$/);

    await globalThis.browser.tauri.execute(({ core }, projectPath) => core.invoke("create_session", {
      input: {
        agentId: "codex-cli",
        interactionMode: "cli",
        title: "Desktop code review fixture",
        folder: projectPath,
        projectPath,
        remoteWorkspace: null,
        worktree: null,
      },
    }), repository);
    const session = await globalThis.browser.waitUntil(async () => {
      const sessions = await globalThis.browser.tauri.execute(({ core }) => core.invoke("list_sessions"));
      return sessions.find((item) => item.title === "Desktop code review fixture") ?? false;
    }, { timeout: 30_000, timeoutMsg: "Desktop review session was not created." });
    const review = await globalThis.browser.tauri.execute(
      ({ core }, sessionId) => core.invoke("open_code_review", { sessionId }),
      session.id,
    );
    assert.equal(review.files.length, 1);
    const diff = await globalThis.browser.tauri.execute(
      ({ core }, input) => core.invoke("load_code_review_file", input),
      { sessionId: session.id, path: "review.txt", expectedSnapshot: review.fingerprint },
    );
    assert.equal(diff.hunks.length, 1);
    await writeFile(join(repository, "review.txt"), "external edit\n", "utf8");
    await assert.rejects(() => globalThis.browser.tauri.execute(
      ({ core }, input) => core.invoke("revert_code_review_change", { input }),
      { sessionId: session.id, path: "review.txt", expectedSnapshot: review.fingerprint, hunkFingerprint: diff.hunks[0].fingerprint, confirmed: true },
    ));

    const settingsButton = await globalThis.$('[data-testid="desktop-smoke-settings"]');
    await settingsButton.waitForClickable();
    await settingsButton.click();
    await globalThis.browser.waitUntil(async () => (await globalThis.browser.getUrl()).includes("/settings"), {
      timeoutMsg: "The native WebView did not navigate to settings.",
    });
    const settingsRoot = await globalThis.$("main");
    await settingsRoot.waitForExist();
    assert.equal(await root.getAttribute("data-vanehub-fatal-error"), null);

    await globalThis.browser.tauri.execute(({ core }) => core.invoke("exit_application"));
    await rm(repository, { recursive: true, force: true });
  });
});
