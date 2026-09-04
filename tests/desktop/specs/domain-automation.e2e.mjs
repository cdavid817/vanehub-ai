import assert from "node:assert/strict";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

async function attempt(command, args) {
  return invoke(({ core }, request) => core.invoke(request.command, request.args).then(
    (value) => ({ ok: true, value }),
    (error) => ({ ok: false, error }),
  ), { command, args: args ?? {} });
}

/**
 * Scheduled tasks and usage statistics -- one chapter in the user guide (automation.md), and two
 * feature areas that had no desktop coverage of any kind before this file.
 *
 * The scheduler itself runs in-process and only makes up missed runs at launch, so nothing here
 * waits for a task to fire: the assertions are about the record surviving its own lifecycle
 * (create/list/toggle/delete) and about the Agent-eligibility rule the create path enforces
 * (scheduled_tasks.rs:97-111 -- CLI Agents and OnePiece only).
 */
const stamp = Date.now().toString(36);
const created = [];

globalThis.describe("VaneHub AI desktop scheduled tasks and usage statistics", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("takes a scheduled task through create, disable, re-enable and delete", async function taskLifecycle() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    // scheduled_tasks.rs:99-111 -- eligibility is by launch kind, not by whether the Agent's
    // binary is installed, so an uninstalled CLI Agent is still a valid task target. That keeps
    // this case running on a host with no CLI at all.
    const eligible = agents.find((agent) => agent.id === "onepiece")
      ?? agents.find((agent) => agent.supportedInteractionModes.includes("cli"));
    if (!eligible) {
      blocked.push("scheduled task lifecycle: no CLI Agent or OnePiece is registered");
      this.skip();
    }

    const name = `desktop-e2e-task-${stamp}`;
    const task = await invoke(({ core }, input) => core.invoke("create_scheduled_task", { input }), {
      name,
      content: "Report the current status.",
      agentId: eligible.id,
      frequency: { kind: "minutes", interval: 5 },
    });
    created.push(task.id);
    assert.equal(task.name, name);
    assert.equal(task.agentId, eligible.id);
    assert.equal(task.enabled, true, "a newly created scheduled task should be enabled");
    assert.ok(task.nextRunAt, "the task was created without a computed next run");
    assert.equal(task.latestRunAt, null, "a task that has never run reported a last-run time");

    const listed = await invoke(({ core }) => core.invoke("list_scheduled_tasks"));
    assert.ok(listed.some((entry) => entry.id === task.id), "the created task was not listed");

    const disabled = await invoke(({ core }, input) => core.invoke("set_scheduled_task_enabled", { input }), {
      taskId: task.id,
      enabled: false,
    });
    assert.equal(disabled.enabled, false);
    const reEnabled = await invoke(({ core }, input) => core.invoke("set_scheduled_task_enabled", { input }), {
      taskId: task.id,
      enabled: true,
    });
    assert.equal(reEnabled.enabled, true);

    // The scheduler runs in-process and this task is five minutes out, so its run history is
    // legitimately empty -- what matters is that the query answers for a real task id rather than
    // rejecting it.
    //
    // `{ items, nextCursor }`, not a bare array: task 18.6 (this same OpenSpec change) moved this
    // command onto the same paginated envelope `list_evaluation_arenas` already uses
    // (scheduled_tasks.rs's own doc comment), and this assertion still expected the pre-18.6 shape
    // -- confirmed failing 3/3 attempts against a real desktop build before this fix.
    const page = await invoke(({ core }, taskId) => core.invoke("list_scheduled_task_runs", { taskId }), task.id);
    assert.ok(Array.isArray(page.items), "list_scheduled_task_runs did not return a paginated items array");
    assert.equal(page.items.length, 0, "a task scheduled five minutes out already had run history");

    await invoke(({ core }, taskId) => core.invoke("delete_scheduled_task", { taskId }), task.id);
    created.pop();
    const afterDelete = await invoke(({ core }) => core.invoke("list_scheduled_tasks"));
    assert.equal(
      afterDelete.some((entry) => entry.id === task.id),
      false,
      "the deleted task was still listed",
    );
  });

  globalThis.it("accepts every scheduling frequency the picker offers", async function frequencies() {
    const agents = await invoke(({ core }) => core.invoke("list_agents", { capabilityTag: null }));
    const eligible = agents.find((agent) => agent.id === "onepiece")
      ?? agents.find((agent) => agent.supportedInteractionModes.includes("cli"));
    if (!eligible) {
      blocked.push("scheduling frequencies: no CLI Agent or OnePiece is registered");
      this.skip();
    }

    // types/agent.ts:445-450 -- the five frequency shapes. Each one exercises a different branch of
    // `compute_next_run` (scheduled_tasks.rs:86), and a shape it cannot compute is rejected before
    // anything is persisted, so a successful create with a non-null nextRunAt is the assertion.
    const frequencies = [
      { kind: "minutes", interval: 15 },
      { kind: "hours", interval: 2 },
      { kind: "daily", timeOfDay: "09:30" },
      { kind: "weekly", weekday: 3, timeOfDay: "14:00" },
      { kind: "monthly", dayOfMonth: 1, timeOfDay: "08:00" },
    ];
    for (const frequency of frequencies) {
      const task = await invoke(({ core }, input) => core.invoke("create_scheduled_task", { input }), {
        name: `desktop-e2e-${frequency.kind}-${stamp}`,
        content: "Report the current status.",
        agentId: eligible.id,
        frequency,
      });
      created.push(task.id);
      assert.equal(task.frequency.kind, frequency.kind, `${frequency.kind} did not round-trip`);
      assert.ok(task.nextRunAt, `${frequency.kind} produced no next run time`);
    }
  });

  globalThis.it("refuses a task with no name, no content, or an ineligible Agent", async () => {
    // scheduled_tasks.rs:79-85 -- both fields are trimmed before the emptiness check, so
    // whitespace is not a way around it.
    const blankName = await attempt("create_scheduled_task", {
      input: { name: "   ", content: "Something", agentId: "onepiece", frequency: { kind: "minutes", interval: 5 } },
    });
    assert.equal(blankName.ok, false, "a whitespace-only task name was accepted");

    const blankContent = await attempt("create_scheduled_task", {
      input: { name: `blank-content-${stamp}`, content: "  ", agentId: "onepiece", frequency: { kind: "minutes", interval: 5 } },
    });
    assert.equal(blankContent.ok, false, "a whitespace-only task content was accepted");

    // scheduled_tasks.rs:106-110 -- an Agent id with no row at all is rejected as unsupported
    // rather than silently persisted against a dangling reference.
    const unknownAgent = await attempt("create_scheduled_task", {
      input: {
        name: `unknown-agent-${stamp}`,
        content: "Something",
        agentId: "no-such-agent",
        frequency: { kind: "minutes", interval: 5 },
      },
    });
    assert.equal(unknownAgent.ok, false, "a task against an unregistered Agent was accepted");

    const after = await invoke(({ core }) => core.invoke("list_scheduled_tasks"));
    const rejectedNames = new Set([`blank-content-${stamp}`, `unknown-agent-${stamp}`]);
    assert.equal(
      after.some((task) => !task.name.trim() || rejectedNames.has(task.name)),
      false,
      "a rejected task was persisted anyway",
    );
  });

  globalThis.it("reports a well-formed token usage summary and detail page", async () => {
    // token-usage.ts:93-102 -- the summary is a fixed shape with a schema version, and it answers
    // even when nothing has been recorded. An empty database returning a valid envelope is the
    // point: the statistics page renders from this, and a null or missing section would be a
    // crash rather than an empty chart.
    const summary = await invoke(({ core }, input) => core.invoke("get_token_usage_summary", { input }), {});
    assert.equal(summary.schemaVersion, 1);
    assert.ok(summary.totals, "the usage summary reported no totals");
    assert.ok(summary.userResponse, "the usage summary reported no user-response split");
    assert.ok(summary.internal, "the usage summary reported no internal split");
    assert.ok(Array.isArray(summary.daily), "the usage summary reported no daily series");
    assert.ok(Array.isArray(summary.breakdowns), "the usage summary reported no breakdowns");
    assert.ok(summary.generatedAt, "the usage summary carried no generation timestamp");

    // token-usage.ts:16-36 -- every quality tier is always present, so the page can label a tier
    // as empty rather than omit it. `reported` vs `estimated` is the distinction the guide makes a
    // point of, and conflating them is exactly what this asserts against.
    for (const tier of ["reported", "reportedDerived", "estimated"]) {
      assert.ok(summary.totals[tier], `the usage summary omitted the ${tier} tier`);
      assert.equal(typeof summary.totals[tier].callCount, "number");
    }

    const details = await invoke(({ core }, input) => core.invoke("get_token_usage_details", { input }), {
      limit: 10,
    });
    assert.ok(details, "get_token_usage_details returned nothing");
  });

  globalThis.it("applies a filter to the usage summary without changing its shape", async () => {
    // token-usage.ts:16-31 -- filters narrow the same envelope rather than switching it. A filter
    // that matches nothing must still answer with the full structure, which is what lets the page
    // show "no usage for this Agent" instead of failing to render.
    const filtered = await invoke(({ core }, input) => core.invoke("get_token_usage_summary", { input }), {
      agentId: "no-such-agent",
    });
    assert.equal(filtered.schemaVersion, 1);
    assert.ok(filtered.totals?.reported, "a filtered summary dropped its quality tiers");
    assert.equal(
      filtered.counts.calls,
      0,
      "a summary filtered to an unregistered Agent still counted calls",
    );
  });

  globalThis.after(async () => {
    for (const taskId of created) {
      try {
        await invoke(({ core }, id) => core.invoke("delete_scheduled_task", { taskId: id }), taskId);
      } catch (error) {
        globalThis.console.warn(`Cleanup step "delete scheduled task ${taskId}" failed: ${error}`);
      }
    }
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
