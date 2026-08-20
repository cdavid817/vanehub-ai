import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// Run-scoped so a second run against the same data directory does not collide with its own
// leftovers: the memory pool is keyed by file name.
const RUN = process.env.VANEHUB_TEST_RUN_ID ?? String(Date.now());

// A Prompt Hook id is 3..=64 characters of [a-z0-9-] starting on a letter or digit
// (src-tauri/src/contexts/tooling/prompt_hooks/domain/identity.rs:9-16).
const LIFECYCLE_HOOK = "desktop-sweep-lifecycle-hook";
const RENDER_HOOK = "desktop-sweep-render-hook";
// `law-runtime-boundary` is the one built-in that is neither disableable nor deletable
// (src-tauri/src/contexts/tooling/prompt_hooks/domain/catalog.rs:28-38).
const IMMUTABLE_HOOK = "law-runtime-boundary";
// (stage, category, order) is a unique slot across every hook
// (src-tauri/src/contexts/tooling/prompt_hooks/domain/ordering.rs:42-51). The seven built-ins take
// orders 100-700 (domain/catalog.rs:16-96), so these two stay free of them and of each other.
const LIFECYCLE_ORDER = 9410;
const RENDER_ORDER = 9411;
// Only `agent_id`, `agent_name`, `current_time`, `sample_input`, `session_id` and the two legacy
// aliases `agentId`/`sampleInput` render; anything else fails publish validation
// (src-tauri/src/contexts/tooling/prompt_hooks/domain/template.rs:113-118).
const BODY_V1 = "Desktop sweep v1 for {{agentId}} :: {{sampleInput}}";
const BODY_V2 = "Desktop sweep v2 for {{agent_name}} :: {{sample_input}}";
const RENDER_BODY = "Desktop sweep render {{agent_name}} / {{agentId}} :: {{sampleInput}}";

// ---------------------------------------------------------------------------------------------
// Command shapes. Every argument name below is the camelCase form Tauri derives from the Rust
// parameter or the `#[serde(rename_all = "camelCase")]` DTO field, taken from the cited file.
// ---------------------------------------------------------------------------------------------

// list_prompt_hooks.rs:8-10 — no arguments; returns `{ hooks, stats }` (prompt_hooks/dto.rs:57-71).
const listHooks = () => invoke(({ core }) => core.invoke("list_prompt_hooks"));
// create_prompt_hook.rs:8-11 — `input: PromptHookMutationInput` (prompt_hooks/dto.rs:73-86).
const createHook = (input) => invoke(({ core }, value) => (
  core.invoke("create_prompt_hook", { input: value })
), input);
// update_prompt_hook.rs:8-12 — `hook_id` plus `input: PromptHookUpdateInput`
// (prompt_hooks/dto.rs:88-102): the mutation shape with an extra `version`.
const updateHook = (hookId, input) => invoke(({ core }, args) => (
  core.invoke("update_prompt_hook", args)
), { hookId, input });
// save_prompt_hook_draft.rs:8-11 — `input: SavePromptHookDraftInput` (prompt_hooks/dto.rs:145-151):
// `{ hookId, expectedRevision, draft }`, where `draft` is a nested `PromptHookMutationInput`
// (prompt_hooks/dto.rs:73-86) — id, name, description, category, stage, order, templateBody,
// enabled, cliBindings, governance. It is not a partial patch: every field must be present.
const saveDraft = (input) => invoke(({ core }, value) => (
  core.invoke("save_prompt_hook_draft", { input: value })
), input);
// publish_prompt_hook.rs:8-11 — `input: PublishPromptHookInput` (prompt_hooks/dto.rs:153-159).
const publishHook = (input) => invoke(({ core }, value) => (
  core.invoke("publish_prompt_hook", { input: value })
), input);
// rollback_prompt_hook.rs:8-11 — `input: RollbackPromptHookInput` (prompt_hooks/dto.rs:161-167).
const rollbackHook = (input) => invoke(({ core }, value) => (
  core.invoke("rollback_prompt_hook", { input: value })
), input);
// get_prompt_hook_version_history.rs:8-11 — `hook_id: String`.
const readHistory = (hookId) => invoke(({ core }, id) => (
  core.invoke("get_prompt_hook_version_history", { hookId: id })
), hookId);
// delete_prompt_hook.rs:7-10 — `hook_id: String`.
const deleteHook = (hookId) => invoke(({ core }, id) => (
  core.invoke("delete_prompt_hook", { hookId: id })
), hookId);
// preview_prompt_hook.rs:8-11 — `input: PromptHookPreviewInput` (prompt_hooks/dto.rs:104-110);
// `sampleInput` is optional there.
const previewHook = (input) => invoke(({ core }, value) => (
  core.invoke("preview_prompt_hook", { input: value })
), input);
// preview_prompt_assembly.rs:8-11 — `input: PromptAssemblyPreviewInput`
// (prompt_hooks/dto.rs:112-117); `sampleInput` is a required String here, not an Option.
const previewAssembly = (input) => invoke(({ core }, value) => (
  core.invoke("preview_prompt_assembly", { input: value })
), input);
// set_prompt_hook_enabled.rs:8-12 — `hook_id`, `enabled`.
const setEnabled = (hookId, enabled) => invoke(({ core }, args) => (
  core.invoke("set_prompt_hook_enabled", args)
), { hookId, enabled });
// set_prompt_hook_cli_bindings.rs:8-12 — `hook_id`, `agent_ids`; each id must be one of the five
// managed CLI ids (prompt_hooks/domain/binding.rs:22-37).
const setBindings = (hookId, agentIds) => invoke(({ core }, args) => (
  core.invoke("set_prompt_hook_cli_bindings", args)
), { hookId, agentIds });
// list_prompt_hook_traces.rs:8-11 — `limit: Option<i64>`, clamped to 1..=100
// (prompt_hooks/application/service.rs:126-131).
const listTraces = (limit) => invoke(({ core }, value) => (
  core.invoke("list_prompt_hook_traces", { limit: value })
), limit);
// list_prompt_hook_variables.rs:7-10 — no arguments, and no `Result` wrapper.
const listVariables = () => invoke(({ core }) => core.invoke("list_prompt_hook_variables"));

