import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { access, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";
import { comparableFilesystemPath } from "../helpers/filesystem-path.mjs";
import { assertNoFatalError, bootDesktopUi, createSessionButton, dialog, dialogButton } from "../helpers/native-ui.mjs";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);

/**
 * The confirmed session-deletion flow against the real desktop client and the host's real Git.
 *
 * The Web/mock Playwright spec proves the interaction contract with a simulated backend. This
 * layer proves the part no simulation can: after the user ticks "also remove the worktree" and
 * confirms, the directory is gone, Git no longer lists it, the branch is still there, and the
 * main checkout is untouched — and that without the tick, or with uncommitted work, nothing on
 * disk changes at all.
 *
 * Every repository here is a temporary one the spec creates; nothing touches a real project.
 */
// Per attempt, not per file: a retried spec must not find the previous attempt's sessions by title.
let stamp = "";
const DIALOG = '//*[@role="dialog"]';
const testId = (id) => globalThis.$(`${DIALOG}//*[@data-testid="${id}"]`);

async function createRepository() {
  // Outside the run root on purpose: a CLI session roots a PTY in the checkout and the OS releases
  // that directory a moment after the child dies, which loses the race with run-root disposal.
  const root = await mkdtemp(join(tmpdir(), "vanehub-deletion-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function gitWorktreePaths(root) {
  const { stdout } = await run("git", ["worktree", "list", "--porcelain"], { cwd: root });
  return stdout.split(/\r?\n/)
    .filter((line) => line.startsWith("worktree "))
    .map((line) => comparableFilesystemPath(line.slice("worktree ".length)));
}

async function gitBranches(root) {
  const { stdout } = await run("git", ["branch", "--list", "--format=%(refname:short)"], { cwd: root });
  return stdout.split(/\r?\n/).filter(Boolean);
}

async function exists(path) {
  return access(path).then(() => true, () => false);
}

/**
 * Creates a session the way a person does: through the create dialog, on the fixture `opencode`.
 *
 * Through the dialog rather than the command so the sidebar shows the session the way it will for
 * a user, and on the fixture Agent by name: the first "available" CLI Agent on a developer machine
 * is a real one, and a real one would start a real model session in the worktree under test.
 */
async function createSessionThroughDialog({ repository, title, worktreeName }) {
  const opener = await createSessionButton();
  await opener.waitForClickable({ timeout: 30_000 });
  await opener.click();
  const opened = await dialog();
  await opened.waitForExist({ timeout: 20_000 });

  const agent = await globalThis.$(`${DIALOG}//button[contains(., "opencode")]`);
  await agent.waitForClickable({ timeout: 20_000 });
  await agent.click();

  const path = await globalThis.$(`${DIALOG}//input[contains(@placeholder, "code")]`);
  await path.waitForEnabled({ timeout: 20_000 });
  await path.setValue(repository);
  // The project is inspected on blur, and the worktree option only unlocks once the inspection
  // says Git. Moving focus into the title field is the blur a person would cause.
  const titleField = await globalThis.$(`${DIALOG}//input[@placeholder="新会话"]`);
  await titleField.click();
  await titleField.setValue(title);
  const inspected = await globalThis.$(`${DIALOG}//p[contains(., "Git 项目")]`);
  await inspected.waitForExist({ timeout: 30_000 });

  if (worktreeName) {
    const checkbox = await globalThis.$(`${DIALOG}//label[contains(., "创建新 Git worktree")]//input[@type="checkbox"]`);
    await checkbox.waitForEnabled({ timeout: 30_000 });
    await checkbox.click();
    const nameField = await globalThis.$(`${DIALOG}//input[@placeholder="feature-a"]`);
    await nameField.waitForDisplayed({ timeout: 10_000 });
    await nameField.setValue(worktreeName);
  }
  const create = await dialogButton("创建");
  await globalThis.browser.waitUntil(async () => create.isEnabled(), {
    timeout: 20_000,
    timeoutMsg: "The create action never became available.",
  });
  await create.click();
  await opened.waitForExist({ timeout: 60_000, reverse: true });

  const session = await globalThis.browser.waitUntil(async () => {
    const listed = await invoke(({ core }) => core.invoke("list_sessions"));
    return listed.find((entry) => entry.title === title) ?? false;
  }, { timeout: 90_000, timeoutMsg: `Session ${title} was not created through the dialog.` });
  assert.equal(session.agentId, "opencode", "the session was not created on the fixture Agent");
  return session;
}

async function sessionListed(title) {
  const listed = await invoke(({ core }) => core.invoke("list_sessions"));
  return listed.some((entry) => entry.title === title);
}

/** Opens the unified deletion dialog from the sidebar card's context menu. */
async function openDeleteDialog(sessionId) {
  await createSessionButton();
  const card = await globalThis.$(`[data-testid="session-sidebar"] [data-session-id="${sessionId}"]`);
  await card.waitForDisplayed({ timeout: 30_000 });
  // The menu is opened by the card's `contextmenu` handler. A synthesized event rather than a
  // pointer right-click: WebKitGTK's right-click through WebDriver is not reliable on every host,
  // and the entry point under test is the handler, not the pointer.
  await globalThis.browser.execute((element) => {
    const rect = element.getBoundingClientRect();
    element.dispatchEvent(new globalThis.MouseEvent("contextmenu", {
      bubbles: true,
      cancelable: true,
      clientX: rect.left + rect.width / 2,
      clientY: rect.top + rect.height / 2,
    }));
  }, card);
  const remove = await globalThis.$('//*[@aria-label="会话操作"]//button[normalize-space(.)="删除"]');
  await remove.waitForClickable({ timeout: 20_000 });
  await remove.click();
  const opened = await dialog();
  await opened.waitForExist({ timeout: 20_000 });
  await globalThis.browser.waitUntil(
    async () => (await (await testId("session-deletion-dialog")).getAttribute("data-status")) !== "loading",
    { timeout: 60_000, timeoutMsg: "The deletion preview never finished checking." },
  );
  return opened;
}

/**
 * Re-queried on every poll rather than held: the result panel is one element while the operation
 * runs and a different one once it settles, and a reference taken during the run goes stale at
 * exactly the moment the outcome appears. With the driver embedded in the page, that stale
 * reference surfaces as a window error the fatal-error check would then blame on the product.
 */
function readResult() {
  return globalThis.browser.execute(() => {
    const node = globalThis.document.querySelector('[role="dialog"] [data-testid="session-deletion-result"]');
    return node ? { outcome: node.getAttribute("data-outcome"), text: node.textContent ?? "" } : null;
  });
}

async function waitForOutcome(expected) {
  let latest = null;
  await globalThis.browser.waitUntil(async () => {
    latest = await readResult();
    return latest?.outcome === expected;
  }, { timeout: 120_000, timeoutMsg: `The deletion did not reach outcome ${expected}.` });
  return latest;
}

globalThis.describe("VaneHub AI desktop session deletion", () => {
  let root;
  let repository;
  let worktreeName;
  let worktreeTitle;
  let worktreeSession;

  globalThis.before(async function () {
    this.timeout(300_000);
    stamp = Date.now().toString(36);
    worktreeName = `feature-${stamp}`;
    worktreeTitle = `删除验证-工作树-${stamp}`;
    root = await bootDesktopUi();
    repository = await createRepository();
    worktreeSession = await createSessionThroughDialog({ repository, title: worktreeTitle, worktreeName });
    assert.ok(worktreeSession.worktreePath, "the worktree session reported no worktree path");
    assert.ok(await exists(worktreeSession.worktreePath), "the worktree directory was not created");
  });

  globalThis.after(async () => {
    await invoke(({ core }) => core.invoke("exit_application"));
  });

  globalThis.it("keeps the worktree by default and deletes nothing on cancel", async function () {
    this.timeout(180_000);
    await openDeleteDialog(worktreeSession.id);

    const remove = await testId("session-deletion-remove-worktree");
    assert.equal(await remove.isSelected(), false, "worktree removal was pre-selected");
    assert.equal(await remove.isEnabled(), true, "a clean worktree should be removable");
    const path = await testId("session-deletion-worktree-path");
    assert.equal(
      comparableFilesystemPath(await path.getText()),
      comparableFilesystemPath(worktreeSession.worktreePath),
      "the dialog shows a different directory than the session's worktree",
    );
    const confirm = await testId("session-deletion-confirm");
    assert.equal(await confirm.getText(), "仅删除会话");

    // The destructive choice is reflected in the confirm label and nowhere else yet.
    await remove.click();
    assert.equal(await confirm.getText(), "删除会话及 worktree");

    await (await testId("session-deletion-cancel")).click();
    await (await dialog()).waitForExist({ timeout: 10_000, reverse: true });
    assert.ok(await sessionListed(worktreeTitle), "cancel deleted the session");
    assert.ok(await exists(worktreeSession.worktreePath), "cancel removed the directory");
    assert.ok(
      (await gitWorktreePaths(repository)).includes(comparableFilesystemPath(worktreeSession.worktreePath)),
      "cancel unregistered the worktree",
    );
    await assertNoFatalError(root);
  });

  globalThis.it("refuses to clean up a worktree with untracked work and says why", async function () {
    this.timeout(180_000);
    const untracked = join(worktreeSession.worktreePath, "unsaved-work.txt");
    await writeFile(untracked, "not committed\n", "utf8");
    try {
      await openDeleteDialog(worktreeSession.id);
      const remove = await testId("session-deletion-remove-worktree");
      assert.equal(await remove.isEnabled(), false, "a dirty worktree was offered for removal");
      const blockers = await testId("session-deletion-worktree-blockers");
      assert.match(await blockers.getText(), /存在未跟踪的文件/);
      assert.match(await (await testId("session-deletion-worktree-status")).getText(), /有未提交修改/);
      assert.equal(await (await testId("session-deletion-confirm")).getText(), "仅删除会话");
      await (await testId("session-deletion-cancel")).click();
      await (await dialog()).waitForExist({ timeout: 10_000, reverse: true });
      assert.ok(await exists(untracked), "the untracked file was touched");
    } finally {
      await rm(untracked, { force: true });
    }
    await assertNoFatalError(root);
  });

  globalThis.it("removes the worktree through Git when explicitly chosen, and keeps the branch", async function () {
    this.timeout(300_000);
    const branchesBefore = await gitBranches(repository);
    assert.ok(branchesBefore.includes(`vanehub/${worktreeName}`), "the fixture branch is missing");

    await openDeleteDialog(worktreeSession.id);
    const remove = await testId("session-deletion-remove-worktree");
    assert.equal(await remove.isEnabled(), true, "the clean worktree should be removable again");
    await remove.click();
    const confirm = await testId("session-deletion-confirm");
    assert.equal(await confirm.getText(), "删除会话及 worktree");
    await confirm.click();

    const result = await waitForOutcome("succeeded");
    const text = result.text;
    assert.match(text, /已移除/, `the result did not report the directory as removed: ${text}`);
    assert.doesNotMatch(text, /模拟/, "a native run reported itself as simulated");
    await (await testId("session-deletion-cancel")).click();
    await (await dialog()).waitForExist({ timeout: 10_000, reverse: true });

    // What actually happened on disk, read from the filesystem and from Git, not from the DTO.
    assert.equal(await exists(worktreeSession.worktreePath), false, "the worktree directory still exists");
    assert.equal(
      (await gitWorktreePaths(repository)).includes(comparableFilesystemPath(worktreeSession.worktreePath)),
      false,
      "Git still lists the removed worktree",
    );
    const branchesAfter = await gitBranches(repository);
    assert.ok(branchesAfter.includes(`vanehub/${worktreeName}`), "the branch was deleted along with the worktree");
    assert.ok(await exists(join(repository, "seed.txt")), "the main checkout lost a tracked file");
    await globalThis.browser.waitUntil(async () => !(await sessionListed(worktreeTitle)), {
      timeout: 30_000,
      timeoutMsg: "The deleted session is still listed.",
    });
    await assertNoFatalError(root);
  });

  globalThis.it("deletes a project session without offering or touching its directory", async function () {
    this.timeout(180_000);
    const title = `删除验证-项目-${stamp}`;
    const session = await createSessionThroughDialog({ repository, title, worktreeName: null });

    await openDeleteDialog(session.id);
    assert.equal(await (await testId("session-deletion-remove-worktree")).isExisting(), false, "a project session was offered a worktree removal");
    assert.match(await (await testId("session-deletion-project-note")).getText(), /项目目录及其中的文件不会被删除/);
    const confirm = await testId("session-deletion-confirm");
    assert.equal(await confirm.getText(), "仅删除会话");
    await confirm.click();

    await waitForOutcome("succeeded");
    await (await testId("session-deletion-cancel")).click();
    await (await dialog()).waitForExist({ timeout: 10_000, reverse: true });

    assert.ok(await exists(join(repository, "seed.txt")), "deleting a project session touched the project");
    await globalThis.browser.waitUntil(async () => !(await sessionListed(title)), {
      timeout: 30_000,
      timeoutMsg: "The deleted project session is still listed.",
    });
    await assertNoFatalError(root);
  });
});
