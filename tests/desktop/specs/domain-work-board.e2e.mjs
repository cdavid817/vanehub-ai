import assert from "node:assert/strict";
import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";

import { readOnePieceApiKey } from "../helpers/onepiece-credential.mjs";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// Ids are run-scoped: `work_item_links` keys on (source_kind, source_id) alone
// (src-tauri/src/contexts/work_board/infrastructure.rs:30), so a fixed source id collides with
// itself the second time this spec runs against the same data directory.
const RUN = process.env.VANEHUB_TEST_RUN_ID ?? String(Date.now());

// A path string, never a directory on disk. The board stores `project_path` verbatim and the
// draft-side Plan commands only check that it is non-empty
// (src-tauri/src/contexts/task_orchestration/domain/model.rs:564-578) -- nothing on these code
// paths opens it. Rooted under the run's fixture directory anyway, so it cannot name a real
// repository even by accident.
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
const createdPlans = new Set();

/**
 * Runs a command that is expected to be rejected and returns the native error message.
 *
 * The rejection is caught inside the WebView rather than at the WDIO boundary. The direct-eval
 * bridge collapses any thrown value to `(e && e.message) || String(e)` and re-raises it as a
 * transport failure on this side, which makes "the command rejected as designed" indistinguishable
 * from "the bridge broke". Work board and Goals reject with `Result<_, String>` and the Plan
 * commands with `CommandError`, which serialises to a bare string
 * (src-tauri/src/commands/error.rs:89-96) -- both arrive here as a plain string.
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

/**
 * A Plan draft that satisfies both gates the draft side applies.
 *
 * `validate_plan_graph` (contexts/task_orchestration/domain/graph.rs:33-53) wants 1-10 SubTasks
 * with 1-3 acceptance criteria each and an acyclic dependency set; `validate_plan_execution_policy`
 * (domain/model.rs:456-545) wants every criterion bound to evidence and at least one *required*
 * validation command per SubTask plus one for the Plan. `maxAttemptsPerSubtask` is deliberately 2
 * rather than the serde default of 3 (domain/model.rs:387-404) so a mistyped key would show up as
 * a round-trip mismatch instead of silently defaulting.
 */
