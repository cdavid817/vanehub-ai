import assert from "node:assert/strict";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// Ids are run-scoped: `work_item_links` keys on (source_kind, source_id) alone
// (src-tauri/src/contexts/work_board/infrastructure.rs:30), so a fixed source id collides with
// itself the second time this spec runs against the same data directory.
const RUN = process.env.VANEHUB_TEST_RUN_ID ?? String(Date.now());

// A path string, never a directory on disk. The board stores `project_path` verbatim and nothing
// on these code paths opens it. Rooted under the run's fixture directory anyway, so it cannot
// name a real repository even by accident.
const PROJECT_PATH = join(
  process.env.VANEHUB_APP_DATA_DIR ? dirname(process.env.VANEHUB_APP_DATA_DIR) : tmpdir(),
  "fixtures",
  `domain-work-board-${RUN}`,
);

// Spec files in one desktop run share a single isolated data directory, so everything created
// here is state the next spec inherits. Registered on creation rather than on success: a case
// that fails halfway still has to hand the run back clean.
const createdWorkItems = new Set();
const createdGoals = new Set();

/**
 * Runs a command that is expected to be rejected and returns the native error message.
 *
 * The rejection is caught inside the WebView rather than at the WDIO boundary. The direct-eval
 * bridge collapses any thrown value to `(e && e.message) || String(e)` and re-raises it as a
 * transport failure on this side, which makes "the command rejected as designed" indistinguishable
 * from "the bridge broke". Work board and Goals reject with `Result<_, String>`, which arrives
 * here as a plain string.
 */
const rejectionOf = (command, args) => invoke(
  ({ core }, name, payload) => core.invoke(name, payload).then(
    () => null,
    (error) => (typeof error === "string" ? error : String(error?.message ?? error)),
  ),
  command,
  args,
);

// work_board/commands.rs:6 -- `filters: WorkItemFilters`, whose only field is
// `archived: bool` (contexts/work_board/models.rs:33-38).
const listWorkItems = (archived) => invoke(
  ({ core }, value) => core.invoke("list_work_items", { filters: { archived: value } }),
  archived,
);

const findWorkItem = async (workItemId, archived) => {
  const items = await listWorkItems(archived);
  return items.find((item) => item.id === workItemId) ?? null;
};

// work_board/commands.rs:14 -- `input: CreateWorkItemInput`
// (contexts/work_board/models.rs:40-50). `stage` and `priority` are checked against the
// STAGES/PRIORITIES lists in contexts/work_board/api.rs:11-12, both snake_case.
const createWorkItem = async (title, stage) => {
  const item = await invoke(({ core }, input) => core.invoke("create_work_item", { input }), {
    title,
    description: "Created by the desktop orchestration sweep.",
    stage,
    priority: "medium",
    projectPath: PROJECT_PATH,
    dueAt: null,
  });
  createdWorkItems.add(item.id);
  return item;
};

// goals/get_goal.rs:8 -- `goal_id: String`, which Tauri exposes as `goalId`.
const getGoal = (goalId) => invoke(
  ({ core }, id) => core.invoke("get_goal", { goalId: id }),
  goalId,
);

