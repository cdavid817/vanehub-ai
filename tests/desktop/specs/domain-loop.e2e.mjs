import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdir, mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { promisify } from "node:util";

const run = promisify(execFile);
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

function normalizedPath(value) {
  return value.replace(/^\\\\\?\\/, "").replaceAll("\\", "/").toLowerCase();
}

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

const fixtureRoot = process.env.VANEHUB_APP_DATA_DIR
  ? join(dirname(process.env.VANEHUB_APP_DATA_DIR), "fixtures")
  : tmpdir();
const stamp = Date.now().toString(36);
const definitions = [];
let repository = null;

/**
 * Loop Engineering's durable side: definitions, their validation rules, and the guards on the run
 * controls.
 *
 * What this file deliberately does not do is start a Loop and let it finish. A run spawns the
 * worker and verifier Agents against a real worktree and iterates until the goal is met or a limit
 * trips (loop_engineering.rs:148-165 caps that at twenty iterations), which is minutes of wall
 * clock and real provider spend per run. The phase machine that would exercise is already covered
 * by the Rust unit tests around `loop_engineering.rs`; what has no coverage anywhere else, and is
 * what this adds, is that the definition survives the IPC boundary intact and that the run
 * controls refuse what they should.
 */
async function createRepository() {
  await mkdir(fixtureRoot, { recursive: true });
  const root = await mkdtemp(join(fixtureRoot, "loop-"));
  await run("git", ["init"], { cwd: root });
  await run("git", ["config", "user.email", "desktop-e2e@example.invalid"], { cwd: root });
  await run("git", ["config", "user.name", "Desktop E2E"], { cwd: root });
  await writeFile(join(root, "seed.txt"), "seed\n", "utf8");
  await run("git", ["add", "seed.txt"], { cwd: root });
  await run("git", ["commit", "-m", "fixture"], { cwd: root });
  return root;
}

function definitionInput(overrides = {}) {
  return {
    name: `desktop-e2e-loop-${stamp}`,
    enabled: true,
    projectPath: repository,
    baseBranch: "main",
    goal: "Keep the fixture green.",
    acceptanceCriteria: ["The seed file still exists."],
    allowedPaths: ["seed.txt"],
    protectedPaths: [],
    workerAgentId: "codex-cli",
    verifierAgentId: "claude-code",
    verificationCommands: [{
      id: "verify-seed",
      program: "git",
      args: ["status", "--porcelain"],
      workingDirectory: null,
      timeoutSeconds: 30,
      required: true,
    }],
    limits: {
      maxIterations: 3,
      stepTimeoutSeconds: 60,
      totalTimeoutSeconds: 600,
      maxConsecutiveRuntimeErrors: 2,
      maxConsecutiveNoProgress: 2,
    },
    ...overrides,
  };
}

