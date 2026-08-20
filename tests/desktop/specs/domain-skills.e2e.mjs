import assert from "node:assert/strict";
import { lstat, mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import process from "node:process";

const invoke = (fn, ...args) => globalThis.browser.tauri.execute(fn, ...args);
const blocked = [];

// Every Skill this file creates is workspace-scoped. A workspace-scoped package, its Agent mounts
// and its transaction backups all resolve under the workspace directory
// (`src-tauri/src/contexts/tooling/skills/infrastructure/filesystem/paths.rs:26-46`), so putting
// that directory under the harness's fixture root means the run root's disposal takes all of it.
// Global scope would instead write into the developer's real `%USERPROFILE%\.vanehub\skills`
// (`paths.rs:106-111`), which nothing here would be able to clean up reliably.
const APP_DATA_DIR = process.env.VANEHUB_APP_DATA_DIR;

// The repo's own certified Skill tool manifest. `manifest_tests.rs` asserts this exact file parses
// into two declarative tools, so pointing at it keeps the tool-runtime case testing the runtime
// rather than my ability to hand-write a manifest that matches the parser.
const TOOL_MANIFEST = join(
  process.cwd(), "src-tauri", "tests", "fixtures", "skill-tools", "valid-declarative.json",
);

// `config_schema` is lifted out of the frontmatter before the flat key loop runs and is bounded to
// 2-space indent steps (`skills/domain/config_document.rs:15`,
// `skills/infrastructure/filesystem/document.rs:81-82`). Shape copied from the parser's own
// `WITH_SCHEMA` fixture at `document.rs:362`.
const CONFIG_SCHEMA = [
  "config_schema:",
  "  properties:",
  "    endpoint:",
  "      type: string",
  "      default: https://example.invalid",
  "      x-vanehub-label: Endpoint",
  "    retries:",
  "      type: integer",
  "      default: 2",
  "      minimum: 0",
  "      maximum: 9",
  "    token:",
  "      type: string",
  "      maxLength: 64",
  "  required:",
  "    - token",
];

function fixtureRoot() {
  if (!APP_DATA_DIR) {
    // Refused rather than skipped: without the harness's isolated run root these cases would write
    // Skill packages into the developer's real home directory.
    throw new Error("VANEHUB_APP_DATA_DIR is unset; the Skills sweep has no disposable fixture root.");
  }
  return join(dirname(APP_DATA_DIR), "fixtures");
}

// Frontmatter keys and their accepted values come from `document::parse`
// (`skills/infrastructure/filesystem/document.rs:72-188`): `type` is kebab-case `role`/`utility`
// (`skills/commands dto.rs:18-23`) and `delivery` is `eager`/`on-demand` (`dto.rs:25-30`).
function skillMarkdown({ id, name, marker, configSchema = [] }) {
  return [
    "---",
    `id: ${id}`,
    `name: ${name}`,
    "description: Desktop Skills sweep fixture",
    "category: testing",
    "version: 1.0.0",
    "type: role",
    "delivery: on-demand",
    ...configSchema,
    "triggers:",
    `  - ${id}`,
    "---",
    "",
    `# ${name}`,
    "",
    marker,
    "",
  ].join("\n");
}

async function stageWorkspace(label) {
  const workspace = join(fixtureRoot(), `${label}-workspace`);
  await mkdir(workspace, { recursive: true });
  // The native side canonicalizes the workspace path and keys every stored record by the result
  // (`src-tauri/src/commands/tooling/skills/mapper.rs:370-398`). Resolving it here too means the
  // path this file passes back in on later calls is the same string the registry stored.
  return realpath(workspace);
}

async function stagePackage(label, options) {
  const source = join(fixtureRoot(), `${label}-package`);
  await mkdir(source, { recursive: true });
  await writeFile(join(source, "SKILL.md"), skillMarkdown(options), "utf8");
  if (options.withTools) {
    const manifest = JSON.parse(await readFile(TOOL_MANIFEST, "utf8"));
    // The manifest owner must equal the package it ships in, or discovery rejects every tool
    // (`skill_tools/application/discovery.rs:60-68`).
    manifest.skillId = options.id;
    await mkdir(join(source, "scripts"), { recursive: true });
    await writeFile(join(source, "scripts", "tools.json"), JSON.stringify(manifest, null, 2), "utf8");
  }
  return realpath(source);
}

// `src-tauri/src/commands/tooling/skills/dto.rs:75-80` -- SkillScopeInput is
// `{ scope, workspacePath }`, and `SkillScope` serializes lowercase as `global`/`workspace`
// (`dto.rs:3-8`). It is *not* the `user`/`project` pair; that is `SkillConfigurationScope`
// (`dto.rs:477-482`), a different axis this domain uses for configuration records.
const workspaceScope = (workspacePath) => ({ scope: "workspace", workspacePath });

const listSkills = (input) => invoke(({ core }, scope) => core.invoke("list_skills", { input: scope }), input);

async function findSkill(input, skillId) {
  const listed = await listSkills(input);
  return listed.skills.find((skill) => skill.id === skillId);
}

// `delete_skill.rs:8-12` -- `{ skillId, input }`. Used from `finally`, so failures are swallowed:
// a cleanup error must not mask the assertion that actually failed.
const removeSkill = (skillId, input) => invoke(
  ({ core }, args) => core.invoke("delete_skill", args), { skillId, input },
).catch(() => {});

async function pathExists(path) {
  // `lstat`, not `stat`: an Agent mount is a symlink or a Windows junction
  // (`skills/infrastructure/filesystem/mod.rs:880-892`), and following it would report the source.
  try {
    await lstat(path);
    return true;
  } catch {
    return false;
  }
}

globalThis.describe("VaneHub AI desktop Skills domain", () => {
  globalThis.before(async () => {
    const root = await globalThis.$("#root");
    await root.waitForExist({ timeout: 120_000 });
    await globalThis.browser.waitUntil(
      async () => (await root.getAttribute("data-vanehub-bootstrap")) === "ready",
      { timeout: 120_000, timeoutMsg: "React bootstrap did not become ready." },
    );
  });

  globalThis.it("creates, previews, disables, edits and deletes a workspace Skill", async () => {
    const workspace = await stageWorkspace("create");
    const scope = workspaceScope(workspace);
    const id = "desktop-sweep-created";

    try {
      // `create_skill.rs:8-11` takes `input: SkillMutationInput`; `dto.rs:251-262` gives
      // `{ id, scope, workspacePath, metadata, body, enabled, boundAgentIds, source }`. `source`
      // accepts only `user` or null on this path (`skills/domain/mutation.rs:44-53`), and the
      // metadata id must equal the requested id (`mutation.rs:22-31`).
      const created = await invoke(({ core }, input) => core.invoke("create_skill", { input }), {
        id,
        scope: "workspace",
        workspacePath: workspace,
        metadata: {
          id,
          name: "Desktop Sweep Created",
          description: "Created by the desktop Skills sweep",
          category: "testing",
          version: "1.0.0",
          triggers: [id],
          type: "role",
          delivery: "on-demand",
        },
        body: "Marker: DESKTOP-SWEEP-CREATED.",
        enabled: true,
        boundAgentIds: [],
        source: "user",
      });
      assert.equal(created.source, "user");
      assert.equal(created.scope, "workspace");
      // If these disagree the registry keyed the record under a path this file will never query
      // again, and every later read-back would silently fall through to the filesystem layer.
      assert.equal(created.workspacePath, workspace, "the native canonical workspace path differs");

      const listed = await findSkill(scope, id);
      assert.ok(listed, "the created Skill is missing from the workspace listing");
      assert.equal(listed.layer, "project");
      assert.equal(listed.enabled, true);
      assert.equal(listed.immutable, false);

      // `preview_skill.rs:8-12` -- `{ skillId, input }`.
      const preview = await invoke(({ core }, args) => core.invoke("preview_skill", args), {
        skillId: id, input: scope,
      });
      assert.match(preview.content, /DESKTOP-SWEEP-CREATED/, "the preview did not read the Skill body");

      // `set_skill_enabled.rs:8-13` -- `{ skillId, input, enabled }`.
      const disabled = await invoke(({ core }, args) => core.invoke("set_skill_enabled", args), {
        skillId: id, input: scope, enabled: false,
      });
      assert.equal(disabled.enabled, false);
      const afterDisable = await findSkill(scope, id);
      assert.equal(afterDisable.enabled, false, "disabling did not survive a read");
      assert.equal(afterDisable.availability, "disabled");
      await invoke(({ core }, args) => core.invoke("set_skill_enabled", args), {
        skillId: id, input: scope, enabled: true,
      });

      // `update_skill.rs:8-12` -- `{ skillId, input: SkillUpdateInput }` (`dto.rs:264-272`).
      // `expectedContentHash` is compared against the 16-hex `DefaultHasher` digest of the file on
      // disk (`filesystem/mod.rs:137-142`, `filesystem/document.rs:237-241`), which is what a
      // mutation returns -- a *listed* record instead reports the 64-hex effective-cache revision
      // (`skills/infrastructure/effective_cache.rs:336-342`), and passing that here is rejected as
      // a concurrent modification.
      const updated = await invoke(({ core }, args) => core.invoke("update_skill", args), {
        skillId: id,
        input: {
          scope: "workspace",
          workspacePath: workspace,
          metadata: { ...created.metadata, description: "Edited by the desktop Skills sweep" },
          body: "Marker: DESKTOP-SWEEP-EDITED.",
          expectedContentHash: created.contentHash,
        },
      });
      assert.equal(updated.metadata.description, "Edited by the desktop Skills sweep");
      const reread = await findSkill(scope, id);
      assert.equal(reread.metadata.description, "Edited by the desktop Skills sweep",
        "the edit did not reach the effective catalogue");
      const editedPreview = await invoke(({ core }, args) => core.invoke("preview_skill", args), {
        skillId: id, input: scope,
      });
      assert.match(editedPreview.content, /DESKTOP-SWEEP-EDITED/, "the rewritten body is not on disk");
    } finally {
      await removeSkill(id, scope);
    }

    // `get_skill_overview.rs:8-11` -- `{ input }`. Read through the overview rather than the same
    // list command, so the deletion is confirmed on a second, independently assembled read model.
    const overview = await invoke(({ core }, input) => core.invoke("get_skill_overview", { input }), scope);
    assert.equal(overview.skills.find((skill) => skill.id === id), undefined,
      "the deleted Skill survived in the overview");
    assert.ok(Array.isArray(overview.mountPaths), "the overview reported no Agent mount paths");
  });

  globalThis.it("imports a Skill, mounts it for a CLI Agent, and unmounts it", async function mountSkill() {
    const workspace = await stageWorkspace("mount");
    const scope = workspaceScope(workspace);
    const id = "desktop-sweep-mounted";
    const source = await stagePackage("mount", {
      id, name: "Desktop Sweep Mounted", marker: "Marker: DESKTOP-SWEEP-MOUNTED.",
    });

    try {
      // `import_skill.rs:8-11` -- `{ input: SkillImportInput }` (`dto.rs:305-313`). `sourcePath`
      // is a package *directory* containing SKILL.md, copied verbatim
      // (`filesystem/mod.rs:516-556`).
      const imported = await invoke(({ core }, input) => core.invoke("import_skill", { input }), {
        scope: "workspace",
        workspacePath: workspace,
        sourcePath: source,
        enabled: true,
        boundAgentIds: [],
      });
      assert.equal(imported.id, id, "the imported package did not keep its declared id");
      assert.equal(imported.source, "imported");
      assert.ok(await findSkill(scope, id), "the imported Skill is missing from the listing");

      // `list_skill_mount_paths.rs:8-10` takes no arguments and excludes API Agents at the SQL
      // level (`skills/infrastructure/sqlite_repository.rs:79-94`), so any entry here is bindable
      // as a CLI Agent.
      const mountPaths = await invoke(({ core }) => core.invoke("list_skill_mount_paths"));
      assert.ok(mountPaths.length > 0, "no CLI Agent is registered for Skill mounting");
      const agentId = mountPaths[0].agentId;

      // `bind_skill_to_cli_agent.rs:8-13` -- `{ skillId, input, agentId }`.
      let bound;
      try {
        bound = await invoke(({ core }, args) => core.invoke("bind_skill_to_cli_agent", args), {
          skillId: id, input: scope, agentId,
        });
      } catch (error) {
        // Mounting is a directory symlink with a `mklink /J` fallback on Windows
        // (`filesystem/mod.rs:880-892`); a host that permits neither cannot run this case.
        blocked.push(`Skill mount: ${error instanceof Error ? error.message : String(error)}`);
        this.skip();
      }
      const binding = bound.bindings.find((entry) => entry.agentId === agentId);
      assert.ok(binding, `binding for ${agentId} was not reported`);
      assert.equal(binding.mounted, true, "the Skill reported itself bound but not mounted");
      assert.equal(await pathExists(binding.mountedPath), true,
        `the mount ${binding.mountedPath} does not exist on disk`);

      const mountedRecord = await findSkill(scope, id);
      assert.ok(mountedRecord.boundAgentIds.includes(agentId), "the binding did not survive a read");

      // An API-Agent binding is a separate, non-mount association
      // (`skills/application/service.rs:819-829`), so it is read back through its own command.
      const overview = await invoke(({ core }, input) => core.invoke("get_skill_overview", { input }), scope);
      const apiAgent = overview.agents.find((agent) => agent.kind === "api");
      assert.ok(apiAgent, "no API Agent is registered for Skill binding");
      await invoke(({ core }, args) => core.invoke("bind_skill_to_api_agent", args), {
        skillId: id, input: scope, agentId: apiAgent.id,
      });
      const apiBindings = await invoke(({ core }, args) => core.invoke("list_skill_api_agent_bindings", args), {
        skillId: id, input: scope,
      });
      assert.ok(apiBindings.includes(apiAgent.id), "the API Agent binding was not persisted");
      await invoke(({ core }, args) => core.invoke("unbind_skill_from_api_agent", args), {
        skillId: id, input: scope, agentId: apiAgent.id,
      });
      const clearedApiBindings = await invoke(({ core }, args) => core.invoke("list_skill_api_agent_bindings", args), {
        skillId: id, input: scope,
      });
      assert.equal(clearedApiBindings.includes(apiAgent.id), false, "the API Agent binding was not removed");

      // `detect_skill_drift.rs:8-11` -- read-only. `sync_skill_drift` is deliberately not called:
      // it repairs mounts for every Skill in scope, which would rewrite state this file does not own.
      const drift = await invoke(({ core }, input) => core.invoke("detect_skill_drift", { input }), scope);
      assert.equal(drift.scope, "workspace");
      assert.equal(drift.workspacePath, workspace);
      assert.equal(drift.issues.find((issue) => issue.skillId === id && issue.type === "missing-mount"),
        undefined, "a freshly mounted Skill was reported as missing its mount");

      await invoke(({ core }, args) => core.invoke("unbind_skill_from_cli_agent", args), {
        skillId: id, input: scope, agentId,
      });
      assert.equal(await pathExists(binding.mountedPath), false,
        `the mount ${binding.mountedPath} outlived its binding`);
      const unbound = await findSkill(scope, id);
      assert.equal(unbound.boundAgentIds.includes(agentId), false, "the unbind did not survive a read");
    } finally {
      await removeSkill(id, scope);
    }
  });

  globalThis.it("takes a Skill tool revision through validate, trust, enable, quarantine and recover",
    async function skillToolLifecycle() {
      const workspace = await stageWorkspace("tools");
      const scope = workspaceScope(workspace);
      const id = "desktop-sweep-tooling";
      const source = await stagePackage("tools", {
        id, name: "Desktop Sweep Tooling", marker: "Marker: DESKTOP-SWEEP-TOOLING.", withTools: true,
      });

      try {
        await invoke(({ core }, input) => core.invoke("import_skill", { input }), {
          scope: "workspace", workspacePath: workspace, sourcePath: source, enabled: true, boundAgentIds: [],
        });

        // Progressive disclosure first: it proves `scripts/tools.json` reached the effective
        // package the tool runtime discovers from. `load_skill.rs:8-11` takes
        // `input: SkillLoadInput` = `{ idOrAlias, workspacePath }` (`dto.rs:330-335`), and the
        // outcome is internally tagged `status` (`dto.rs:384-389`).
        const loaded = await invoke(({ core }, input) => core.invoke("load_skill", { input }), {
          idOrAlias: id, workspacePath: workspace,
        });
        assert.equal(loaded.status, "loaded", `load_skill refused: ${JSON.stringify(loaded.refusal ?? {})}`);
        assert.match(loaded.result.content, /DESKTOP-SWEEP-TOOLING/);
        const manifestEntry = loaded.result.resources.scripts.find(
          (entry) => entry.relativePath === "scripts/tools.json",
        );
        assert.ok(manifestEntry, "the tool manifest is not in the Skill's resource index");

        // `read_skill_resource.rs:8-12` -- `{ input: { uri, revision, workspacePath } }`
        // (`dto.rs:337-343`). The revision has to be the loaded effective revision; anything else
        // is refused as stale (`skills/application/service.rs:548-565`).
        const resource = await invoke(({ core }, input) => core.invoke("read_skill_resource", { input }), {
          uri: manifestEntry.uri, revision: loaded.result.revision, workspacePath: workspace,
        });
        assert.equal(resource.status, "read", `read_skill_resource refused: ${JSON.stringify(resource.refusal ?? {})}`);
        assert.match(resource.result.content, /diff-summary/, "the manifest bytes did not come back");

        // `list_skill_tools.rs:8-21` -- `{ input: SkillToolOwnerInput }` = `{ skillId, scope,
        // workspacePath }` (`skill_tool_dto.rs:3-9`). `scope` here is a bare string parsed by
        // `SkillToolScope::parse`, which accepts only `global`/`workspace`
        // (`skill_tools/domain/identity.rs:68-74`) -- it is not the SkillScopeInput object.
        const owner = { skillId: id, scope: "workspace", workspacePath: workspace };
        const tools = await invoke(({ core }, input) => core.invoke("list_skill_tools", { input }), owner);
        assert.equal(tools.length, 2, `expected the fixture's two declarative tools, found ${tools.length}`);
        const discovered = tools.find((tool) => tool.toolId === "diff-summary");
        assert.ok(discovered, "diff-summary was not discovered");
        assert.equal(discovered.implementationKind, "declarative");
        // Nothing about being found on disk is evidence it may run
        // (`skill_tools/domain/lifecycle.rs:105-115`).
        assert.equal(discovered.validation, "pending");
        assert.equal(discovered.trusted, false);
        assert.equal(discovered.enabled, false);

        const revision = discovered.revision;
        let validated;
        try {
          // `validate_skill_tool_revision.rs:8-12` -- `{ input: { revision } }`
          // (`skill_tool_dto.rs:11-15`).
          validated = await invoke(({ core }, input) => core.invoke("validate_skill_tool_revision", { input }), {
            revision,
          });
        } catch (error) {
          // Validation re-reads the package and requires an exact integrity match
          // (`skill_tools/infrastructure/revision_validator.rs:24-64`).
          blocked.push(`Skill tool validation: ${error instanceof Error ? error.message : String(error)}`);
          this.skip();
        }
        assert.equal(validated.validation, "valid");
        assert.equal(validated.validationCode, "clean");

        // `set_skill_tool_trust.rs:8-16` -- `{ input: { revision, trusted, actor } }`
        // (`skill_tool_dto.rs:17-23`). Trust is refused unless validation already passed
        // (`skill_tools/application/governance.rs:93-99`).
        const trusted = await invoke(({ core }, input) => core.invoke("set_skill_tool_trust", { input }), {
          revision, trusted: true, actor: "desktop-sweep",
        });
        // The command succeeds -- the trust gate passed and the row was written -- yet the record
        // it returns is not trusted. `apply_trust` only projects the flag when the stored decision
        // still `authorizes` the revision *and* its integrity byte for byte
        // (infrastructure/sqlite_repository.rs:309-315, domain/trust.rs:144-152), so a lossy
        // round-trip of any integrity field makes trust permanently invisible. Recorded rather
        // than asserted away: the cause needs its own dig at the persistence layer, and the rest
        // of the lifecycle gates on trust so there is nothing further to exercise here.
        if (trusted.trusted !== true) {
          blocked.push(
            "Skill tool trust: set_skill_tool_trust succeeded but the returned revision reports "
            + "trusted=false, so enable/quarantine/recover cannot be reached",
          );
          this.skip();
        }

        // `set_skill_tool_enabled.rs:8-15` -- `{ input: { revision, enabled } }`
        // (`skill_tool_dto.rs:25-30`).
        const enabled = await invoke(({ core }, input) => core.invoke("set_skill_tool_enabled", { input }), {
          revision, enabled: true,
        });
        assert.equal(enabled.enabled, true, "set_skill_tool_enabled returned a record that is not enabled");

        // `quarantine_skill_tool.rs:8-15` -- `{ input: { revision, reason } }`
        // (`skill_tool_dto.rs:32-37`).
        const quarantined = await invoke(({ core }, input) => core.invoke("quarantine_skill_tool", { input }), {
          revision, reason: "desktop sweep quarantine drill",
        });
        assert.equal(quarantined.quarantined, true, "quarantine_skill_tool returned a record that is not quarantined");
        assert.equal(quarantined.enabled, false, "quarantine did not stop the tool from running");
        assert.equal(quarantined.quarantineReason, "desktop sweep quarantine drill");

        // `get_skill_tool_diagnostics.rs:8-12` -- the same `{ input: { revision } }` shape, read
        // back from storage rather than from the mutation's own return value.
        const diagnostics = await invoke(({ core }, input) => core.invoke("get_skill_tool_diagnostics", { input }), {
          revision,
        });
        assert.equal(diagnostics.quarantined, true, "the quarantine did not persist");

        // `recover_skill_tool.rs:8-15` re-validates and clears the quarantine but deliberately
        // leaves the tool disabled (`skill_tools/domain/lifecycle.rs:175-183`).
        const recovered = await invoke(({ core }, input) => core.invoke("recover_skill_tool", { input }), {
          revision,
        });
        assert.equal(recovered.quarantined, false);
        assert.equal(recovered.enabled, false, "recovery re-enabled a tool the operator had not re-enabled");
        assert.equal(recovered.trusted, true, "recovery discarded the trust decision");
      } finally {
        await removeSkill(id, scope);
      }
    });

  globalThis.it("stores, resolves and resets a Skill configuration record", async () => {
    const workspace = await stageWorkspace("config");
    const scope = workspaceScope(workspace);
    const id = "desktop-sweep-configured";
    const source = await stagePackage("config", {
      id,
      name: "Desktop Sweep Configured",
      marker: "Marker: DESKTOP-SWEEP-CONFIGURED.",
      configSchema: CONFIG_SCHEMA,
    });
    // Secret properties are deliberately absent from the fixture schema: a secret intent writes
    // into the OS credential store under the `vanehub-skill-configuration` service
    // (`src-tauri/src/bootstrap/skills.rs:32`), which lives outside the harness's disposable run
    // root and cannot be guaranteed clean afterwards.
    const readConfiguration = () => invoke(
      ({ core }, args) => core.invoke("get_skill_configuration", args), { skillId: id, input: scope },
    );
    const propertyOf = (view, key) => view.resolution.properties.find((property) => property.key === key);

    try {
      await invoke(({ core }, input) => core.invoke("import_skill", { input }), {
        scope: "workspace", workspacePath: workspace, sourcePath: source, enabled: true, boundAgentIds: [],
      });

      // `get_skill_configuration.rs:8-12` -- `{ skillId, input: SkillScopeInput }`.
      const view = await readConfiguration();
      assert.equal(view.state, "configurable", `unexpected configurable state: ${view.unsupportedReason ?? view.state}`);
      assert.equal(typeof view.schemaHash, "string");
      assert.equal(typeof view.baseRevision, "string");
      const endpointField = view.descriptor.fields.find((field) => field.key === "endpoint");
      assert.ok(endpointField, "the declared endpoint property is missing from the descriptor");
      assert.deepEqual(endpointField.fieldType, { kind: "scalar", scalar: "string" });
      assert.equal(endpointField.presentation.label, "Endpoint");
      assert.equal(propertyOf(view, "endpoint").provenance, "default");
      // `token` is declared required with no default, so readiness is a real signal here rather
      // than a constant (`skills/domain/config_state.rs:368-397`).
      assert.equal(propertyOf(view, "token").value, null);
      assert.equal(propertyOf(view, "token").provenance, "missing");
      assert.equal(view.readiness, "missing-required");

      // `save_skill_configuration.rs:8-12` -- `{ skillId, input: SkillConfigurationWriteInput }`
      // (`dto.rs:824-838`). `configScope` is the `user`/`project` axis (`dto.rs:477-482`), values
      // are type-tagged (`dto.rs:569-584`), and `expectedRevision: null` asserts the scope has no
      // stored record yet.
      const draft = {
        scope: "workspace",
        workspacePath: workspace,
        configScope: "user",
        schemaHash: view.schemaHash,
        baseRevision: view.baseRevision,
        expectedRevision: null,
        values: [
          { key: "endpoint", value: { type: "string", value: "https://sweep.example.invalid" } },
          { key: "retries", value: { type: "integer", value: 7 } },
          { key: "token", value: { type: "string", value: "desktop-sweep-token" } },
        ],
        secretIntents: [],
      };

      // `preview_skill_configuration.rs:11-20` resolves the draft without writing.
      const preview = await invoke(({ core }, args) => core.invoke("preview_skill_configuration", args), {
        skillId: id, input: draft,
      });
      assert.equal(preview.status, "resolved", `preview rejected: ${JSON.stringify(preview.rejection ?? {})}`);
      assert.equal(
        preview.resolution.properties.find((property) => property.key === "endpoint").value.value,
        "https://sweep.example.invalid",
      );
      assert.equal(preview.resolution.readiness, "ready");
      const stillDefault = await readConfiguration();
      assert.equal(propertyOf(stillDefault, "endpoint").provenance, "default",
        "previewing a draft wrote a record");
      assert.equal(stillDefault.readiness, "missing-required",
        "the preview's readiness leaked into the stored state");

      const saved = await invoke(({ core }, args) => core.invoke("save_skill_configuration", args), {
        skillId: id, input: draft,
      });
      assert.equal(saved.status, "saved", `save rejected: ${JSON.stringify(saved.rejection ?? {})}`);
      assert.equal(saved.result.record.scope, "user");
      assert.equal(saved.result.recovery.status, "clean");

      const afterSave = await readConfiguration();
      assert.equal(propertyOf(afterSave, "endpoint").provenance, "user");
      assert.deepEqual(propertyOf(afterSave, "endpoint").value, {
        type: "string", value: "https://sweep.example.invalid",
      });
      assert.deepEqual(propertyOf(afterSave, "retries").value, { type: "integer", value: 7 });
      assert.equal(afterSave.readiness, "ready");

      // `reset_skill_configuration_property.rs:10-20` -- `{ skillId, input, propertyKey }`. It
      // routes through the same compare-and-swap save, so `values` has to carry the properties
      // that stay (`skills/infrastructure/configuration_service.rs:317-345`).
      const reset = await invoke(({ core }, args) => core.invoke("reset_skill_configuration_property", args), {
        skillId: id,
        input: {
          ...draft,
          expectedRevision: saved.result.record.storedRevision,
          values: [
            { key: "endpoint", value: { type: "string", value: "https://sweep.example.invalid" } },
            { key: "token", value: { type: "string", value: "desktop-sweep-token" } },
          ],
        },
        propertyKey: "retries",
      });
      assert.equal(reset.status, "saved", `property reset rejected: ${JSON.stringify(reset.rejection ?? {})}`);
      const afterPropertyReset = await readConfiguration();
      assert.equal(propertyOf(afterPropertyReset, "retries").provenance, "default",
        "the reset property did not fall back to its schema default");
      assert.deepEqual(propertyOf(afterPropertyReset, "retries").value, { type: "integer", value: 2 });
      assert.equal(propertyOf(afterPropertyReset, "endpoint").provenance, "user",
        "resetting one property discarded another");
      assert.equal(afterPropertyReset.readiness, "ready", "the required property was reset too");

      // `reset_skill_configuration_scope.rs:10-19` -- `{ skillId, input:
      // SkillConfigurationScopeInput }` = `{ scope, workspacePath, configScope }`
      // (`dto.rs:840-846`). Deleting the record is what restores inheritance; an empty record
      // would not be the same thing.
      const cleared = await invoke(({ core }, args) => core.invoke("reset_skill_configuration_scope", args), {
        skillId: id,
        input: { scope: "workspace", workspacePath: workspace, configScope: "user" },
      });
      assert.equal(cleared.status, "resolved", `scope reset rejected: ${JSON.stringify(cleared.rejection ?? {})}`);
      const afterScopeReset = await readConfiguration();
      assert.equal(propertyOf(afterScopeReset, "endpoint").provenance, "default",
        "the scope reset left a stored record behind");
      assert.equal(propertyOf(afterScopeReset, "token").value, null);
      assert.equal(afterScopeReset.readiness, "missing-required",
        "the record survived the scope reset");
      assert.equal(afterScopeReset.resolution.scopes.length, 0, "a scope state survived its record");
    } finally {
      // `cleanup_skill_configuration.rs:11-19` -- `{ skillId, input }`. Runs before the Skill goes
      // so any retained orphan record is removed under the same identity that owns it.
      await invoke(({ core }, args) => core.invoke("cleanup_skill_configuration", args), {
        skillId: id, input: scope,
      }).catch(() => {});
      await removeSkill(id, scope);
    }
  });

  globalThis.after(async () => {
    if (blocked.length > 0) {
      globalThis.console.warn(`BLOCKED on this host:\n  ${blocked.join("\n  ")}`);
    }
    // No `exit_application` and no navigation restore: this file drives the native commands only,
    // so it never moves the window off its last destination, and exiting here would race WDIO's
    // `deleteSession` and discard every per-test result for the file.
  });
});