globalThis.describe("VaneHub AI desktop orchestration domains", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("creates, edits, moves and reorders work items on the board", async () => {
    const first = await createWorkItem(`Board sweep primary ${RUN}`, "inbox");
    const second = await createWorkItem(`Board sweep secondary ${RUN}`, "inbox");
    assert.equal(first.stage, "inbox");
    assert.equal(first.priority, "medium");
    assert.equal(first.archived, false);
    assert.equal(first.projectPath, PROJECT_PATH);
    assert.deepEqual(first.sources, []);

    // work_board/commands.rs:22 -- `input: UpdateWorkItemInput`
    // (contexts/work_board/models.rs:52-61). `projectPath` and `dueAt` are `Option<Option<String>>`:
    // a JSON `null` collapses to the *outer* `None`, which contexts/work_board/api.rs:61-65 reads as
    // "leave it alone". Setting a value works; clearing one is unreachable through this DTO.
    const edited = await invoke(({ core }, input) => core.invoke("update_work_item", { input }), {
      workItemId: first.id,
      title: `Board sweep primary ${RUN} (edited)`,
      description: "Edited by the desktop orchestration sweep.",
      priority: "high",
      projectPath: null,
      dueAt: "2026-12-31T00:00:00Z",
    });
    assert.equal(edited.title, `Board sweep primary ${RUN} (edited)`);
    assert.equal(edited.priority, "high");
    assert.equal(edited.dueAt, "2026-12-31T00:00:00Z", "a supplied dueAt did not persist");
    assert.equal(edited.projectPath, PROJECT_PATH, "a null projectPath cleared the field");

    // work_board/commands.rs:30 -- `input: MoveWorkItemInput`
    // (contexts/work_board/models.rs:63-69).
    const moved = await invoke(({ core }, input) => core.invoke("move_work_item", { input }), {
      workItemId: first.id,
      stage: "review",
      beforeWorkItemId: null,
    });
    assert.equal(moved.stage, "review", "move_work_item did not change the stage");
    assert.notEqual(moved.stage, first.stage);

    // Moving *before* an item is the drag-and-drop path: it renormalises the stage and then places
    // the moved item at a lower rank than its target (contexts/work_board/api.rs:79-107).
    const reordered = await invoke(({ core }, input) => core.invoke("move_work_item", { input }), {
      workItemId: second.id,
      stage: "review",
      beforeWorkItemId: first.id,
    });
    assert.equal(reordered.stage, "review");
    const target = await findWorkItem(first.id, false);
    assert.ok(reordered.rank < target.rank, "the reordered item did not land ahead of its target");

    // work_board/commands.rs:38 -- `input: LinkWorkItemSourceInput`
    // (contexts/work_board/models.rs:71-78). `sourceKind` and `relation` are checked against the
    // SOURCES/RELATIONS lists in contexts/work_board/api.rs:13-14.
    //
    // A deliberately absent session: every real source in this database may already be claimed by
    // another card, and the link table's primary key is the source alone. An unresolvable source
    // also exercises the projection
    // branch that reports a vanished source as unavailable rather than dropping the link
    // (contexts/work_board/infrastructure.rs:302-305).
    const sourceId = `desktop-sweep-missing-run-${RUN}`;
    const linked = await invoke(({ core }, input) => core.invoke("link_work_item_source", { input }), {
      workItemId: first.id,
      sourceKind: "session",
      sourceId,
      relation: "execution",
    });
    const source = linked.sources.find((entry) => entry.sourceId === sourceId);
    assert.ok(source, "the source link was not persisted on the work item");
    assert.equal(source.sourceKind, "session");
    assert.equal(source.relation, "execution");
    assert.equal(source.available, false);
    assert.equal(source.status, "unavailable");

    const duplicate = await rejectionOf("link_work_item_source", {
      input: { workItemId: second.id, sourceKind: "session", sourceId, relation: "supporting" },
    });
    assert.match(
      duplicate ?? "",
      /already linked/,
      "a source claimed by another card was linked a second time",
    );
  });

  globalThis.it("round-trips a work item through archive, restore and delete", async () => {
    const item = await createWorkItem(`Board sweep archival ${RUN}`, "planned");

    // work_board/commands.rs:62 -- deletion is gated on the item already being archived
    // (contexts/work_board/api.rs:173-184).
    const premature = await rejectionOf("delete_work_item", { workItemId: item.id });
    assert.match(premature ?? "", /Archive a work item/, "a live work item was deletable");

    // work_board/commands.rs:46 / :54 -- `work_item_id: String`, exposed as `workItemId`.
    const archived = await invoke(
      ({ core }, id) => core.invoke("archive_work_item", { workItemId: id }),
      item.id,
    );
    assert.equal(archived.archived, true);
    assert.equal(await findWorkItem(item.id, false), null, "an archived card is still on the board");
    assert.ok(await findWorkItem(item.id, true), "an archived card is missing from the archive");

    const restored = await invoke(
      ({ core }, id) => core.invoke("restore_work_item", { workItemId: id }),
      item.id,
    );
    assert.equal(restored.archived, false);
    assert.equal(restored.stage, "planned", "restoring moved the card to another stage");
    assert.equal(restored.title, item.title);
    assert.ok(await findWorkItem(item.id, false), "a restored card did not return to the board");
    assert.equal(await findWorkItem(item.id, true), null, "a restored card is still in the archive");

    await invoke(({ core }, id) => core.invoke("archive_work_item", { workItemId: id }), item.id);
    await invoke(({ core }, id) => core.invoke("delete_work_item", { workItemId: id }), item.id);
    createdWorkItems.delete(item.id);
    assert.equal(await findWorkItem(item.id, false), null);
    assert.equal(await findWorkItem(item.id, true), null, "the deleted card survived in the archive");
  });

  globalThis.it("takes a Goal through create, activate, link, accept, reopen and abandon", async () => {
    // Every one of these commands answered "Command list_goals not found" until the name-based
    // router learned about them: they were in the handler macro but not in `is_command`
    // (src-tauri/src/commands/supplemental_registry.rs:57-111). Reading state back after each
    // transition is what makes this a guard rather than a smoke test.

    // goals/create_goal.rs:8 -- `input: GoalInputDto` (commands/goals/dto.rs:5-15).
    const goal = await invoke(({ core }, input) => core.invoke("create_goal", { input }), {
      title: `Goal sweep ${RUN}`,
      description: "Created by the desktop orchestration sweep.",
      acceptanceNotes: "Accepted once the linked work item reaches done.",
      projectPath: PROJECT_PATH,
    });
    createdGoals.add(goal.id);
    // `status` is what the row stores; `derivedStatus` is recomputed from the children on every
    // read and is the only place `awaiting_acceptance` exists (commands/goals/dto.rs:46-49,
    // contexts/goals/application/progress.rs:11-31).
    assert.equal(goal.status, "draft");
    assert.equal(goal.derivedStatus, "draft");
    assert.deepEqual(goal.links, []);

    // goals/update_goal.rs:8 -- `goal_id` plus a second `input: GoalInputDto`.
    const renamed = await invoke(({ core }, payload) => core.invoke("update_goal", payload), {
      goalId: goal.id,
      input: {
        title: `Goal sweep ${RUN} (edited)`,
        description: "Edited by the desktop orchestration sweep.",
        acceptanceNotes: "Accepted once the linked work item reaches done.",
        projectPath: PROJECT_PATH,
      },
    });
    assert.equal(renamed.title, `Goal sweep ${RUN} (edited)`);
    assert.equal(renamed.status, "draft", "editing a goal moved it out of draft");
    assert.equal((await getGoal(goal.id)).title, `Goal sweep ${RUN} (edited)`);

    // goals/activate_goal.rs:11 -- draft -> active (contexts/goals/domain/goal.rs:64-73).
    const active = await invoke(
      ({ core }, id) => core.invoke("activate_goal", { goalId: id }),
      goal.id,
    );
    assert.equal(active.status, "active");
    assert.equal(active.derivedStatus, "active");

    // goals/accept_goal.rs:10 -- acceptance is the one transition the system never makes on its
    // own, and it is refused while nothing derives to awaiting acceptance
    // (contexts/goals/domain/goal.rs:86-91).
    const early = await rejectionOf("accept_goal", { goalId: goal.id });
    assert.match(early ?? "", /awaiting acceptance/, "a goal with no finished children was accepted");

    const child = await createWorkItem(`Goal sweep child ${RUN}`, "inbox");
    // goals/link_goal_target.rs:8 -- `goal_id`, `target_kind`, `target_id`. `target_kind` goes
    // through `GoalLinkTarget::parse`, which spells the work item variant `work_item`
    // (contexts/goals/domain/link.rs:26-35).
    const withLink = await invoke(({ core }, payload) => core.invoke("link_goal_target", payload), {
      goalId: goal.id,
      targetKind: "work_item",
      targetId: child.id,
    });
    assert.deepEqual(withLink.links, [{
      targetKind: "work_item",
      targetId: child.id,
      progress: "active",
    }]);
    assert.equal(withLink.counted, 1);
    assert.equal(withLink.terminal, 0);
    assert.equal(withLink.unresolvable, 0);
    assert.equal(withLink.derivedStatus, "active", "an unfinished child already derived to ready");

    // A work item counts as terminal once it is archived or reaches `done`
    // (contexts/goals/infrastructure/progress_probes.rs:122-138), which is what flips the goal's
    // derived status without touching its stored one.
    await invoke(({ core }, input) => core.invoke("move_work_item", { input }), {
      workItemId: child.id,
      stage: "done",
      beforeWorkItemId: null,
    });
    const ready = await getGoal(goal.id);
    assert.equal(ready.status, "active", "the stored status changed on its own");
    assert.equal(ready.derivedStatus, "awaiting_acceptance");
    assert.equal(ready.terminal, 1);
    assert.equal(ready.links[0].progress, "terminal");

    const accepted = await invoke(
      ({ core }, id) => core.invoke("accept_goal", { goalId: id }),
      goal.id,
    );
    assert.equal(accepted.status, "achieved");
    assert.equal(accepted.derivedStatus, "achieved");
    assert.equal((await getGoal(goal.id)).status, "achieved", "acceptance did not survive a read");

    // goals/reopen_goal.rs:8 -- achieved -> active. The child is still done, so the reopened goal
    // immediately derives back to awaiting acceptance while storing `active`. That divergence is
    // the whole point of the derived field.
    const reopened = await invoke(
      ({ core }, id) => core.invoke("reopen_goal", { goalId: id }),
      goal.id,
    );
    assert.equal(reopened.status, "active");
    assert.equal(reopened.derivedStatus, "awaiting_acceptance");

    // goals/unlink_goal_target.rs:8 -- same three arguments as the link command.
    const unlinked = await invoke(({ core }, payload) => core.invoke("unlink_goal_target", payload), {
      goalId: goal.id,
      targetKind: "work_item",
      targetId: child.id,
    });
    assert.deepEqual(unlinked.links, []);
    assert.equal(unlinked.counted, 0);
    assert.equal(unlinked.derivedStatus, "active", "a goal with no children still derived as ready");

    // goals/abandon_goal.rs:8 -- active -> abandoned.
    const abandoned = await invoke(
      ({ core }, id) => core.invoke("abandon_goal", { goalId: id }),
      goal.id,
    );
    assert.equal(abandoned.status, "abandoned");
    assert.equal(abandoned.derivedStatus, "abandoned");

    // goals/list_goals.rs:8 -- no arguments.
    const listed = await invoke(({ core }) => core.invoke("list_goals"));
    const stored = listed.find((entry) => entry.id === goal.id);
    assert.ok(stored, "an abandoned goal fell out of the list");
    assert.equal(stored.status, "abandoned");

    // goals/delete_goal.rs:7
    await invoke(({ core }, id) => core.invoke("delete_goal", { goalId: id }), goal.id);
    createdGoals.delete(goal.id);
    const missing = await rejectionOf("get_goal", { goalId: goal.id });
    assert.match(missing ?? "", /was not found/, "a deleted goal is still readable");
    const remaining = await invoke(({ core }) => core.invoke("list_goals"));
    assert.equal(remaining.find((entry) => entry.id === goal.id), undefined, "deletion did not persist");
  });

  globalThis.after(async () => {
    // Best effort, in dependency order: Goals first because their links point at the cards. Every
    // call is allowed to fail -- a case that already removed its own fixture must not turn a
    // passing file into a failing one.
    for (const goalId of createdGoals) {
      await invoke(({ core }, id) => core.invoke("delete_goal", { goalId: id }), goalId).catch(() => {});
    }
    for (const workItemId of createdWorkItems) {
      // Archiving first because deletion refuses a live card (contexts/work_board/api.rs:173-184).
      await invoke(({ core }, id) => core.invoke("archive_work_item", { workItemId: id }), workItemId)
        .catch(() => {});
      await invoke(({ core }, id) => core.invoke("delete_work_item", { workItemId: id }), workItemId)
        .catch(() => {});
    }

    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // No `exit_application` here, and no UI navigation anywhere in this file, so there is nothing
    // to restore: every case goes straight at the command surface. Teardown is left to WDIO --
    // exiting the app from an after hook races its `deleteSession` and discards every per-test
    // result for the file.
  });
});