// agent_runtime/expert_roles/list_expert_roles.rs:7-9 — no arguments.
const listRoles = () => invoke(({ core }) => core.invoke("list_expert_roles"));
// agent_runtime/expert_roles/save_expert_role.rs:7-10 — `input: SaveExpertRoleInput`
// (commands/agent_runtime/dto.rs:738-750): `{ id, displayName, avatar, color, responsibility,
// instruction, skillIds, reviewPolicy: { peerReviewer, requireDifferentFamily },
// preferredProviders }`, with `id: null` meaning "create".
const saveRole = (input) => invoke(({ core }, value) => (
  core.invoke("save_expert_role", { input: value })
), input);
// agent_runtime/expert_roles/delete_expert_role.rs:6-9 — `role_id: String`.
const deleteRole = (roleId) => invoke(({ core }, id) => (
  core.invoke("delete_expert_role", { roleId: id })
), roleId);

// desktop/get_settings.rs:7-9 — no arguments; returns `AppSettings` (desktop/dto.rs:23-40).
const readSettings = () => invoke(({ core }) => core.invoke("get_settings"));
// desktop/save_setting.rs:7-11 — `input: SaveSettingInput` (desktop/dto.rs:42-47): `{ key, value }`.
// `value` must be a JSON boolean or string; a JSON number is accepted for
// `contextQualityRetentionDays` alone (desktop/mapper.rs:14-28).
const saveSetting = (key, value) => invoke(({ core }, input) => (
  core.invoke("save_setting", { input })
), { key, value });
// agent_runtime/list_agent_memories.rs:7-9 — no arguments.
const listMemories = () => invoke(({ core }) => core.invoke("list_agent_memories"));
// agent_runtime/delete_agent_memory.rs:46-52 — `memory_id: String`.
const deleteMemory = (memoryId) => invoke(({ core }, id) => (
  core.invoke("delete_agent_memory", { memoryId: id })
), memoryId);

function mutation(id, order, templateBody, overrides = {}) {
  return {
    id,
    name: "Desktop sweep hook",
    description: "Created by the desktop domain sweep.",
    // `category` and `stage` are kebab-case enums, not camelCase
    // (src-tauri/src/commands/tooling/prompt_hooks/dto.rs:3-20).
    category: "dynamic",
    stage: "per-turn",
    order,
    templateBody,
    enabled: true,
    cliBindings: ["codex-cli"],
    // Three free-form tiers, camelCase and unvalidated (prompt_hooks/dto.rs:29-35); these values
    // are the ones the transport contract test pins (commands/tooling/prompt_hooks/mapper.rs:
    // 366-370).
    governance: {
      safetyTier: "editable",
      transparencyTier: "visible-by-default",
      governanceTier: "human-gated",
    },
    ...overrides,
  };
}

