import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

const stamp = Date.now().toString(36);

/**
 * Agent evaluation -- running several Agents head to head on one task and ranking the results.
 *
 * The catalogue and the guards are covered end to end here. Actually starting an arena is not:
 * `start_evaluation` (evaluation_api.rs:78-110) creates one agent run per Agent and drives each
 * through the real provider, so a single arena is several billed generations plus however long
 * the task's own timeout allows. What that would prove beyond the guards below is the ranking
 * itself, which has its own Rust coverage; what has no coverage anywhere is that the catalogue
 * crosses IPC intact and that the entry points refuse a malformed arena rather than half-starting
 * one.
 */
globalThis.describe("VaneHub AI desktop Agent evaluation domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("publishes the built-in evaluation task catalogue", async () => {
    // evaluation_api.rs:71-76 -- the catalogue is parsed from built-in manifests on every call, so
    // a manifest that stopped parsing would surface here as an error rather than as a task
    // silently missing from the list.
    const tasks = await invoke(({ core }) => core.invoke("list_evaluation_tasks"));
    assert.ok(Array.isArray(tasks) && tasks.length > 0, "the evaluation task catalogue was empty");
    for (const task of tasks) {
      assert.ok(task.id, "an evaluation task carried no id");
      assert.ok(Number.isInteger(task.version) && task.version > 0, `${task.id} carried no usable version`);
      assert.ok(task.category, `${task.id} carried no category`);
      assert.ok(task.prompt, `${task.id} carried no prompt`);
      assert.ok(task.timeoutSeconds > 0, `${task.id} carried no timeout`);
      assert.ok(Array.isArray(task.verifierProfiles), `${task.id} carried no verifier profiles`);
    }
    const ids = tasks.map((task) => task.id);
    assert.equal(new Set(ids).size, ids.length, "the evaluation catalogue contained duplicate task ids");
  });

  globalThis.it("reports the arena list and refuses ids that were never created", async () => {
    const arenas = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    assert.ok(Array.isArray(arenas), "list_evaluation_arenas did not return an array");

    for (const [command, key] of [
      ["get_evaluation_arena", "arenaId"],
      ["cancel_evaluation", "arenaId"],
      ["export_evaluation", "arenaId"],
    ]) {
      const refused = await attempt(command, { [key]: `no-such-arena-${stamp}` });
      assert.equal(refused.ok, false, `${command} answered for an arena that does not exist`);
    }

    const refusedAttempt = await attempt("get_evaluation_attempt", { attemptId: `no-such-attempt-${stamp}` });
    assert.equal(refusedAttempt.ok, false, "get_evaluation_attempt answered for an attempt that does not exist");
  });

  globalThis.it("refuses to start an arena with no Agents, too many, or an unknown task", async () => {
    const tasks = await invoke(({ core }) => core.invoke("list_evaluation_tasks"));
    const task = tasks[0];
    assert.ok(task, "no evaluation task to build the guard cases from");

    // evaluation_api.rs:80-84 -- an arena needs between one and eight Agents. Zero has nothing to
    // rank; past the cap the arena would outrun MAX_ARENA_ATTEMPTS.
    const empty = await attempt("start_evaluation", {
      input: { taskId: task.id, taskVersion: task.version, agentIds: [] },
    });
    assert.equal(empty.ok, false, "an arena with no Agents was started");

    const tooMany = await attempt("start_evaluation", {
      input: {
        taskId: task.id,
        taskVersion: task.version,
        agentIds: Array.from({ length: 9 }, (_, index) => `agent-${index}`),
      },
    });
    assert.equal(tooMany.ok, false, "an arena with nine Agents was started");

    // evaluation_api.rs:79 -- the manifest is resolved by id *and* version, so a task id that
    // exists at a version that does not is still rejected.
    const unknownTask = await attempt("start_evaluation", {
      input: { taskId: `no-such-task-${stamp}`, taskVersion: 1, agentIds: ["onepiece"] },
    });
    assert.equal(unknownTask.ok, false, "an arena against an unknown task was started");

    const unknownVersion = await attempt("start_evaluation", {
      input: { taskId: task.id, taskVersion: task.version + 1000, agentIds: ["onepiece"] },
    });
    assert.equal(unknownVersion.ok, false, "an arena against an unknown task version was started");

    // A rejected start must not leave a half-built arena behind -- the arena id is minted before
    // the per-Agent runs are created (evaluation_api.rs:85), so this is the assertion that the
    // failure path does not persist one.
    const arenas = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    assert.equal(
      arenas.some((arena) => arena.taskId === `no-such-task-${stamp}`),
      false,
      "a refused start left an arena behind",
    );
  });

  globalThis.it("refuses an arena naming an Agent that is not registered", async function unknownAgent() {
    const tasks = await invoke(({ core }) => core.invoke("list_evaluation_tasks"));
    const task = tasks[0];
    if (!task) {
      blocked.push("evaluation unknown-Agent guard: the task catalogue is empty");
      this.skip();
    }

    // evaluation_api.rs:102 resolves every Agent id through the runtime before any attempt is
    // created, so an unregistered id fails the whole start rather than producing an arena with a
    // dead attempt in it.
    const refused = await attempt("start_evaluation", {
      input: { taskId: task.id, taskVersion: task.version, agentIds: [`no-such-agent-${stamp}`] },
    });
    assert.equal(refused.ok, false, "an arena naming an unregistered Agent was started");
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
