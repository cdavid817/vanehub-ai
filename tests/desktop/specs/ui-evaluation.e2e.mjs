import assert from "node:assert/strict";

/**
 * Interactive coverage of Agent evaluation: everything here goes through a control on screen.
 *
 * `domain-evaluation.e2e.mjs` drives `core.invoke` and proves the commands answer. That leaves the
 * gap this file covers -- the evaluation center can be wired to nothing, or wired to a stale copy
 * of what the backend said, and no amount of IPC coverage notices. IPC is used here only to state
 * what the screen should agree with; every assertion reads the rendered DOM.
 *
 * An arena is started for real. On a host with no provider credential every attempt settles as
 * `agent_failed` within a couple of seconds, which is the point: the lifecycle, the persistence and
 * the rendering are all exercised without billing a single generation. What is deliberately not
 * covered here is a *passing* attempt -- that needs a live provider, and the ranking that orders
 * one above another has its own Rust coverage (evaluation_api.rs `ranking_beats_repository_id_order`).
 *
 * Selectors are structural or `data-testid`. The app's default language on this host is zh-CN
 * (src/i18n/supported-locales.ts:47), so a selector spelled as visible text is one translation away
 * from silently matching nothing.
 */
const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const TERMINAL = new Set(["succeeded", "task_failed", "agent_failed", "timed_out", "stuck", "cancelled", "benchmark_error"]);
const blocked = [];
let startedArenaId = null;