globalThis.describe("VaneHub AI desktop Loop Engineering domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
    repository = await createRepository();
  });

  globalThis.it("takes a Loop definition through create, read back, update and delete", async () => {
    const input = definitionInput();
    const created = await invoke(({ core }, payload) => core.invoke("create_loop_definition", { input: payload }), input);
    definitions.push(created.id);

    assert.equal(created.name, input.name);
    assert.equal(normalizedPath(created.projectPath), normalizedPath(repository));
    assert.equal(created.workerAgentId, "codex-cli");
    assert.equal(created.verifierAgentId, "claude-code");
    assert.deepEqual(created.acceptanceCriteria, input.acceptanceCriteria);
    assert.equal(created.verificationCommands.length, 1);
    assert.equal(created.verificationCommands[0].program, "git");
    assert.equal(created.limits.maxIterations, 3);
    assert.ok(created.version >= 1, "a created definition carried no version");

    const listed = await invoke(({ core }) => core.invoke("list_loop_definitions"));
    assert.ok(listed.some((entry) => entry.id === created.id), "the created definition was not listed");

    // `expectedVersion` is the optimistic-concurrency token (types/loop.ts:80): the update has to
    // name the version it read, so two editors cannot silently overwrite each other.
    const updated = await invoke(
      ({ core }, payload) => core.invoke("update_loop_definition", {
        definitionId: payload.id,
        input: payload.input,
      }),
      {
        id: created.id,
        input: definitionInput({ goal: "Keep the fixture greener.", expectedVersion: created.version }),
      },
    );
    assert.equal(updated.goal, "Keep the fixture greener.");
    assert.ok(updated.version > created.version, "an accepted update did not advance the version");

    // Replaying the now-stale version is the case the token exists for.
    const stale = await attempt("update_loop_definition", {
      definitionId: created.id,
      input: definitionInput({ goal: "Third writer wins?", expectedVersion: created.version }),
    });
    assert.equal(stale.ok, false, "an update against a stale version was accepted");

    await invoke(({ core }, definitionId) => core.invoke("delete_loop_definition", { definitionId }), created.id);
    definitions.pop();
    const afterDelete = await invoke(({ core }) => core.invoke("list_loop_definitions"));
    assert.equal(
      afterDelete.some((entry) => entry.id === created.id),
      false,
      "the deleted definition was still listed",
    );
  });

  globalThis.it("refuses a definition that is missing what a Loop cannot run without", async () => {
    // loop_engineering.rs:302-306 -- a Loop with no acceptance criteria has no definition of done,
    // and one with no verification command has no way to check it. Both are rejected together.
    const noCriteria = await attempt("create_loop_definition", {
      input: definitionInput({ acceptanceCriteria: [] }),
    });
    assert.equal(noCriteria.ok, false, "a Loop with no acceptance criteria was accepted");

    const noVerification = await attempt("create_loop_definition", {
      input: definitionInput({ verificationCommands: [] }),
    });
    assert.equal(noVerification.ok, false, "a Loop with no verification command was accepted");

    // loop_engineering.rs:295-298 -- these are trimmed and then required, so whitespace is not a
    // way around them.
    for (const field of ["goal", "baseBranch", "workerAgentId", "verifierAgentId"]) {
      const refused = await attempt("create_loop_definition", {
        input: definitionInput({ [field]: "   " }),
      });
      assert.equal(refused.ok, false, `a Loop with a blank ${field} was accepted`);
    }
  });

  globalThis.it("refuses limits outside the range the runtime can honour", async () => {
    // loop_engineering.rs:154-165 -- iterations are capped at twenty, a step timeout cannot be
    // zero, the total cannot be shorter than one step, and neither consecutive-failure budget can
    // be zero (which would mean "give up before trying").
    const cases = [
      ["maxIterations", { maxIterations: 0 }],
      ["maxIterations above the cap", { maxIterations: 21 }],
      ["stepTimeoutSeconds", { stepTimeoutSeconds: 0 }],
      ["totalTimeoutSeconds shorter than a step", { stepTimeoutSeconds: 600, totalTimeoutSeconds: 60 }],
      ["maxConsecutiveRuntimeErrors", { maxConsecutiveRuntimeErrors: 0 }],
      ["maxConsecutiveNoProgress", { maxConsecutiveNoProgress: 0 }],
    ];
    for (const [label, override] of cases) {
      const refused = await attempt("create_loop_definition", {
        input: definitionInput({ limits: { ...definitionInput().limits, ...override } }),
      });
      assert.equal(refused.ok, false, `a Loop with an invalid ${label} was accepted`);
    }
  });

  globalThis.it("guards the run controls against ids that do not exist", async () => {
    // Every control takes a run id and none of them should invent a run for one that was never
    // started. Covering them together is the point: a control wired to the wrong lookup would
    // answer for a missing run rather than refuse.
    for (const command of ["get_loop_run", "pause_loop", "resume_loop", "cancel_loop", "accept_loop", "reject_loop"]) {
      const refused = await attempt(command, { runId: `no-such-run-${stamp}` });
      assert.equal(refused.ok, false, `${command} answered for a run id that does not exist`);
    }

    const refusedStart = await attempt("start_loop", { definitionId: `no-such-definition-${stamp}` });
    assert.equal(refusedStart.ok, false, "start_loop accepted a definition id that does not exist");
  });

  globalThis.it("reports an empty run history for a definition that has never started", async () => {
    const created = await invoke(
      ({ core }, payload) => core.invoke("create_loop_definition", { input: payload }),
      definitionInput({ name: `desktop-e2e-loop-history-${stamp}` }),
    );
    definitions.push(created.id);

    const runs = await invoke(
      ({ core }, definitionId) => core.invoke("list_loop_runs", { definitionId }),
      created.id,
    );
    assert.ok(Array.isArray(runs), "list_loop_runs did not return an array");
    assert.equal(runs.length, 0, "a definition that was never started already had run history");

    // The unscoped listing has to answer too -- the Loop centre opens on it.
    const all = await invoke(({ core }) => core.invoke("list_loop_runs", { definitionId: null }));
    assert.ok(Array.isArray(all), "the unscoped list_loop_runs did not return an array");
  });

  globalThis.after(async () => {
    for (const definitionId of definitions) {
      try {
        await invoke(({ core }, id) => core.invoke("delete_loop_definition", { definitionId: id }), definitionId);
      } catch (error) {
        globalThis.console.warn(`Cleanup step "delete Loop definition ${definitionId}" failed: ${error}`);
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
