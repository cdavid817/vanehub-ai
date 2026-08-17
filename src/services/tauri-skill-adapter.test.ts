import { beforeEach, describe, expect, it, vi } from "vitest";
import type { Skill, SkillLoadOutcome, SkillPreview, SkillResourceReadOutcome } from "../types/skill";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";

const effectiveSkill: Skill = {
  id: "code-review",
  scope: "global",
  workspacePath: null,
  source: "user",
  enabled: true,
  skillDir: "D:/home/.vanehub/skills/code-review",
  skillMdPath: "D:/home/.vanehub/skills/code-review/SKILL.md",
  contentHash: "user-revision",
  metadata: {
    id: "code-review",
    name: "Code Review",
    description: "Review changes.",
    category: "review",
    version: "2.0.0",
    triggers: ["review"],
    aliases: ["review"],
    type: "role",
    delivery: "on-demand",
    compatibilityDefaults: { skillType: false, delivery: false },
  },
  boundAgentIds: ["codex-cli"],
  bindings: [],
  createdAt: "2026-08-01T00:00:00Z",
  updatedAt: "2026-08-02T00:00:00Z",
  layer: "user",
  origin: "migrated",
  trust: "trusted",
  availability: "available",
  delegationCapability: { supported: false, reason: "not-utility" },
  immutable: false,
  shadowedDefinitions: [{
    layer: "system",
    origin: "shipped",
    version: "1.0.0",
    availability: "available",
  }],
  usage: {
    viewCount: 4,
    useCount: 2,
    lastViewedAt: "2026-08-03T00:00:00Z",
    lastUsedAt: "2026-08-02T00:00:00Z",
    revisionWitness: "usage-3",
  },
};

describe("Tauri effective Skill adapter", () => {
  beforeEach(() => invokeMock.mockReset());

  it("keeps effective inventory and preview metadata intact", async () => {
    invokeMock
      .mockResolvedValueOnce({ skills: [effectiveSkill], stats: { total: 1, enabled: 1, mounted: 1 } })
      .mockResolvedValueOnce({
        id: effectiveSkill.id,
        scope: effectiveSkill.scope,
        workspacePath: null,
        content: "# Code Review",
        path: "skill://code-review/",
        layer: "system",
        origin: "shipped",
        availability: "available",
        immutable: true,
        shadowedDefinitions: [],
      } satisfies SkillPreview);

    await expect(tauriAgentClient.listSkills({ scope: "global" })).resolves.toMatchObject({
      skills: [{ layer: "user", delegationCapability: { supported: false, reason: "not-utility" }, usage: { viewCount: 4 }, shadowedDefinitions: [{ layer: "system" }] }],
    });
    await expect(tauriAgentClient.previewSkill("code-review", { scope: "global" })).resolves.toMatchObject({
      path: "skill://code-review/",
      immutable: true,
    });
  });

  it("maps state operations and bounded resource operations only through invoke", async () => {
    const loaded: SkillLoadOutcome = {
      status: "loaded",
      result: {
        id: "code-review",
        name: "Code Review",
        content: "instructions",
        truncated: true,
        revision: "system-v1",
        baseUri: "skill://code-review/",
        resources: {
          scripts: [],
          references: [{
            uri: "skill://code-review/references/checklist.md",
            relativePath: "references/checklist.md",
            sizeBytes: 42,
          }],
          templates: [],
          assets: [],
          truncated: false,
        },
      },
    };
    const resource: SkillResourceReadOutcome = {
      status: "read",
      result: {
        id: "code-review",
        uri: "skill://code-review/references/checklist.md",
        revision: "system-v1",
        content: "checklist",
        sizeBytes: 42,
      },
    };
    invokeMock
      .mockResolvedValueOnce(effectiveSkill)
      .mockResolvedValueOnce(effectiveSkill)
      .mockResolvedValueOnce(effectiveSkill)
      .mockResolvedValueOnce(undefined)
      .mockResolvedValueOnce(effectiveSkill)
      .mockResolvedValueOnce(loaded)
      .mockResolvedValueOnce(resource);
    const scope = { scope: "global" as const };
    const readInput = {
      uri: "skill://code-review/references/checklist.md",
      revision: "system-v1",
      workspacePath: null,
    };

    await tauriAgentClient.setSkillEnabled("code-review", scope, false);
    await tauriAgentClient.bindSkillToCliAgent("code-review", scope, "codex-cli");
    await tauriAgentClient.setSkillAgentBindings("code-review", scope, ["codex-cli"]);
    await tauriAgentClient.bindSkillToApiAgent("code-review", scope, "onepiece");
    await tauriAgentClient.restoreBuiltinSkill("code-review");
    await expect(tauriAgentClient.loadSkill({ idOrAlias: "review", workspacePath: null }))
      .resolves.toEqual(loaded);
    await expect(tauriAgentClient.readSkillResource(readInput)).resolves.toEqual(resource);

    expect(invokeMock.mock.calls).toEqual([
      ["set_skill_enabled", { skillId: "code-review", input: scope, enabled: false }],
      ["bind_skill_to_cli_agent", { skillId: "code-review", input: scope, agentId: "codex-cli" }],
      ["set_skill_agent_bindings", { skillId: "code-review", input: scope, agentIds: ["codex-cli"] }],
      ["bind_skill_to_api_agent", { skillId: "code-review", input: scope, agentId: "onepiece" }],
      ["restore_builtin_skill", { skillId: "code-review" }],
      ["load_skill", { input: { idOrAlias: "review", workspacePath: null } }],
      ["read_skill_resource", { input: readInput }],
    ]);
  });

  it("routes every Skill tool governance operation through the native boundary", async () => {
    invokeMock.mockResolvedValue({});
    const owner = { skillId: "code-review", scope: "global" as const };
    const revision = { revision: "a".repeat(64) };
    await tauriAgentClient.listSkillTools(owner);
    await tauriAgentClient.validateSkillToolRevision(revision);
    await tauriAgentClient.setSkillToolTrust({ ...revision, trusted: true, actor: "operator" });
    await tauriAgentClient.setSkillToolEnabled({ ...revision, enabled: true });
    await tauriAgentClient.quarantineSkillTool({ ...revision, reason: "operator" });
    await tauriAgentClient.recoverSkillTool(revision);
    await tauriAgentClient.getSkillToolDiagnostics(revision);

    expect(invokeMock.mock.calls).toEqual([
      ["list_skill_tools", { input: owner }],
      ["validate_skill_tool_revision", { input: revision }],
      ["set_skill_tool_trust", { input: { ...revision, trusted: true, actor: "operator" } }],
      ["set_skill_tool_enabled", { input: { ...revision, enabled: true } }],
      ["quarantine_skill_tool", { input: { ...revision, reason: "operator" } }],
      ["recover_skill_tool", { input: revision }],
      ["get_skill_tool_diagnostics", { input: revision }],
    ]);
  });
});