async function bootstrapReady() {
  const root = await globalThis.$("#root");
  await root.waitForExist({ timeout: 120_000 });
  await globalThis.browser.waitUntil(
    async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
    { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
  );
  assert.equal(
    await root.getAttribute("data-vanehub-fatal-error"),
    null,
    "the workspace tripped its fatal error boundary",
  );
}

async function navigate(path) {
  await globalThis.browser.execute((target) => {
    globalThis.history.pushState({}, "", target);
    globalThis.dispatchEvent(new globalThis.PopStateEvent("popstate"));
  }, path);
}

async function openEvaluationCenter() {
  await navigate("/workspace/evaluations");
  const center = await globalThis.$('[data-testid="evaluation-center"]');
  // Lazily loaded on first visit (main-layout.tsx:445), so the wait is for the chunk as well as
  // for the initial catalogue round trip.
  await center.waitForExist({ timeout: 60_000 });
  return center;
}

function rows() {
  return globalThis.$$('[data-testid="evaluation-row"]');
}

async function rowOutcomes() {
  const found = [];
  for (const row of await rows()) {
    found.push({
      attemptId: await row.getAttribute("data-attempt-id"),
      outcome: await row.getAttribute("data-outcome"),
      text: await row.getText(),
    });
  }
  return found;
}

globalThis.describe("VaneHub AI desktop Agent evaluation UI", () => {
  globalThis.before(async () => {
    // Never inherit the previous spec's page: route changes commit inside `startTransition`, so a
    // page left mid-flight keeps rendering the old DOM with no error to notice.
    await globalThis.browser.refresh();
    await bootstrapReady();
  });

  globalThis.it("offers exactly the benchmark tasks the catalogue publishes", async () => {
    const center = await openEvaluationCenter();
    const tasks = await invoke(({ core }) => core.invoke("list_evaluation_tasks"));
    const picker = await center.$('[data-testid="evaluation-task"]');
    await picker.waitForExist({ timeout: 30_000 });
    const options = await globalThis.browser.execute(
      (select) => Array.from(select.options).map((option) => option.value),
      picker,
    );
    assert.deepEqual(
      options,
      tasks.map((task) => task.id),
      "the task picker and the catalogue disagree about which benchmarks exist",
    );
    assert.equal(
      await picker.getValue(),
      tasks[0].id,
      "the picker opened on no task, so Run would have nothing to run",
    );
  });

  globalThis.it("refuses to run an arena with no Agent selected", async () => {
    const center = await openEvaluationCenter();
    const run = await center.$('[data-testid="evaluation-run"]');
    await run.waitForClickable({ timeout: 30_000 });

    const toggles = await center.$$('fieldset input[type="checkbox"]');
    assert.ok(toggles.length > 0, "the Agent picker offered nothing to select");
    for (const toggle of toggles) {
      if (await toggle.isSelected()) await toggle.click();
    }
    await globalThis.browser.waitUntil(async () => !(await run.isEnabled()), {
      timeout: 15_000,
      timeoutMsg: "Run stayed enabled with every Agent unchecked -- an arena needs at least one.",
    });

    for (const toggle of toggles) await toggle.click();
    await globalThis.browser.waitUntil(async () => run.isEnabled(), {
      timeout: 15_000,
      timeoutMsg: "Run stayed disabled after the Agents were checked back on.",
    });
  });

  globalThis.it("runs an arena, settles every attempt, and persists it", async () => {
    const center = await openEvaluationCenter();
    const before = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    const known = new Set(before.map((arena) => arena.id));

    const toggles = await center.$$('fieldset input[type="checkbox"]');
    const selectedAgents = [];
    for (const toggle of toggles) {
      if (await toggle.isSelected()) {
        selectedAgents.push((await toggle.getAttribute("data-testid")).replace("evaluation-agent-", ""));
      }
    }
    // The table lists every arena's attempts together, so the count to wait for is the previous
    // one plus this run's, not this run's alone -- and it has to be read before the click, because
    // `start_evaluation` answers with the queued attempts fast enough to be counted as "existing".
    const existingRows = (await rows()).length;
    const run = await center.$('[data-testid="evaluation-run"]');
    await run.waitForClickable({ timeout: 30_000 });
    await run.click();

    await globalThis.browser.waitUntil(
      async () => (await rows()).length >= existingRows + selectedAgents.length,
      { timeout: 60_000, timeoutMsg: "Run produced no result rows." },
    );
    // Attempts start `queued` and the center polls until they settle. Waiting on the row's own
    // outcome attribute is what proves the polling reaches the screen rather than only the store.
    await globalThis.browser.waitUntil(
      async () => (await rowOutcomes()).every((row) => TERMINAL.has(row.outcome)),
      { timeout: 180_000, timeoutMsg: "An attempt never reached a terminal outcome on screen." },
    );

    const rendered = await rowOutcomes();
    for (const row of rendered) {
      assert.ok(
        !row.text.includes("evaluation.outcome"),
        `the outcome cell rendered a raw translation key: ${row.text}`,
      );
    }

    const after = await invoke(({ core }) => core.invoke("list_evaluation_arenas"));
    const created = after.filter((arena) => !known.has(arena.id));
    assert.equal(created.length, 1, "the run did not persist exactly one arena");
    startedArenaId = created[0].id;
    assert.equal(
      created[0].attempts.length,
      selectedAgents.length,
      "the arena did not create one attempt per selected Agent",
    );
    // Newest arena first (evaluation_repository.rs list orders by created_at DESC), so this run's
    // attempts head the table -- in the order the ranked arena reports them.
    assert.deepEqual(
      rendered.slice(0, selectedAgents.length).map((row) => row.attemptId),
      created[0].attempts.map((attempt) => attempt.id),
      "the table rendered the attempts in a different order than the ranked arena reports",
    );
  });

  globalThis.it("keeps the detail pane on the live attempt", async () => {
    const center = await openEvaluationCenter();
    const rendered = await rowOutcomes();
    if (rendered.length === 0) {
      blocked.push("evaluation detail pane: no arena on screen to inspect");
      return;
    }
    const detail = await center.$('[data-testid="evaluation-detail"]');
    // Read before anything is clicked. Run selected this attempt while it was still `queued`, and
    // the pane used to hold that captured object: polling replaced the arena around it, so it went
    // on reporting `queued` -- Cancel button and all -- beside a row that had already settled.
    const autoSelected = await detail.getAttribute("data-selected-attempt");
    const autoRow = rendered.find((row) => row.attemptId === autoSelected);
    assert.ok(autoRow, "Run left the detail pane on an attempt the table does not list");
    assert.equal(
      await detail.getAttribute("data-selected-outcome"),
      autoRow.outcome,
      "the detail pane is still showing the attempt as it looked when the run started",
    );

    const last = (await rows()).at(-1);
    await last.click();
    const lastRow = rendered.at(-1);
    await globalThis.browser.waitUntil(
      async () => (await detail.getAttribute("data-selected-attempt")) === lastRow.attemptId,
      { timeout: 30_000, timeoutMsg: "Clicking a row did not move the detail pane to it." },
    );
    assert.equal(
      await detail.getAttribute("data-selected-outcome"),
      lastRow.outcome,
      "The detail pane reported a different outcome than the row it was opened from.",
    );
    assert.equal(
      await (await center.$$('[data-testid="evaluation-cancel"]')).length,
      0,
      "Cancel was offered for an attempt that had already settled",
    );
    const text = await detail.getText();
    assert.ok(text.includes("runtime-snapshot-v1"), "the detail pane showed no configuration fingerprint");
    assert.ok(text.includes(lastRow.outcome), "the timeline did not name the attempt's outcome");

    // An attempt whose Agent could not be dispatched used to render an empty verification block:
    // `agent_failed`, `0/0` checks, no metrics, nothing saying why. The reason is recorded as a
    // failed `agent-dispatch` check, so it has to be readable here rather than only in the logs.
    if (lastRow.outcome === "agent_failed") {
      const attempt = await invoke(
        ({ core }, attemptId) => core.invoke("get_evaluation_attempt", { attemptId }),
        lastRow.attemptId,
      );
      const diagnostic = attempt.checks.find((check) => check.checkId === "agent-dispatch");
      assert.ok(diagnostic, "a dispatch failure recorded no diagnostic check");
      assert.equal(diagnostic.passed, false, "the dispatch diagnostic was recorded as passing");
      assert.ok(diagnostic.summary.length > 0, "the dispatch diagnostic carried no reason");
      assert.ok(
        text.includes(diagnostic.summary),
        `the detail pane did not show the recorded reason: ${diagnostic.summary}`,
      );
    }
  });

  globalThis.it("filters the results by Agent", async () => {
    const center = await openEvaluationCenter();
    const rendered = await rowOutcomes();
    if (rendered.length < 2) {
      blocked.push("evaluation filter: fewer than two attempts on screen to narrow between");
      return;
    }
    const target = rendered[0].text.split("\n")[0];
    const filter = await center.$('[data-testid="evaluation-filter"]');
    // Assigned through the prototype setter rather than typed: synthetic key events do not reach
    // controlled inputs reliably on WebKitGTK (tests/desktop/helpers/form-control.mjs).
    await globalThis.browser.execute((input, next) => {
      const setter = Object.getOwnPropertyDescriptor(globalThis.HTMLInputElement.prototype, "value")?.set;
      setter?.call(input, next);
      input.dispatchEvent(new globalThis.Event("input", { bubbles: true }));
    }, filter, target);

    await globalThis.browser.waitUntil(async () => (await rows()).length < rendered.length, {
      timeout: 15_000,
      timeoutMsg: `Filtering on ${target} narrowed nothing.`,
    });
    for (const row of await rowOutcomes()) {
      assert.ok(row.text.includes(target), `a row that does not match ${target} survived the filter`);
    }

    await globalThis.browser.execute((input) => {
      const setter = Object.getOwnPropertyDescriptor(globalThis.HTMLInputElement.prototype, "value")?.set;
      setter?.call(input, "");
      input.dispatchEvent(new globalThis.Event("input", { bubbles: true }));
    }, filter);
    await globalThis.browser.waitUntil(async () => (await rows()).length === rendered.length, {
      timeout: 15_000,
      timeoutMsg: "Clearing the filter did not restore every row.",
    });
  });

  globalThis.it("exports the arena the row belongs to", async function exportRow() {
    const center = await openEvaluationCenter();
    if (!startedArenaId) {
      blocked.push("evaluation export: no arena was started in this run");
      this.skip();
    }
    const exported = await invoke(
      ({ core }, arenaId) => core.invoke("export_evaluation", { arenaId }),
      startedArenaId,
    );
    assert.equal(exported.schemaVersion, 1, "the export carried no schema version");
    assert.equal(exported.arena.id, startedArenaId, "the export named a different arena");
    const ids = new Set(exported.arena.attempts.map((attempt) => attempt.id));
    assert.deepEqual(
      (await rowOutcomes()).map((row) => row.attemptId).filter((id) => ids.has(id)),
      exported.arena.attempts.map((attempt) => attempt.id),
      "the export ordered attempts differently than the table renders them",
    );

    // The button's own effect -- a blob anchor download -- is not observable from the driver in
    // this runtime, so what is asserted is that pressing it neither throws into the error banner
    // nor trips the fatal boundary. The payload it hands over is the one asserted above.
    const button = await center.$('[data-testid="evaluation-export"]');
    await button.waitForClickable({ timeout: 30_000 });
    await button.click();
    await globalThis.browser.pause(500);
    assert.equal(
      await (await center.$$('[role="alert"]')).length,
      0,
      "exporting raised the evaluation error banner",
    );
    assert.equal(
      await (await globalThis.$("#root")).getAttribute("data-vanehub-fatal-error"),
      null,
      "exporting tripped the fatal error boundary",
    );
  });

  globalThis.it("shows the persisted arena again after a reload", async function afterReload() {
    if (!startedArenaId) {
      blocked.push("evaluation reload: no arena was started in this run");
      this.skip();
    }
    const expected = await invoke(
      ({ core }, arenaId) => core.invoke("get_evaluation_arena", { arenaId }),
      startedArenaId,
    );
    await globalThis.browser.refresh();
    await bootstrapReady();
    await openEvaluationCenter();
    await globalThis.browser.waitUntil(
      async () => (await rows()).length >= expected.attempts.length,
      { timeout: 60_000, timeoutMsg: "The persisted arena did not come back after a reload." },
    );
    const restored = await rowOutcomes();
    assert.deepEqual(
      restored.filter((row) => expected.attempts.some((attempt) => attempt.id === row.attemptId))
        .map((row) => row.outcome),
      expected.attempts.map((attempt) => attempt.outcome),
      "the reloaded table disagreed with the stored arena about its outcomes",
    );
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
  });
});