const planDraft = (planId, versionId, version, goal) => ({
  id: planId,
  versionId,
  version,
  goal,
  projectPath: PROJECT_PATH,
  baseRef: "main",
  plannerProfileId: null,
  discovery: { status: "complete", limitations: [] },
  executionPolicy: {
    maxAttemptsPerSubtask: 2,
    repairEligibleClasses: ["verification_failed"],
    finalValidationCommands: [{
      id: "final-check",
      program: "node",
      args: ["--version"],
      workingDirectory: null,
      timeoutSeconds: 300,
      required: true,
    }],
  },
  subtasks: [
    {
      id: "sweep-task-a",
      title: "Analyse",
      description: "Analyse the sweep fixture.",
      acceptanceCriteria: ["The analysis is recorded"],
      // `kind` is a snake_case enum: CriterionEvidenceKind (domain/model.rs:334-339).
      criterionEvidence: [{ criterionIndex: 0, kind: "automated", commandId: "verify-a" }],
      ordinal: 0,
      assignedRole: "worker",
      limits: { tokenBudget: 1000, toolCallLimit: 10, timeoutSeconds: 300 },
      validationCommands: [{
        id: "verify-a",
        program: "node",
        args: ["--version"],
        workingDirectory: null,
        timeoutSeconds: 300,
        required: true,
      }],
    },
    {
      id: "sweep-task-b",
      title: "Verify",
      description: "Verify the sweep fixture.",
      acceptanceCriteria: ["The verification is reviewed"],
      criterionEvidence: [{ criterionIndex: 0, kind: "manual", commandId: null }],
      ordinal: 1,
      assignedRole: "worker",
      limits: { tokenBudget: null, toolCallLimit: null, timeoutSeconds: null },
      validationCommands: [{
        id: "verify-b",
        program: "node",
        args: ["--version"],
        workingDirectory: null,
        timeoutSeconds: 300,
        required: true,
      }],
    },
  ],
  dependencies: [{ predecessorId: "sweep-task-a", successorId: "sweep-task-b" }],
});

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
    // A deliberately absent plan run: every real source in this database is already claimed by the
    // card `reconcile` (contexts/work_board/infrastructure.rs:119-131) minted for it, and the link
    // table's primary key is the source alone. An unresolvable source also exercises the projection
    // branch that reports a vanished source as unavailable rather than dropping the link
    // (contexts/work_board/infrastructure.rs:302-305).
    const sourceId = `desktop-sweep-missing-run-${RUN}`;
    const linked = await invoke(({ core }, input) => core.invoke("link_work_item_source", { input }), {
      workItemId: first.id,
      sourceKind: "plan_run",
      sourceId,
      relation: "execution",
    });
    const source = linked.sources.find((entry) => entry.sourceId === sourceId);
    assert.ok(source, "the source link was not persisted on the work item");
    assert.equal(source.sourceKind, "plan_run");
    assert.equal(source.relation, "execution");
    assert.equal(source.available, false);
    assert.equal(source.status, "unavailable");

    const duplicate = await rejectionOf("link_work_item_source", {
      input: { workItemId: second.id, sourceKind: "plan_run", sourceId, relation: "supporting" },
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

  globalThis.it("validates, versions and deletes a Plan draft", async () => {
    // This case must stay after the work board cases and must never call `list_work_items`:
    // `reconcile` (contexts/work_board/infrastructure.rs:165-192) mints a board card for every
    // row in `plans`, and a card minted for a draft deleted moments later is a leak the next
    // spec inherits.
    const planId = `desktop-sweep-plan-${RUN}`;
    const first = planDraft(planId, `${planId}-v1`, 1, "Sweep the draft-side Plan commands");

    // task_orchestration/plans.rs:8 -- `input: PlanDraft`, the same DTO `save_plan_draft` takes.
    // A valid graph resolves with nothing; the command returns `Result<(), CommandError>`.
    assert.equal(
      await invoke(({ core }, input) => core.invoke("validate_plan_draft", { input }), first),
      null,
      "a valid Plan draft was rejected",
    );
    const cyclic = await rejectionOf("validate_plan_draft", {
      input: {
        ...first,
        dependencies: [{ predecessorId: "sweep-task-a", successorId: "sweep-task-a" }],
      },
    });
    assert.match(cyclic ?? "", /cannot depend on itself/, "a self-dependency validated");

    // task_orchestration/save_plan_draft.rs:6 -- `input: PlanDraft`, which is `PlanVersion`
    // (contexts/task_orchestration/domain/model.rs:438-454 and the alias at :561).
    const saved = await invoke(({ core }, input) => core.invoke("save_plan_draft", { input }), first);
    createdPlans.add(planId);
    // Equality against the object that was sent, not a field spot-check: the fields carrying
    // `#[serde(default)]` (`discovery`, `executionPolicy`, `criterionEvidence`) absorb a mistyped
    // key silently, and only a whole-shape comparison notices.
    assert.deepEqual(saved, first, "save_plan_draft did not echo the draft it was given");

    // task_orchestration/get_plan_draft.rs:6 -- `plan_id: String`, exposed as `planId`.
    const fetched = await invoke(
      ({ core }, id) => core.invoke("get_plan_draft", { planId: id }),
      planId,
    );
    assert.deepEqual(fetched, first, "the stored draft did not survive a database round trip");

    // A second version needs its own `versionId`: `save_draft` deletes and reinserts the row with
    // the id it is handed (infrastructure/repository.rs:101, :766-777), so reusing one replaces the
    // earlier version instead of adding to it.
    const second = planDraft(planId, `${planId}-v2`, 2, "Sweep the draft-side Plan commands, revised");
    await invoke(({ core }, input) => core.invoke("save_plan_draft", { input }), second);
    assert.deepEqual(
      await invoke(({ core }, id) => core.invoke("get_plan_draft", { planId: id }), planId),
      second,
      "the current version did not advance to the newer draft",
    );

    // task_orchestration/plans.rs:16 -- `plan_id`, newest version first.
    const versions = await invoke(
      ({ core }, id) => core.invoke("list_plan_versions", { planId: id }),
      planId,
    );
    assert.deepEqual(versions.map((entry) => entry.version), [2, 1]);
    assert.deepEqual(versions[0], second);
    assert.deepEqual(versions[1], first);

    // Saving behind the current version is a conflict, not a silent rollback
    // (infrastructure/repository.rs:85-88, mapped at commands/error.rs:286-289).
    const stale = await rejectionOf("save_plan_draft", {
      input: planDraft(planId, `${planId}-v1-stale`, 1, "A stale revision"),
    });
    assert.match(stale ?? "", /Plan state changed/, "a stale Plan version overwrote a newer one");

    // work_board/commands.rs:70 -- `list_plan_summaries` lives in the work board module because it
    // is what the board reads to project Plan cards (contexts/work_board/models.rs:80-91).
    const summaries = await invoke(({ core }) => core.invoke("list_plan_summaries"));
    const summary = summaries.find((entry) => entry.id === planId);
    assert.ok(summary, "the saved Plan is missing from the summary projection");
    assert.equal(summary.status, "draft");
    assert.equal(summary.goal, second.goal, "the summary showed a stale version's goal");
    assert.equal(summary.projectPath, PROJECT_PATH);
    assert.equal(summary.latestRunId, null, "an unapproved draft reported a run");

    // task_orchestration/plans.rs:24 -- deletion is scoped to plans still in `draft`
    // (infrastructure/repository.rs:175-187).
    await invoke(({ core }, id) => core.invoke("delete_plan_draft", { planId: id }), planId);
    createdPlans.delete(planId);
    assert.equal(
      await invoke(({ core }, id) => core.invoke("get_plan_draft", { planId: id }), planId),
      null,
      "a deleted Plan draft is still readable",
    );
    assert.deepEqual(
      await invoke(({ core }, id) => core.invoke("list_plan_versions", { planId: id }), planId),
      [],
      "the deleted Plan left its versions behind",
    );
    const gone = await invoke(({ core }) => core.invoke("list_plan_summaries"));
    assert.equal(gone.find((entry) => entry.id === planId), undefined, "deletion did not persist");
    const twice = await rejectionOf("delete_plan_draft", { planId });
    assert.match(twice ?? "", /Plan state changed/, "deleting an absent Plan reported success");
  });

  globalThis.it("generates a Plan draft from the planner over the live provider", async function generateDraft() {
    // `generate_plan_draft` is the one draft-side command that is not a pure database operation:
    // it builds a planner prompt and runs a real OnePiece turn, so it needs an active provider
    // profile and bills a generation. A host without a key reports BLOCKED rather than failing.
    const apiKey = readOnePieceApiKey();
    if (!apiKey) {
      blocked.push("generate_plan_draft: set VANEHUB_ONEPIECE_API_KEY or VANEHUB_ONEPIECE_PROFILE_ID");
      this.skip();
    }

    const profiles = await invoke(({ core }, input) => core.invoke("save_onepiece_provider_profile", { input }), {
      id: null,
      name: `Plan planner ${RUN}`,
      providerId: "deepseek",
      endpointType: "openai-chat-completions",
      modelId: "deepseek-v4-flash",
      apiKey,
    });
    const profile = profiles.profiles.find((entry) => entry.name === `Plan planner ${RUN}`);

    try {
      // A real directory, unlike `PROJECT_PATH`, which every other case in this file uses as an
      // opaque string because the draft-side commands only check it is non-empty. The planner is
      // the exception: it canonicalizes the path
      // (workspaces/infrastructure/filesystem.rs:39-44) and answers "Project unavailable" for one
      // that is not on disk.
      const plannerProject = await mkdtemp(join(tmpdir(), "vanehub-planner-"));
      // `generate_plan_draft` is synchronous: it runs the whole planner turn inline rather than
      // returning an operation to poll, so this one `execute` call has to outlast a model round
      // trip. WebDriver's script timeout defaults to thirty seconds and the turn has been measured
      // either side of that on this host, so it is raised for this call and put back afterwards --
      // leaving it raised would blunt every other spec's detection of a genuine hang.
      await globalThis.browser.setTimeout({ script: 180_000 });
      let draft;
      try {
        draft = await invoke(({ core }, input) => core.invoke("generate_plan_draft", { input }), {
          planId: null,
          version: 1,
          // Deliberately small and concrete. The assertion is that the planner returns a
          // structured draft this side can persist, not that the model decomposed a hard problem
          // well -- the latter is not a property a test can hold a provider to.
          goal: "Add a CHANGELOG.md file with a single Unreleased heading.",
          projectPath: plannerProject,
          baseRef: "main",
          availableTools: ["read_file", "write_file"],
        });
      } catch (error) {
        // Reported rather than failed: a turn that runs past even the raised ceiling is the
        // provider being slow, which is not a defect this suite can hold it to.
        if (/timed out/i.test(String(error?.message ?? error))) {
          blocked.push("generate_plan_draft: the planner turn ran past 180s on this host");
          this.skip();
        }
        throw error;
      } finally {
        await globalThis.browser.setTimeout({ script: 30_000 });
      }

      assert.ok(draft, "the planner returned no draft");
      // `PlanDraft` is an alias of `PlanVersion` (domain/model.rs:561), whose identifier is `id`;
      // `planId` is only the *argument* name the read command takes.
      assert.ok(draft.id, "the generated draft carried no plan id");
      createdPlans.add(draft.id);
      assert.ok(Array.isArray(draft.subtasks), "the generated draft carried no subtask list");
      assert.ok(draft.subtasks.length > 0, "the planner produced a draft with no subtasks at all");
      // Every subtask has to be addressable and ordered, because the run side schedules them by
      // exactly these two fields; a draft that generated but could not be run would otherwise pass.
      for (const subtask of draft.subtasks) {
        assert.ok(subtask.id, `a generated subtask carried no id: ${JSON.stringify(subtask)}`);
        assert.ok(subtask.title, `subtask ${subtask.id} carried no title`);
      }
      const ids = draft.subtasks.map((subtask) => subtask.id);
      assert.equal(new Set(ids).size, ids.length, "the planner produced duplicate subtask ids");

      // The generated draft has to survive the same read path the Plan centre opens it through --
      // generating into a shape the reader cannot load is the failure worth catching here.
      const reloaded = await invoke(
        ({ core }, planId) => core.invoke("get_plan_draft", { planId }),
        draft.id,
      );
      assert.equal(reloaded.id, draft.id, "the generated draft did not read back");
      assert.equal(
        reloaded.subtasks.length,
        draft.subtasks.length,
        "the reloaded draft lost subtasks the planner produced",
      );
    } finally {
      if (profile?.id) {
        await invoke(({ core }, profileId) => core.invoke("delete_onepiece_provider_profile", {
          profileId,
        }), profile.id).catch(() => {});
      }
    }
  });

  globalThis.after(async () => {
    // Best effort, in dependency order: Goals first (their links point at the cards), then Plans,
    // then the cards themselves. Every call is allowed to fail -- a case that already removed its
    // own fixture must not turn a passing file into a failing one.
    for (const goalId of createdGoals) {
      await invoke(({ core }, id) => core.invoke("delete_goal", { goalId: id }), goalId).catch(() => {});
    }
    for (const planId of createdPlans) {
      await invoke(({ core }, id) => core.invoke("delete_plan_draft", { planId: id }), planId)
        .catch(() => {});
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