function roleInput(overrides = {}) {
  return {
    id: null,
    displayName: "Desktop sweep reviewer",
    avatar: "🔍",
    // Exactly `#` plus six hex digits (agent_runtime/domain/expert_role.rs:122-128).
    color: "#3366CC",
    responsibility: "Reviews the desktop sweep's own changes.",
    instruction: "Review the change and report what you checked.",
    skillIds: ["desktop-sweep-skill"],
    reviewPolicy: { peerReviewer: true, requireDifferentFamily: true },
    preferredProviders: ["anthropic", "deepseek"],
    ...overrides,
  };
}

async function readHook(hookId) {
  const { hooks } = await listHooks();
  return hooks.find((hook) => hook.id === hookId) ?? null;
}

async function removeHookQuietly(hookId) {
  await deleteHook(hookId).catch(() => {});
}

/**
 * Asserts a call fails and hands back its message.
 *
 * The `Command ... not found` check is the part that earns its keep: a typo'd command name rejects
 * too, so a bare `assert.rejects` would let a negative case pass while proving nothing about the
 * guard it claims to cover. Everything past that is asserted by reading the state back instead of
 * by matching the message, which crosses the WebDriver boundary in an unspecified wrapper.
 */
async function rejects(run, description) {
  try {
    await run();
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    assert.doesNotMatch(
      message,
      /Command \w+ not found/,
      `${description}: rejected because the command is not registered, not by the guard`,
    );
    return message;
  }
  assert.fail(`${description}: the call was expected to fail and it succeeded`);
}

