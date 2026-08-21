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
const TERMINAL = new Set(["succeeded", "task_failed", "agent_failed", "timed_out", "stuck", "cancelled", "benchmark_error"]);

/**
 * Agent evaluation -- running several Agents head to head on one task and ranking the results.
 *
 * The catalogue, the guards and one full arena lifecycle are covered end to end here.
 *
 * The lifecycle case is affordable because a host without a provider credential settles every
 * attempt as `agent_failed` in seconds -- no generation is billed, and the start, the background
 * worker, the persistence, the read-side ranking, the export and the cancel guard are all real.
 * What is still out of reach is a *passing* attempt, which needs a live provider; the ordering of a
 * pass above a failure is covered in Rust instead (evaluation_api.rs
 * `ranking_beats_repository_id_order`).
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

  /**
   * The whole arena lifecycle, end to end, on a host with no provider credential.
   *
   * Every attempt settles as `agent_failed` within seconds there, which is what makes this
   * affordable: no generation is billed and nothing depends on a model's output, yet the start,
   * the background worker, the per-attempt persistence, the read-side ranking, the export and the
   * cancel guard are all real. What it deliberately does not cover is a *passing* attempt -- that
   * needs a live provider, and the ordering of a pass above a failure is covered in Rust
   * (evaluation_api.rs `ranking_beats_repository_id_order`).
   */
  globalThis.it("runs an arena through to a terminal verdict and reads it back consistently", async function lifecycle() {
    const tasks = await invoke(({ core }) => core.invoke("list_evaluation_tasks"));
    const task = tasks[0];
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const agentIds = agents.slice(0, 2).map((agent) => agent.id);
    if (!task || agentIds.length < 2) {
      blocked.push("evaluation lifecycle: fewer than two registered Agents to put in an arena");
      this.skip();
    }

    const arena = await invoke(({ core }, input) => core.invoke("start_evaluation", { input }), {
      taskId: task.id, taskVersion: task.version, agentIds,
    });
    assert.equal(arena.attempts.length, agentIds.length, "the arena did not create one attempt per Agent");
    assert.equal(arena.rankingVersion, "deterministic-v2", "the arena reported no ranking version");
    for (const attempt of arena.attempts) {
      assert.ok(attempt.canonicalRunId, `${attempt.id} was not linked to a canonical run`);
      assert.ok(Array.isArray(attempt.timeline) && attempt.timeline.length > 0, `${attempt.id} carried no timeline`);
    }

    // evaluation_api.rs:157 runs the attempts on a background thread, so `start_evaluation` answers
    // while every attempt is still queued. An arena that never leaves `queued` is the failure this
    // waits for: the client polls a non-terminal arena forever.
    const settled = await globalThis.browser.waitUntil(async () => {
      const current = await invoke(({ core }, arenaId) => core.invoke("get_evaluation_arena", { arenaId }), arena.id);
      return current.attempts.every((attempt) => TERMINAL.has(attempt.outcome)) ? current : false;
    }, { timeout: 240_000, timeoutMsg: "an evaluation attempt never reached a terminal outcome" });

    // A failure that records nothing is a failure nobody can act on: an attempt whose Agent could
    // not be dispatched carries the reason as a failed `agent-dispatch` check
    // (evaluation_api.rs `DISPATCH_CHECK_ID`), redacted to an exact safe reason.
    for (const attempt of settled.attempts.filter((item) => item.outcome === "agent_failed")) {
      const diagnostic = attempt.checks.find((check) => check.checkId === "agent-dispatch");
      assert.ok(diagnostic, `${attempt.agent.agentId} failed to dispatch without recording why`);
      assert.equal(diagnostic.passed, false, "the dispatch diagnostic was recorded as passing");
      assert.ok(diagnostic.summary.length > 0, "the dispatch diagnostic carried no reason");
    }

    const ids = settled.attempts.map((attempt) => attempt.id);
    const listed = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    const fromList = listed.find((item) => item.id === arena.id);
    assert.ok(fromList, "the settled arena was missing from the arena list");
    assert.deepEqual(fromList.attempts.map((attempt) => attempt.id), ids, "list and get disagree about attempt order");

    const exported = await invoke(({ core }, arenaId) => core.invoke("export_evaluation", { arenaId }), arena.id);
    assert.equal(exported.schemaVersion, 1, "the export carried no schema version");
    assert.deepEqual(exported.arena.attempts.map((attempt) => attempt.id), ids, "the export reordered the attempts");

    for (const attempt of settled.attempts) {
      const fetched = await invoke(({ core }, attemptId) => core.invoke("get_evaluation_attempt", { attemptId }), attempt.id);
      assert.equal(fetched.outcome, attempt.outcome, `${attempt.id} reported a different outcome when fetched alone`);
    }

    // evaluation_api.rs `cancel` must not rewrite a verdict an attempt already earned; an attempt
    // reported as cancelled loses the one fact that explained why it failed.
    const cancelled = await invoke(({ core }, arenaId) => core.invoke("cancel_evaluation", { arenaId }), arena.id);
    assert.deepEqual(
      cancelled.attempts.map((attempt) => attempt.outcome),
      settled.attempts.map((attempt) => attempt.outcome),
      "cancelling a settled arena overwrote the outcomes its attempts had already earned",
    );
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
