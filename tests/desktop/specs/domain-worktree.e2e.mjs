import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

const stamp = Date.now().toString(36);
const sessions = [];
let repository = null;
let plainFolder = null;

/**
 * Git worktrees: a session that gets its own checkout so parallel work does not collide.
 *
 * Worktrees have no command surface of their own -- they are a field on `create_session`
 * (sessions/dto.rs:277-280) -- so this drives them the only way the product does, and then checks
 * the result against git itself rather than against what the session record claims. A session row
 * saying it has a worktree while `git worktree list` disagrees is exactly the failure this is for,
 * and no assertion made purely against the session DTO would catch it.
 *
 * The fixture lives outside the harness run root on purpose: a CLI session mounts a terminal tab
 * that roots a PTY in the checkout, and the OS releases that directory a moment after the child
 * dies rather than synchronously with it, so a fixture inside the run root loses the race with
 * `disposeRunContext`.
 */
async function createRepository() {
  const root = await mkdtemp(join(tmpdir(), "vanehub-worktree-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

async function settleOperation(operation, timeoutMsg) {
  return globalThis.browser.waitUntil(async () => {
    const status = await invoke(
      ({ core }, operationId) => core.invoke("get_operation_status", { operationId }),
      operation.id,
    );
    return ["succeeded", "failed", "cancelled"].includes(status.status) ? status : false;
  }, { timeout: 90_000, timeoutMsg });
}

async function createSession(input) {
  return attempt("create_session", {
    input: {
      remoteWorkspace: null,
      worktree: null,
      folder: repository,
      projectPath: repository,
      interactionMode: "cli",
      ...input,
    },
  });
}

/** Reads the worktree inventory from git rather than from the session record. */
async function gitWorktrees(root) {
  const { stdout } = await run("git", ["worktree", "list", "--porcelain"], { cwd: root });
  return stdout;
}

globalThis.describe("VaneHub AI desktop Git worktree sessions", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
    plainFolder = await mkdtemp(join(tmpdir(), "vanehub-plain-"));
  });

  globalThis.it("creates a real Git worktree for a session that asks for one", async function createWorktree() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((entry) => entry.availabilityState === "available"
      && entry.supportedInteractionModes.includes("cli"));
    if (!agent) {
      blocked.push("worktree session: no installed CLI Agent to create the session with");
      this.skip();
    }

    const name = `wt-${stamp}`;
    const created = await createSession({
      agentId: agent.id,
      title: `worktree-session-${stamp}`,
      worktree: { enabled: true, name },
    });
    assert.equal(created.ok, true, `creating a worktree session failed: ${created.error}`);
    const settled = await settleOperation(created.value, "Creating the worktree session never settled.");
    assert.equal(settled.status, "succeeded", settled.error ?? "worktree session creation failed");

    const session = await globalThis.browser.waitUntil(async () => {
      const listed = await invoke(({ core }) => core.invoke("list_sessions"));
      return listed.find((entry) => entry.title === `worktree-session-${stamp}`) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The worktree session was not listed." });
    sessions.push(session.id);

    assert.ok(session.worktreePath, "the session reported no worktree path");
    assert.equal(session.worktreeName, name, "the session reported a different worktree name");
    // workspaces/domain/worktree.rs:53-55 -- the branch is always namespaced under `vanehub/`,
    // which is what keeps these off the names a person would pick by hand.
    assert.equal(session.worktreeBranch, `vanehub/${name}`, "the worktree branch was not namespaced");

    // The assertion that matters: git agrees. `--porcelain` rather than the human listing, because
    // the human one is a formatted table whose columns move, and a substring match against it was
    // how an earlier round of this suite reported a defect that did not exist.
    const listing = await gitWorktrees(repository);
    assert.ok(
      listing.includes(`worktree ${session.worktreePath}`),
      `git does not list the worktree the session claims: ${listing}`,
    );
    assert.ok(
      listing.includes(`branch refs/heads/vanehub/${name}`),
      `git does not list the namespaced branch: ${listing}`,
    );
  });

  globalThis.it("refuses a worktree name git could not carry", async function rejectNames() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((entry) => entry.availabilityState === "available"
      && entry.supportedInteractionModes.includes("cli"));
    if (!agent) {
      blocked.push("worktree name validation: no installed CLI Agent to create the session with");
      this.skip();
    }

    // workspaces/domain/worktree.rs:33-46 -- a name is rejected when it is blank, carries a path
    // separator, walks up with `..`, or contains a control character. Each of those would either
    // escape the repository or produce a ref git cannot name.
    const rejected = ["", "   ", "nested/name", "nested\\name", "..", "up/../out"];
    for (const name of rejected) {
      const refused = await createSession({
        agentId: agent.id,
        title: `worktree-reject-${stamp}-${rejected.indexOf(name)}`,
        worktree: { enabled: true, name },
      });
      // The refusal can land either as a rejected command or as a failed operation, depending on
      // how far the request gets before the name is parsed; both are a refusal, and asserting only
      // one of them would make this pass for the wrong reason.
      if (refused.ok) {
        const settled = await settleOperation(refused.value, `The refused worktree name ${JSON.stringify(name)} never settled.`);
        assert.notEqual(
          settled.status,
          "succeeded",
          `a session was created with the invalid worktree name ${JSON.stringify(name)}`,
        );
      }
    }

    const listing = await gitWorktrees(repository);
    assert.equal(
      listing.includes("vanehub/nested"),
      false,
      "a rejected worktree name still produced a branch",
    );
  });

  globalThis.it("refuses a worktree on a folder that is not a Git repository", async function plainFolderCase() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((entry) => entry.availabilityState === "available"
      && entry.supportedInteractionModes.includes("cli"));
    if (!agent) {
      blocked.push("non-Git worktree guard: no installed CLI Agent to create the session with");
      this.skip();
    }

    // The create dialog disables the worktree checkbox until the path inspects as a Git project,
    // but the command is reachable regardless of what the dialog allows, so the guard has to hold
    // on this side too.
    const refused = await attempt("create_session", {
      input: {
        agentId: agent.id,
        interactionMode: "cli",
        title: `worktree-plain-${stamp}`,
        folder: plainFolder,
        projectPath: plainFolder,
        remoteWorkspace: null,
        worktree: { enabled: true, name: `plain-${stamp}` },
      },
    });
    if (refused.ok) {
      const settled = await settleOperation(refused.value, "The non-Git worktree request never settled.");
      assert.notEqual(
        settled.status,
        "succeeded",
        "a worktree session was created against a folder that is not a Git repository",
      );
    }
  });

  globalThis.it("leaves a session without a worktree when it does not ask for one", async function noWorktree() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agent = agents.find((entry) => entry.availabilityState === "available"
      && entry.supportedInteractionModes.includes("cli"));
    if (!agent) {
      blocked.push("plain session control: no installed CLI Agent to create the session with");
      this.skip();
    }

    // The control case. Without it, a build that created a worktree for every session would pass
    // the case above and nothing here would notice.
    const created = await createSession({
      agentId: agent.id,
      title: `plain-session-${stamp}`,
      worktree: { enabled: false, name: null },
    });
    assert.equal(created.ok, true, `creating a plain session failed: ${created.error}`);
    await settleOperation(created.value, "Creating the plain session never settled.");

    const session = await globalThis.browser.waitUntil(async () => {
      const listed = await invoke(({ core }) => core.invoke("list_sessions"));
      return listed.find((entry) => entry.title === `plain-session-${stamp}`) ?? false;
    }, { timeout: 30_000, timeoutMsg: "The plain session was not listed." });
    sessions.push(session.id);

    assert.equal(session.worktreePath, null, "a session that asked for no worktree reported one");
    assert.equal(session.worktreeName, null, "a session that asked for no worktree reported a name");
    assert.equal(session.worktreeBranch, null, "a session that asked for no worktree reported a branch");
  });

  globalThis.after(async () => {
    for (const sessionId of sessions) {
      try {
        await invoke(({ core }, id) => core.invoke("delete_session", { sessionId: id }), sessionId);
      } catch (error) {
        globalThis.console.warn(`Cleanup step "delete session ${sessionId}" failed: ${error}`);
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