globalThis.describe("VaneHub AI desktop Prompt Hooks, Expert Roles and Personalization", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("publishes the Prompt Hook variable catalogue the editor renders", async () => {
    const variables = await listVariables();
    assert.deepEqual(
      variables.map((variable) => variable.name).sort(),
      ["agent_id", "agent_name", "current_time", "sample_input", "session_id"],
    );
    for (const variable of variables) {
      assert.equal(variable.token, `{{${variable.name}}}`, `${variable.name} published a wrong token`);
      assert.ok(variable.descriptionKey.startsWith("promptHooks.variables."), "no i18n description key");
      assert.ok(variable.example.length > 0, `${variable.name} published no example`);
    }
    // The two legacy aliases are the reason older templates still render
    // (prompt_hooks/application/service.rs:143-147).
    const aliases = Object.fromEntries(variables.map((item) => [item.name, item.aliases]));
    assert.deepEqual(aliases.agent_id, ["agentId"]);
    assert.deepEqual(aliases.sample_input, ["sampleInput"]);
    assert.deepEqual(aliases.session_id, []);
  });

  globalThis.it("takes a user Prompt Hook through draft, publish, edit, rollback and delete", async () => {
    await removeHookQuietly(LIFECYCLE_HOOK);
    const created = await createHook(mutation(LIFECYCLE_HOOK, LIFECYCLE_ORDER, BODY_V1));
    try {
      // A new user hook is unpublished and disabled whatever the input asked for: the requested
      // `enabled` lives in the seeded draft until a publish promotes it
      // (prompt_hooks/application/service.rs:317-344).
      assert.equal(created.version, 0);
      assert.equal(created.enabled, false);
      assert.equal(created.source, "user");
      assert.equal(created.disableable, true);

      const seeded = await readHistory(LIFECYCLE_HOOK);
      assert.equal(seeded.publishedVersion, null);
      assert.equal(seeded.draft.revision, 1);
      assert.equal(seeded.draft.input.templateBody, BODY_V1);
      assert.deepEqual(seeded.versions, []);

      // `expectedPublishedVersion` is null while the record sits at version 0: the guard compares
      // against `(version > 0).then_some(version)`
      // (prompt_hooks/infrastructure/sqlite_repository.rs:775-798).
      const first = await publishHook({
        hookId: LIFECYCLE_HOOK,
        expectedDraftRevision: 1,
        expectedPublishedVersion: null,
      });
      assert.equal(first.version, 1);
      assert.equal(first.publicationKind, "publish");
      assert.equal(first.rollbackFromVersion, null);

      const published = await readHook(LIFECYCLE_HOOK);
      assert.equal(published.version, 1);
      assert.equal(published.templateBody, BODY_V1);
      // Publishing promotes the draft's own `enabled` onto the record
      // (prompt_hooks/infrastructure/sqlite_repository.rs:743-773).
      assert.equal(published.enabled, true);

      // `expectedRevision` is null because publishing deleted the draft row
      // (prompt_hooks/infrastructure/sqlite_repository.rs:365-373), so the next draft starts at
      // revision 1 again (prompt_hooks/application/service.rs:176-182).
      const draft = await saveDraft({
        hookId: LIFECYCLE_HOOK,
        expectedRevision: null,
        draft: mutation(LIFECYCLE_HOOK, LIFECYCLE_ORDER, BODY_V2, { name: "Desktop sweep hook v2" }),
      });
      assert.equal(draft.revision, 1);
      assert.equal(draft.input.templateBody, BODY_V2);

      // The published version has to survive the edit: an unpublished draft must never reach the
      // record the runtime assembles prompts from.
      const duringDraft = await readHook(LIFECYCLE_HOOK);
      assert.equal(duringDraft.version, 1, "a draft edit moved the published version");
      assert.equal(duringDraft.templateBody, BODY_V1, "a draft edit leaked into the published content");
      assert.equal(duringDraft.name, "Desktop sweep hook");
      const drafted = await readHistory(LIFECYCLE_HOOK);
      assert.equal(drafted.publishedVersion, 1);
      assert.equal(drafted.draft.input.templateBody, BODY_V2);
      assert.deepEqual(drafted.versions.map((version) => version.version), [1]);

      // Saving again with the same `expectedRevision: null` is a stale write now that the row sits
      // at revision 1 (prompt_hooks/application/service.rs:168-174). It must change nothing.
      await rejects(() => saveDraft({
        hookId: LIFECYCLE_HOOK,
        expectedRevision: null,
        draft: mutation(LIFECYCLE_HOOK, LIFECYCLE_ORDER, BODY_V2, { name: "Desktop sweep hook stale" }),
      }), "a stale draft revision");
      const afterConflict = await readHistory(LIFECYCLE_HOOK);
      assert.equal(afterConflict.draft.revision, 1, "the refused draft write still bumped the revision");
      assert.equal(afterConflict.draft.input.name, "Desktop sweep hook v2");

      const second = await publishHook({
        hookId: LIFECYCLE_HOOK,
        expectedDraftRevision: 1,
        expectedPublishedVersion: 1,
      });
      assert.equal(second.version, 2);
      const republished = await readHook(LIFECYCLE_HOOK);
      assert.equal(republished.version, 2);
      assert.equal(republished.templateBody, BODY_V2);
      assert.equal(republished.name, "Desktop sweep hook v2");

      // A rollback does not rewind the counter; it republishes the target snapshot as a new
      // version (prompt_hooks/application/service.rs:282-292).
      const rolled = await rollbackHook({
        hookId: LIFECYCLE_HOOK,
        version: 1,
        expectedPublishedVersion: 2,
      });
      assert.equal(rolled.version, 3);
      assert.equal(rolled.publicationKind, "rollback");
      assert.equal(rolled.rollbackFromVersion, 1);

      const restored = await readHook(LIFECYCLE_HOOK);
      assert.equal(restored.templateBody, BODY_V1, "the rollback did not restore the earlier content");
      assert.equal(restored.name, "Desktop sweep hook");
      assert.equal(restored.version, 3);
      const rolledHistory = await readHistory(LIFECYCLE_HOOK);
      assert.equal(rolledHistory.publishedVersion, 3);
      assert.equal(rolledHistory.draft, null);
      assert.deepEqual(rolledHistory.versions.map((version) => version.version), [3, 2, 1]);
      assert.deepEqual(
        rolledHistory.versions.map((version) => version.publicationKind),
        ["rollback", "publish", "publish"],
      );
      // Rolling back to a version that was never published must not invent one.
      await rejects(() => rollbackHook({
        hookId: LIFECYCLE_HOOK,
        version: 99,
        expectedPublishedVersion: 3,
      }), "a rollback to an unknown version");
      assert.equal((await readHook(LIFECYCLE_HOOK)).version, 3);

      // `update_prompt_hook` is the second write path into the same draft slot: it stores a draft
      // and returns the record *unchanged* (prompt_hooks/application/service.rs:347-375), so it is
      // a second, independent proof that the published content survives an edit.
      const returned = await updateHook(LIFECYCLE_HOOK, {
        ...mutation(LIFECYCLE_HOOK, LIFECYCLE_ORDER, BODY_V2, { name: "Desktop sweep hook v3" }),
        version: 3,
      });
      assert.equal(returned.templateBody, BODY_V1);
      const afterUpdate = await readHook(LIFECYCLE_HOOK);
      assert.equal(afterUpdate.version, 3);
      assert.equal(afterUpdate.templateBody, BODY_V1);
      const updated = await readHistory(LIFECYCLE_HOOK);
      assert.equal(updated.draft.revision, 1);
      assert.equal(updated.draft.input.name, "Desktop sweep hook v3");

      await deleteHook(LIFECYCLE_HOOK);
      assert.equal(await readHook(LIFECYCLE_HOOK), null, "the deleted hook is still in the registry");
      // Drafts and versions go with it (prompt_hooks/infrastructure/sqlite_repository.rs:85-124),
      // so the history read has nothing left to resolve.
      await rejects(() => readHistory(LIFECYCLE_HOOK), "history for a deleted hook");
    } finally {
      await removeHookQuietly(LIFECYCLE_HOOK);
    }
  });

  globalThis.it("renders a published hook through preview, assembly and the trace log", async () => {
    await removeHookQuietly(RENDER_HOOK);
    await createHook(mutation(RENDER_HOOK, RENDER_ORDER, RENDER_BODY));
    try {
      await publishHook({
        hookId: RENDER_HOOK,
        expectedDraftRevision: 1,
        expectedPublishedVersion: null,
      });

      const preview = await previewHook({
        hookId: RENDER_HOOK,
        agentId: "codex-cli",
        sampleInput: "Desktop sweep sample",
      });
      assert.equal(preview.hookId, RENDER_HOOK);
      assert.equal(preview.agentId, "codex-cli");
      assert.match(preview.renderedContent, /Codex CLI \/ codex-cli/);
      assert.match(preview.renderedContent, /Desktop sweep sample/);
      assert.equal(preview.trace[0].status, "fired");
      assert.equal(preview.trace[0].version, 1);
      assert.equal(preview.trace[0].stage, "per-turn");
      assert.equal(preview.trace[0].category, "dynamic");

      // Assembly reports no single hook id (prompt_hooks/mapper.rs:151-161) and folds every
      // eligible hook plus the user prompt into one string.
      const assembly = await previewAssembly({
        agentId: "codex-cli",
        sampleInput: "Desktop sweep assembly",
      });
      assert.equal(assembly.hookId, null);
      assert.match(assembly.renderedContent, /Desktop sweep render/);
      assert.match(assembly.renderedContent, /Desktop sweep assembly/);
      const fired = assembly.trace.find((entry) => entry.hookId === RENDER_HOOK);
      assert.equal(fired.status, "fired");
      assert.equal(fired.reason, null);

      const rebound = await setBindings(RENDER_HOOK, ["claude-code", "opencode"]);
      assert.deepEqual(rebound.cliBindings, ["claude-code", "opencode"]);
      assert.deepEqual((await readHook(RENDER_HOOK)).cliBindings, ["claude-code", "opencode"]);
      // Unbound from codex-cli the hook has to be skipped with a reason, not silently included.
      const unbound = await previewAssembly({
        agentId: "codex-cli",
        sampleInput: "Desktop sweep unbound",
      });
      const skipped = unbound.trace.find((entry) => entry.hookId === RENDER_HOOK);
      assert.equal(skipped.status, "skipped");
      assert.equal(skipped.reason, "unbound-cli");
      assert.doesNotMatch(unbound.renderedContent, /Desktop sweep render/);

      await rejects(
        () => setBindings(RENDER_HOOK, ["not-a-managed-cli"]),
        "an unknown CLI binding",
      );
      assert.deepEqual(
        (await readHook(RENDER_HOOK)).cliBindings,
        ["claude-code", "opencode"],
        "the refused binding write still landed",
      );

      await setBindings(RENDER_HOOK, ["codex-cli"]);
      const disabled = await setEnabled(RENDER_HOOK, false);
      assert.equal(disabled.enabled, false);
      assert.equal((await readHook(RENDER_HOOK)).enabled, false);
      const off = await previewAssembly({
        agentId: "codex-cli",
        sampleInput: "Desktop sweep disabled",
      });
      const offTrace = off.trace.find((entry) => entry.hookId === RENDER_HOOK);
      assert.equal(offTrace.status, "disabled");
      assert.equal(offTrace.reason, "disabled");
      assert.equal((await setEnabled(RENDER_HOOK, true)).enabled, true);

      // The trace store is a 50-row ring buffer written by preview and assembly alike
      // (prompt_hooks/application/service.rs:21, :469, :545).
      const traces = await listTraces(50);
      assert.ok(traces.length > 0, "the trace log is empty after a preview and three assemblies");
      assert.ok(
        traces.some((entry) => entry.hookId === RENDER_HOOK && entry.agentId === "codex-cli"),
        "no trace was recorded for the previewed hook",
      );

      const { hooks, stats } = await listHooks();
      assert.equal(stats.total, hooks.length);
      assert.equal(stats.builtin, 7, "the built-in Prompt Hook catalogue changed size");
      assert.equal(stats.user, hooks.filter((hook) => hook.source === "user").length);

      await deleteHook(RENDER_HOOK);
      assert.equal(await readHook(RENDER_HOOK), null);
    } finally {
      await removeHookQuietly(RENDER_HOOK);
    }
  });

  globalThis.it("refuses every mutation of a built-in Prompt Hook", async () => {
    const before = await readHook(IMMUTABLE_HOOK);
    assert.ok(before, `${IMMUTABLE_HOOK} is missing from the built-in catalogue`);
    assert.equal(before.source, "builtin");
    assert.equal(before.disableable, false);

    await rejects(() => deleteHook(IMMUTABLE_HOOK), "deleting a built-in hook");
    await rejects(() => setEnabled(IMMUTABLE_HOOK, false), "disabling a non-disableable hook");
    await rejects(() => saveDraft({
      hookId: IMMUTABLE_HOOK,
      expectedRevision: null,
      draft: mutation(IMMUTABLE_HOOK, 200, "Overwritten by the desktop sweep", {
        category: "law",
        stage: "session-init",
      }),
    }), "drafting over built-in content");

    // Read the whole record back: the guards must be refusals, not partial writes.
    assert.deepEqual(await readHook(IMMUTABLE_HOOK), before);
    // A built-in reports a synthesised single-version history with no draft
    // (prompt_hooks/application/service.rs:236-253).
    const history = await readHistory(IMMUTABLE_HOOK);
    assert.equal(history.publishedVersion, 1);
    assert.equal(history.draft, null);
    assert.deepEqual(history.versions.map((version) => version.version), [1]);
  });

  globalThis.it("takes an expert role through create, edit and delete beside the built-ins", async () => {
    const builtinIds = ["builtin-architect", "builtin-implementer", "builtin-reviewer"];
    const initial = await listRoles();
    const architect = initial.find((role) => role.id === "builtin-architect");
    for (const id of builtinIds) {
      const role = initial.find((item) => item.id === id);
      assert.ok(role, `built-in expert role ${id} is missing`);
      assert.equal(role.origin, "builtin");
    }

    const created = await saveRole(roleInput());
    let roleId = created.id;
    try {
      assert.equal(created.origin, "user");
      assert.ok(created.id.startsWith("expert-role-"), `unexpected generated id: ${created.id}`);

      const stored = (await listRoles()).find((role) => role.id === roleId);
      assert.ok(stored, "the created expert role was not persisted");
      assert.equal(stored.displayName, "Desktop sweep reviewer");
      assert.deepEqual(stored.skillIds, ["desktop-sweep-skill"]);
      assert.deepEqual(stored.preferredProviders, ["anthropic", "deepseek"]);
      assert.deepEqual(stored.reviewPolicy, { peerReviewer: true, requireDifferentFamily: true });

      // Editing keeps `createdAt` and moves `updatedAt` (agent_runtime/application/expert_role.rs:
      // 60-76). Both are `chrono::Utc::now().to_rfc3339()`, so they are compared as instants
      // rather than as strings: the fractional-digit count is not fixed.
      const edited = await saveRole(roleInput({
        id: roleId,
        displayName: "Desktop sweep reviewer (edited)",
        skillIds: [],
        reviewPolicy: { peerReviewer: false, requireDifferentFamily: false },
      }));
      assert.equal(edited.id, roleId);
      assert.equal(edited.createdAt, stored.createdAt, "editing a role rewrote its creation time");
      assert.ok(Date.parse(edited.updatedAt) >= Date.parse(stored.updatedAt));
      const reread = (await listRoles()).find((role) => role.id === roleId);
      assert.equal(reread.displayName, "Desktop sweep reviewer (edited)");
      assert.deepEqual(reread.skillIds, []);
      assert.deepEqual(reread.reviewPolicy, { peerReviewer: false, requireDifferentFamily: false });

      // Validation mirrors the frontend's own `validateExpertRoleInput`
      // (agent_runtime/domain/expert_role.rs:66-109); each refusal is confirmed by re-reading.
      await rejects(() => saveRole(roleInput({ id: roleId, color: "#12345" })), "a non-hex colour");
      await rejects(() => saveRole(roleInput({ id: roleId, displayName: "   " })), "a blank display name");
      await rejects(
        () => saveRole(roleInput({ id: roleId, skillIds: ["duplicate", "duplicate"] })),
        "a repeated Skill id",
      );
      await rejects(
        () => saveRole(roleInput({
          id: roleId,
          reviewPolicy: { peerReviewer: false, requireDifferentFamily: true },
        })),
        "requiring a different model family without peer review",
      );
      assert.deepEqual((await listRoles()).find((role) => role.id === roleId), reread);

      // A built-in is neither editable nor deletable
      // (agent_runtime/application/expert_role.rs:84-98).
      await rejects(
        () => saveRole(roleInput({ id: "builtin-architect", displayName: "Hijacked" })),
        "editing a built-in expert role",
      );
      await rejects(() => deleteRole("builtin-architect"), "deleting a built-in expert role");
      assert.deepEqual(
        (await listRoles()).find((role) => role.id === "builtin-architect"),
        architect,
        "a refused built-in edit still changed the role",
      );

      await deleteRole(roleId);
      roleId = null;
      const remaining = await listRoles();
      assert.equal(remaining.find((role) => role.id === created.id), undefined, "the role survived its delete");
      assert.deepEqual(
        remaining.filter((role) => role.origin === "builtin").map((role) => role.id),
        builtinIds,
      );
    } finally {
      if (roleId) {
        await deleteRole(roleId).catch(() => {});
      }
    }
  });

  globalThis.it("round-trips the personalization settings the runtime reads at generation time", async () => {
    const before = await readSettings();
    assert.equal(typeof before.automaticContextCompactionEnabled, "boolean");
    assert.ok(
      [7, 30, 90].includes(before.contextQualityRetentionDays),
      `unexpected retention window: ${before.contextQualityRetentionDays}`,
    );
    // These five were write-only when this spec was written: they parsed as mutations and
    // persisted, but `AppSettings` had no field for them, so the response omitted them and the
    // frontend normalizer replaced them with its own defaults -- the Personalization page showed
    // an empty box over a stored instruction. That is D-09, fixed in 39bcf278 by carrying them
    // through the DTO and mapper, which landed after this case was first written and left its
    // BLOCKED note behind. They are asserted as a real round trip now, which is the assertion that
    // would have caught the defect and is what stops it coming back.
    assert.equal(typeof before.customInstructionsAboutUser, "string");
    assert.equal(typeof before.customInstructionsStyleRules, "string");
    for (const key of ["customInstructionsEnabled", "memoryEnabled", "memoryToolAssistedChatsEnabled"]) {
      assert.equal(typeof before[key], "boolean", `get_settings omitted ${key}`);
    }

    try {
      await saveSetting("automaticContextCompactionEnabled", !before.automaticContextCompactionEnabled);
      const toggled = await readSettings();
      assert.equal(
        toggled.automaticContextCompactionEnabled,
        !before.automaticContextCompactionEnabled,
        "the compaction toggle did not survive a read",
      );

      // 90 deliberately, never a shorter window: this drives context-quality retention, and a
      // temporarily shorter one could prune assessments another spec in this run depends on.
      await saveSetting("contextQualityRetentionDays", 90);
      assert.equal((await readSettings()).contextQualityRetentionDays, 90);
      // The accepted set is closed to 7, 30 and 90 (desktop/domain/settings.rs:349-354).
      await rejects(() => saveSetting("contextQualityRetentionDays", 45), "an unsupported retention window");
      assert.equal((await readSettings()).contextQualityRetentionDays, 90);
      // Only that one key takes a number; every other key must arrive as a string or boolean
      // (desktop/mapper.rs:14-28).
      await rejects(
        () => saveSetting("automaticContextCompactionEnabled", 1),
        "a numeric value for a boolean setting",
      );
      await rejects(() => saveSetting("notASetting", "value"), "an unknown setting key");

      // Custom instructions are capped at 3000 characters
      // (desktop/domain/settings.rs:4, :426-428). Each accepted value is read back before it is
      // reverted -- reverting without reading is what let a write-only setting look healthy -- and
      // reverted in the same breath so a later spec's generation never inherits it.
      const sentinel = "Desktop sweep personalization sentinel.";
      await saveSetting("customInstructionsAboutUser", sentinel);
      assert.equal(
        (await readSettings()).customInstructionsAboutUser,
        sentinel,
        "a saved custom instruction did not survive a read",
      );
      await saveSetting("customInstructionsAboutUser", "");
      assert.equal((await readSettings()).customInstructionsAboutUser, "");

      const longRule = "x".repeat(3_000);
      await saveSetting("customInstructionsStyleRules", longRule);
      assert.equal(
        (await readSettings()).customInstructionsStyleRules,
        longRule,
        "a style rule at the 3000-character limit did not survive a read",
      );
      await saveSetting("customInstructionsStyleRules", "");
      await rejects(
        () => saveSetting("customInstructionsAboutUser", "x".repeat(3_001)),
        "an over-limit custom instruction",
      );

      // Same flip-and-read-back-and-revert for the three booleans, whose defaults are all `true`
      // (desktop/domain/settings.rs:467-472).
      for (const key of ["customInstructionsEnabled", "memoryEnabled", "memoryToolAssistedChatsEnabled"]) {
        await saveSetting(key, false);
        assert.equal((await readSettings())[key], false, `${key} did not survive a read after being turned off`);
        await saveSetting(key, true);
        assert.equal((await readSettings())[key], true, `${key} did not survive a read after being turned back on`);
      }

      const memories = await listMemories();
      assert.ok(Array.isArray(memories), "the shared agent memory pool is not readable");
    } finally {
      // Restored from the values read at the top, not from a remembered default. Retried because a
      // concurrent write can hold the settings row and a lost restore would hand every later spec
      // the wrong personalization.
      await globalThis.browser.waitUntil(async () => {
        try {
          await saveSetting(
            "automaticContextCompactionEnabled",
            before.automaticContextCompactionEnabled,
          );
          await saveSetting("contextQualityRetentionDays", before.contextQualityRetentionDays);
          // The five write-only keys cannot be read, so they go back to their documented defaults
          // (desktop/domain/settings.rs:467-472) rather than to a value observed at the start.
          // Restoring them here as well as inline covers a failure between a flip and its revert.
          await saveSetting("customInstructionsAboutUser", "");
          await saveSetting("customInstructionsStyleRules", "");
          for (const key of ["customInstructionsEnabled", "memoryEnabled", "memoryToolAssistedChatsEnabled"]) {
            await saveSetting(key, true);
          }
          return true;
        } catch {
          return false;
        }
      }, { timeout: 30_000, interval: 1_000, timeoutMsg: "The sweep's personalization settings were not restored." });
      const restored = await readSettings();
      assert.equal(restored.automaticContextCompactionEnabled, before.automaticContextCompactionEnabled);
      assert.equal(restored.contextQualityRetentionDays, before.contextQualityRetentionDays);
    }
  });

  globalThis.it("reads and deletes an entry in the shared agent memory pool", async function deleteOneMemory() {
    const dataDir = process.env.VANEHUB_APP_DATA_DIR;
    if (!dataDir) {
      blocked.push("agent memory delete: VANEHUB_APP_DATA_DIR is unset, so the pool cannot be seeded");
      this.skip();
    }

    // Seeded as a file rather than produced by a generation. Nothing writes a memory except
    // extraction, and extraction only runs inside compaction, so producing one for real would mean
    // pushing a conversation past the compaction threshold and then depending on the model
    // choosing to emit a well-formed create action -- which makes the provider's behaviour the
    // precondition of the test. The pool is a directory of markdown files whose id is the relative
    // path (infrastructure/memory_directory.rs:53-54, :412-418), so a file is a first-class way in,
    // the same way domain-skills seeds a Skill package. Under test here is the read and delete
    // surface over real on-disk state, not extraction.
    const name = `desktop-sweep-memory-${RUN}`;
    const memoryRoot = join(dataDir, "memory");
    await mkdir(memoryRoot, { recursive: true });
    // Frontmatter shape taken from `compose_memory_document`
    // (domain/memory_document.rs:131-153), which is what writes these files in production.
    await writeFile(
      join(memoryRoot, `${name}.md`),
      [
        "---",
        `name: ${name}`,
        "description: Seeded by the desktop prompt-hook sweep.",
        "type: project",
        "source: automatic",
        "---",
        "",
        "The desktop sweep seeded this memory to exercise the pool's read and delete surface.",
        "",
      ].join("\n"),
      "utf8",
    );

    const memories = await listMemories();
    const target = memories.find((memory) => memory.name === name);
    assert.ok(
      target,
      `the seeded memory was not listed; the pool held ${JSON.stringify(memories.map((entry) => entry.name))}`,
    );
    assert.equal(target.description, "Seeded by the desktop prompt-hook sweep.");
    assert.match(target.content, /read and delete surface/);

    await deleteMemory(target.id);
    const remaining = await listMemories();
    assert.equal(
      remaining.find((memory) => memory.id === target.id),
      undefined,
      "the deleted memory is still in the pool",
    );
    assert.equal(
      remaining.length,
      memories.length - 1,
      "deleting one memory changed the pool by more than one entry",
    );
  });

  globalThis.after(async () => {
    // Belt and braces: a test that failed before its own cleanup must not leave a user hook that
    // an enabled binding would inject into the next spec's prompts.
    await removeHookQuietly(LIFECYCLE_HOOK);
    await removeHookQuietly(RENDER_HOOK);
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // No `exit_application` here, and no navigation to restore: this spec never leaves the route
    // the app booted on. Exiting from an after hook races WDIO's `deleteSession` and discards
    // every per-test result for the file.
  });
});
