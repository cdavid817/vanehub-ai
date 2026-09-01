// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { Skill, SkillOverview } from "../../types/skill";

const serviceMocks = vi.hoisted(() => ({
  bindSkillToApiAgent: vi.fn(),
  bindSkillToCliAgent: vi.fn(),
  createSkill: vi.fn(),
  deleteSkill: vi.fn(),
  getSkillOverlayDetail: vi.fn(),
  listSkillTools: vi.fn(),
  quarantineSkillTool: vi.fn(),
  recoverSkillTool: vi.fn(),
  setSkillToolEnabled: vi.fn(),
  setSkillToolTrust: vi.fn(),
  validateSkillToolRevision: vi.fn(),
  getSkillEvolutionSeedLineage: vi.fn(),
  getSkillOverview: vi.fn(),
  purgeSkillEvolutionEvidence: vi.fn(),
  querySkillEvolutionEvidence: vi.fn(),
  importSkill: vi.fn(),
  loadSkill: vi.fn(),
  previewSkill: vi.fn(),
  restoreBuiltinSkill: vi.fn(),
  setSkillEnabled: vi.fn(),
  unbindSkillFromApiAgent: vi.fn(),
  unbindSkillFromCliAgent: vi.fn(),
  updateSkill: vi.fn(),
  updateSkillMountPath: vi.fn(),
  syncSkillDrift: vi.fn(),
}));

vi.mock("../../services/runtime-agent-client", () => ({ agentService: serviceMocks }));

import { SkillsPage } from "./skills-page";

const skill: Skill = {
  id: "reliable-skill",
  scope: "global",
  workspacePath: null,
  source: "user",
  enabled: true,
  skillDir: "~/.vanehub/skills/reliable-skill",
  skillMdPath: "~/.vanehub/skills/reliable-skill/SKILL.md",
  contentHash: "current-hash",
  metadata: {
    id: "reliable-skill",
    name: "Reliable Skill",
    description: "Interaction fixture",
    category: "testing",
    version: "1.0.0",
    triggers: ["reliable"],
  },
  boundAgentIds: [],
  bindings: [],
  createdAt: "now",
  updatedAt: "now",
  layer: "user",
  origin: "created",
  trust: "trusted",
  availability: "available",
  immutable: false,
  shadowedDefinitions: [],
  usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
};

const overview: SkillOverview = {
  skills: [skill],
  stats: { total: 1, enabled: 1, mounted: 0 },
  agents: [
    { id: "codex-cli", displayName: "Codex CLI", kind: "cli" },
    { id: "claude-code", displayName: "Claude Code", kind: "cli" },
    { id: "api-agent", displayName: "API Agent", kind: "api" },
  ],
  mountPaths: [
    { agentId: "codex-cli", mountPath: ".codex/skills", isDefault: true },
    { agentId: "claude-code", mountPath: ".claude/skills", isDefault: true },
  ],
  apiAgentBindings: { "reliable-skill": [] },
  restoreCandidates: ["code-review"],
  drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" },
};

function renderPage() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={queryClient}>
      <SkillsPage searchTerm="" />
    </QueryClientProvider>,
  );
}

function setWideDetailsLayout(matches: boolean) {
  Object.defineProperty(window, "matchMedia", {
    configurable: true,
    writable: true,
    value: vi.fn().mockImplementation(() => ({
      matches,
      media: "(min-width: 1280px)",
      onchange: null,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
      addListener: vi.fn(),
      removeListener: vi.fn(),
      dispatchEvent: vi.fn(),
    })),
  });
}

function modalFor(title: string): HTMLElement {
  const modal = screen.getByRole("heading", { level: 3, name: title }).closest("section");
  if (!modal) throw new Error(`Modal not found: ${title}`);
  return modal;
}

function resolvedSkillMocks() {
  serviceMocks.getSkillOverlayDetail.mockResolvedValue({
    summary: {
      canonicalSkillId: skill.id,
      baseLayer: skill.layer,
      status: "none",
      needsReconcile: false,
      pinned: false,
      baseInstructionHash: "base-instructions",
      basePackageHash: "base-package",
      effectiveHash: "base-instructions",
      lastHealthyScope: null,
      scopes: [],
      scopesTruncated: false,
    },
    baseInstructions: { content: "Current body", totalCharacters: 12, truncated: false },
    effectiveInstructions: { content: "Current body", totalCharacters: 12, truncated: false },
    diff: { baseHash: "base-instructions", effectiveHash: "base-instructions", addedCharacters: 0, removedCharacters: 0, hunks: [], hunksTruncated: false },
    scopeDiffs: [], scopeDiffsTruncated: false,
    mutations: [], mutationsTruncated: false, resources: [], resourcesTruncated: false, conflicts: [], conflictsTruncated: false,
  });
  serviceMocks.listSkillTools.mockResolvedValue([{
    skillId: skill.id, toolId: "inspect-diff", canonicalId: "skill__reliable-skill__inspect-diff__abcdef123456",
    revision: "f".repeat(64), sourceScope: "global", implementationKind: "declarative",
    baseRevision: skill.contentHash, manifestHash: `sha256:${"a".repeat(64)}`, implementationHash: `sha256:${"b".repeat(64)}`,
    capabilityDigest: "read-workspace", capabilityDiff: { currentDigest: "read-workspace", added: ["filesystem.read"], removed: [], changed: true },
    validation: "valid", trusted: false, enabled: false, quarantined: false, consecutiveFailures: 0,
    diagnostics: [{ severity: "info", code: "validated", detail: "manifest accepted" }], runtimeSupport: "supported",
    enforcementStrength: "bounded-native-io", createdAt: "now", updatedAt: "now",
  }]);
  serviceMocks.setSkillToolTrust.mockResolvedValue({ revision: "f".repeat(64) });
  serviceMocks.validateSkillToolRevision.mockResolvedValue({ revision: "f".repeat(64) });
  serviceMocks.setSkillToolEnabled.mockResolvedValue({ revision: "f".repeat(64) });
  serviceMocks.quarantineSkillTool.mockResolvedValue({ revision: "f".repeat(64) });
  serviceMocks.recoverSkillTool.mockResolvedValue({ revision: "f".repeat(64) });
  serviceMocks.setSkillEnabled.mockResolvedValue(skill);
  serviceMocks.bindSkillToCliAgent.mockResolvedValue(skill);
  serviceMocks.bindSkillToApiAgent.mockResolvedValue(undefined);
  serviceMocks.createSkill.mockResolvedValue(skill);
  serviceMocks.importSkill.mockResolvedValue(skill);
  serviceMocks.loadSkill.mockResolvedValue({
    status: "loaded",
    result: {
      id: skill.id,
      name: skill.metadata.name,
      content: "Current body",
      truncated: false,
      revision: skill.contentHash,
      baseUri: `skill://${skill.id}/`,
      resources: {
        scripts: [],
        references: [{ uri: `skill://${skill.id}/references/guide.md`, relativePath: "references/guide.md", sizeBytes: 20 }],
        templates: [],
        assets: [],
        truncated: false,
      },
    },
  });
  serviceMocks.restoreBuiltinSkill.mockResolvedValue(skill);
  serviceMocks.updateSkill.mockResolvedValue(skill);
  serviceMocks.deleteSkill.mockResolvedValue(undefined);
  serviceMocks.updateSkillMountPath.mockResolvedValue({
    agentId: "codex-cli", oldMountPath: ".codex/skills", newMountPath: ".codex/skills-next",
    migrated: [], removed: [], overwritten: [], backedUp: [], failed: [],
  });
}

beforeEach(() => {
  vi.clearAllMocks();
  setWideDetailsLayout(false);
  serviceMocks.getSkillOverview.mockResolvedValue(overview);
  serviceMocks.querySkillEvolutionEvidence.mockResolvedValue({
    signalCount: 0,
    seedCount: 0,
    distributions: {},
    signals: [],
    seeds: [],
    pipeline: { collectionEnabled: true, status: "healthy", queueDepth: 0, failureCount: 0 },
    retentionDays: 90,
    signalQuota: 10_000,
    seedQuota: 2_000,
    byteQuota: 64 * 1024 * 1024,
    droppedCount: 0,
    expiredCount: 0,
  });
  serviceMocks.getSkillEvolutionSeedLineage.mockResolvedValue(null);
  serviceMocks.previewSkill.mockResolvedValue({
    id: skill.id,
    scope: "global",
    workspacePath: null,
    path: skill.skillMdPath,
    content: "---\nid: reliable-skill\n---\n# Reliable Skill\n\nCurrent body",
    layer: "user",
    origin: "created",
    availability: "available",
    immutable: false,
    shadowedDefinitions: [],
  });
  resolvedSkillMocks();
});

describe("SkillsPage interactions", () => {
  it("replaces resolved drift with the refreshed post-synchronization overview", async () => {
    const reportedDrift = {
      scope: "global" as const,
      workspacePath: null,
      issues: [{
        skillId: "code-review",
        type: "metadata-changed" as const,
        agentId: null,
        path: "~/.vanehub/cache/skills/system/code-review/SKILL.md",
        message: "SKILL.md differs from the registry snapshot",
      }],
      driftHash: "legacy-drift",
    };
    serviceMocks.getSkillOverview
      .mockResolvedValueOnce({ ...overview, drift: reportedDrift })
      .mockResolvedValue(overview);
    serviceMocks.syncSkillDrift.mockResolvedValue({
      mounted: [],
      unmounted: [],
      overwritten: [],
      backedUp: [],
      restored: ["code-review"],
      failed: [],
      resolvedFrom: reportedDrift,
    });
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("检测到 1 个 Skill 漂移问题")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "同步" }));

    await waitFor(() => expect(serviceMocks.getSkillOverview).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Skill 同步完成")).toBeTruthy();
    expect(screen.queryByText("检测到 1 个 Skill 漂移问题")).toBeNull();
    expect((screen.getByRole("button", { name: "同步" }) as HTMLButtonElement).disabled).toBe(true);
  });

  it("exposes an accessible Tools tab with governed inventory facts", async () => {
    const user = userEvent.setup();
    renderPage();
    const row = await screen.findByText("Reliable Skill");
    const card = row.closest("article");
    if (!card) throw new Error("Skill card not found");
    await user.click(within(card).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    await user.click(screen.getByRole("tab", { name: "工具" }));

    const panel = screen.getByRole("tabpanel", { name: "工具" });
    expect(await within(panel).findByRole("heading", { name: "inspect-diff" })).toBeTruthy();
    for (const text of ["DECLARATIVE", "精确修订", "实现哈希", "能力", "未信任", "已禁用", "支持原生执行", "INFO · validated"]) {
      expect(within(panel).getAllByText(text, { exact: false }).length).toBeGreaterThan(0);
    }
    expect(serviceMocks.listSkillTools).toHaveBeenCalledWith({ skillId: skill.id, scope: "global", workspacePath: null });
  });

  it("reviews exact hashes and capability changes before trust without enabling", async () => {
    const user = userEvent.setup();
    renderPage();
    const card = (await screen.findByText("Reliable Skill")).closest("article");
    if (!card) throw new Error("Skill card not found");
    await user.click(within(card).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    await user.click(screen.getByRole("tab", { name: "工具" }));
    await user.click(await screen.findByRole("button", { name: "信任此修订" }));

    const dialog = screen.getByRole("dialog", { name: "信任精确工具修订" });
    for (const text of ["基础修订", "清单哈希", "实现哈希", "能力摘要", "filesystem.read"]) expect(within(dialog).getByText(text, { exact: false })).toBeTruthy();
    expect(within(dialog).getByText("信任仅固定到所示精确哈希，且绝不会自动启用执行。")).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "信任此修订" }));
    await waitFor(() => expect(serviceMocks.setSkillToolTrust).toHaveBeenCalledWith({ revision: "f".repeat(64), trusted: true, actor: "settings-user" }));
    expect(serviceMocks.setSkillEnabled).not.toHaveBeenCalled();
  });

  it("validates and confirms quarantine against the displayed exact revision", async () => {
    const user = userEvent.setup();
    renderPage();
    const card = (await screen.findByText("Reliable Skill")).closest("article");
    if (!card) throw new Error("Skill card not found");
    await user.click(within(card).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    await user.click(screen.getByRole("tab", { name: "工具" }));
    await user.click(await screen.findByRole("button", { name: "校验修订" }));
    await waitFor(() => expect(serviceMocks.validateSkillToolRevision).toHaveBeenCalledWith({ revision: "f".repeat(64) }));
    await user.click(screen.getByRole("button", { name: "隔离" }));
    const dialog = screen.getByRole("dialog", { name: "隔离精确修订" });
    expect(within(dialog).getByText("f".repeat(64), { exact: false })).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "隔离修订" }));
    await waitFor(() => expect(serviceMocks.quarantineSkillTool).toHaveBeenCalledWith({ revision: "f".repeat(64), reason: "manual-security-review" }));
  });

  it("keeps focus bounded and returns it when keyboard users cancel trust review", async () => {
    const user = userEvent.setup();
    renderPage();
    const card = (await screen.findByText("Reliable Skill")).closest("article");
    if (!card) throw new Error("Skill card not found");
    await user.click(within(card).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    await user.click(screen.getByRole("tab", { name: "工具" }));
    const trigger = await screen.findByRole("button", { name: "信任此修订" });
    await user.click(trigger);
    const confirm = within(screen.getByRole("dialog", { name: "信任精确工具修订" })).getByRole("button", { name: "信任此修订" });
    await waitFor(() => expect(document.activeElement).toBe(confirm));
    await user.keyboard("{Escape}");
    await waitFor(() => expect(document.activeElement).toBe(trigger));
  });

  it("surfaces stale trust rejection and recovers from a transient validation failure", async () => {
    const user = userEvent.setup();
    serviceMocks.setSkillToolTrust.mockRejectedValueOnce(new Error("stale-revision"));
    serviceMocks.validateSkillToolRevision.mockRejectedValueOnce(new Error("temporary validation failure"));
    renderPage();
    const card = (await screen.findByText("Reliable Skill")).closest("article");
    if (!card) throw new Error("Skill card not found");
    await user.click(within(card).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    await user.click(screen.getByRole("tab", { name: "工具" }));
    await user.click(await screen.findByRole("button", { name: "校验修订" }));
    expect((await screen.findByRole("alert")).textContent).toContain("temporary validation failure");
    await user.click(screen.getByRole("button", { name: "校验修订" }));
    await waitFor(() => expect(serviceMocks.validateSkillToolRevision).toHaveBeenCalledTimes(2));
    await user.click(screen.getByRole("button", { name: "信任此修订" }));
    const dialog = screen.getByRole("dialog", { name: "信任精确工具修订" });
    await user.click(within(dialog).getByRole("button", { name: "信任此修订" }));
    expect((await within(dialog).findByRole("alert")).textContent).toContain("stale-revision");
    expect(screen.getByRole("dialog", { name: "信任精确工具修订" })).toBeTruthy();
  });

  it("renders an overview error without claiming an empty or synchronized result", async () => {
    serviceMocks.getSkillOverview.mockRejectedValueOnce(new Error("overview failed"));
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("overview failed")).toBeTruthy();
    expect(screen.queryByText("没有匹配的 Skill。")).toBeNull();
    expect(screen.queryByText("Skill 配置已同步。")).toBeNull();
    await user.click(screen.getByRole("button", { name: "重试" }));
    expect(await screen.findByText("Reliable Skill")).toBeTruthy();
    expect(serviceMocks.getSkillOverview).toHaveBeenCalledTimes(2);
  });

  it("guards a pending enablement mutation and uses granular CLI/API binding calls", async () => {
    let finishEnablement: ((value: Skill) => void) | undefined;
    serviceMocks.setSkillEnabled.mockReturnValueOnce(
      new Promise<Skill>((resolve) => {
        finishEnablement = resolve;
      }),
    );
    const user = userEvent.setup();
    renderPage();

    const enabled = await screen.findByRole("checkbox", { name: "已启用" });
    await user.click(enabled);
    await waitFor(() => expect((enabled as HTMLInputElement).disabled).toBe(true));
    await user.click(enabled);
    expect(serviceMocks.setSkillEnabled).toHaveBeenCalledTimes(1);
    expect(serviceMocks.setSkillEnabled).toHaveBeenCalledWith(skill.id, { scope: "global", workspacePath: null }, false);

    finishEnablement?.(skill);
    await waitFor(() => expect(
      (screen.getByRole("checkbox", { name: "已启用" }) as HTMLInputElement).disabled,
    ).toBe(false));
    await user.click(screen.getByRole("button", { name: /Codex CLI/ }));
    expect(screen.queryByRole("checkbox", { name: "已启用" })).toBeNull();
    expect(screen.getByText("全局已启用")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "分配给 Codex CLI" }));
    await waitFor(() => expect(serviceMocks.bindSkillToCliAgent).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
      "codex-cli",
    ));
    expect(serviceMocks.bindSkillToCliAgent).toHaveBeenCalledTimes(1);
    expect(serviceMocks.unbindSkillFromCliAgent).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: /API Agent/ }));
    expect(screen.queryByRole("checkbox", { name: "已启用" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "分配给 API Agent" }));
    await waitFor(() => expect(serviceMocks.bindSkillToApiAgent).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
      "api-agent",
    ));
    expect(serviceMocks.updateSkillMountPath).not.toHaveBeenCalled();
  });

  it("preserves existing Agent assignments when global enablement changes", async () => {
    const paused = {
      ...skill,
      enabled: false,
      boundAgentIds: ["claude-code", "codex-cli"],
      bindings: [
        { agentId: "claude-code", mountPath: ".claude/skills", mountedPath: ".claude/skills/reliable-skill", mounted: false },
        { agentId: "codex-cli", mountPath: ".codex/skills", mountedPath: ".codex/skills/reliable-skill", mounted: false },
      ],
    } satisfies Skill;
    serviceMocks.getSkillOverview.mockResolvedValue({ ...overview, skills: [paused] });
    serviceMocks.setSkillEnabled.mockResolvedValue({ ...paused, enabled: true });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("checkbox", { name: "已启用" }));
    await waitFor(() => expect(serviceMocks.setSkillEnabled).toHaveBeenCalledWith(
      paused.id,
      { scope: "global", workspacePath: null },
      true,
    ));
    expect(serviceMocks.bindSkillToCliAgent).not.toHaveBeenCalled();
    expect(serviceMocks.unbindSkillFromCliAgent).not.toHaveBeenCalled();
    expect(serviceMocks.bindSkillToApiAgent).not.toHaveBeenCalled();
    expect(serviceMocks.unbindSkillFromApiAgent).not.toHaveBeenCalled();
  });

  it("keeps the Skill visible and reports an actionable CLI mount-root failure", async () => {
    serviceMocks.bindSkillToCliAgent.mockRejectedValueOnce(new Error(
      "The Skill root for codex-cli is managed by an external directory link. Migrate the whole-directory link to a normal directory before assigning Skills.",
    ));
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Codex CLI/ }));
    await user.click(screen.getByRole("button", { name: "分配给 Codex CLI" }));

    expect((await screen.findByRole("alert")).textContent).toContain(
      "The Skill root for codex-cli is managed by an external directory link",
    );
    expect(screen.getByText("Reliable Skill")).toBeTruthy();
    expect(screen.getByRole("button", { name: "分配给 Codex CLI" })).toBeTruthy();
    const available = screen.getByRole("heading", { name: "可分配" }).closest("section");
    if (!available) throw new Error("Available panel not found");
    expect(within(available).getByText("Reliable Skill")).toBeTruthy();
    expect(serviceMocks.getSkillOverview).toHaveBeenCalledTimes(1);
  });

  it("keeps broken-root failures in Available while unrelated controls remain usable", async () => {
    const otherSkill = {
      ...skill,
      id: "unrelated-skill",
      metadata: {
        ...skill.metadata,
        id: "unrelated-skill",
        name: "Unrelated Skill",
      },
    } satisfies Skill;
    serviceMocks.getSkillOverview.mockResolvedValue({
      ...overview,
      skills: [skill, otherSkill],
      stats: { total: 2, enabled: 2, mounted: 0 },
      apiAgentBindings: { "reliable-skill": [], "unrelated-skill": [] },
    });
    serviceMocks.bindSkillToCliAgent.mockRejectedValueOnce(new Error(
      "The Skill root for codex-cli is a broken directory link. Repair or remove the stale link before assigning Skills.",
    ));
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Codex CLI/ }));
    const reliableRow = screen.getByText("Reliable Skill").closest("article");
    if (!reliableRow) throw new Error("Reliable Skill row not found");
    await user.click(within(reliableRow).getByRole("button", { name: "分配给 Codex CLI" }));

    expect((await within(reliableRow).findByRole("alert")).textContent).toContain(
      "broken directory link. Repair or remove the stale link",
    );
    const available = screen.getByRole("heading", { name: "可分配" }).closest("section");
    if (!available) throw new Error("Available panel not found");
    expect(within(available).getByText("Reliable Skill")).toBeTruthy();
    const unrelatedRow = within(available).getByText("Unrelated Skill").closest("article");
    if (!unrelatedRow) throw new Error("Unrelated Skill row not found");
    expect((within(unrelatedRow).getByRole("button", { name: "分配给 Codex CLI" }) as HTMLButtonElement).disabled).toBe(false);
    expect((screen.getByRole("button", { name: /Claude Code/ }) as HTMLButtonElement).disabled).toBe(false);
  });

  it("separates Agent types and shows the compact effective layer", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("Reliable Skill")).toBeTruthy();
    expect(screen.getAllByText("用户层").length).toBeGreaterThan(0);
    await user.click(screen.getByRole("button", { name: /Codex CLI/ }));
    const mountPanel = screen.getByText("Agent 挂载路径").closest("details");
    if (!mountPanel) throw new Error("Mount panel not found");
    expect(within(mountPanel).getByText("Codex CLI")).toBeTruthy();
    expect(within(mountPanel).queryByText("API Agent")).toBeNull();
    const board = screen.getByTestId("skill-selection-board");
    expect(board.className).toContain("xl:grid-cols-2");
    expect(Array.from(board.children).map((child) => child.getAttribute("data-skill-group"))).toEqual([
      "assigned",
      "available",
    ]);
    expect(screen.queryByRole("button", { name: "编辑 Skill" })).toBeNull();
    expect(screen.queryByRole("button", { name: "删除 Skill" })).toBeNull();
    expect(screen.queryByRole("checkbox", { name: "分配给 Codex CLI" })).toBeNull();
    await user.click(screen.getByRole("button", { name: /API Agent/ }));
    expect(screen.getByRole("button", { name: "分配给 API Agent" })).toBeTruthy();
    expect(screen.queryByRole("checkbox", { name: "已启用" })).toBeNull();
  });

  it("moves effective metadata into details while keeping immutable and Utility rows concise", async () => {
    const systemSkill = {
      ...skill,
      id: "system-role",
      source: "builtin" as const,
      metadata: {
        ...skill.metadata,
        id: "system-role",
        name: "System Role",
        type: "role" as const,
        delivery: "on-demand" as const,
        compatibilityDefaults: { skillType: false, delivery: false },
      },
      layer: "system" as const,
      origin: "shipped" as const,
      immutable: true,
      usage: { ...skill.usage, viewCount: 7, useCount: 3 },
    } satisfies Skill;
    const migratedOverride = {
      ...skill,
      id: "migrated-override",
      metadata: {
        ...skill.metadata,
        id: "migrated-override",
        name: "Migrated Override",
        type: "role" as const,
        delivery: "eager" as const,
        compatibilityDefaults: { skillType: true, delivery: true },
      },
      origin: "migrated" as const,
      shadowedDefinitions: [{ layer: "system" as const, origin: "shipped" as const, version: "1.0.0", availability: "available" as const }],
    } satisfies Skill;
    const utilitySkill = {
      ...skill,
      id: "utility-skill",
      metadata: { ...skill.metadata, id: "utility-skill", name: "Utility Skill", type: "utility" as const, delivery: "on-demand" as const },
      availability: "unsupported" as const,
      delegationCapability: { supported: true, reason: "available" as const },
    } satisfies Skill;
    serviceMocks.getSkillOverview.mockResolvedValue({
      ...overview,
      skills: [systemSkill, migratedOverride, utilitySkill],
      apiAgentBindings: { "system-role": [], "migrated-override": [], "utility-skill": [] },
    });
    serviceMocks.previewSkill.mockResolvedValue({
      id: systemSkill.id,
      scope: "global",
      workspacePath: null,
      path: `skill://${systemSkill.id}/`,
      content: "# System Role\n\nRead only.",
      layer: "system",
      origin: "shipped",
      availability: "available",
      immutable: true,
      shadowedDefinitions: [],
    });
    serviceMocks.loadSkill.mockResolvedValue({
      status: "loaded",
      result: {
        id: systemSkill.id,
        name: systemSkill.metadata.name,
        content: "Read only.",
        truncated: false,
        revision: systemSkill.contentHash,
        baseUri: `skill://${systemSkill.id}/`,
        resources: {
          scripts: [], references: [{ uri: `skill://${systemSkill.id}/references/guide.md`, relativePath: "references/guide.md", sizeBytes: 20 }], templates: [], assets: [], truncated: false,
        },
      },
    });
    const user = userEvent.setup();
    renderPage();

    const systemRow = (await screen.findByText("System Role")).closest("article");
    if (!systemRow) throw new Error("System row not found");
    expect(within(systemRow).getByText("系统层")).toBeTruthy();
    expect(within(systemRow).getByText("角色")).toBeTruthy();
    expect(within(systemRow).getByText("只读")).toBeTruthy();
    expect(within(systemRow).queryByText("按需加载")).toBeNull();
    expect(within(systemRow).queryByText("查看 7 次 · 使用 3 次")).toBeNull();
    expect(within(systemRow).queryByRole("button", { name: "编辑 Skill" })).toBeNull();
    expect(within(systemRow).queryByRole("button", { name: "删除 Skill" })).toBeNull();

    const systemDetailsButton = within(systemRow).getByRole("button", { name: "查看 System Role 详情" });
    await user.click(systemDetailsButton);
    const systemDetails = await screen.findByRole("dialog", { name: "System Role 详情" });
    expect(within(systemDetails).getByText("按需加载")).toBeTruthy();
    expect(within(systemDetails).getByText("查看 7 次 · 使用 3 次")).toBeTruthy();
    expect(await within(systemDetails).findByText(/已索引 1 个资源/)).toBeTruthy();
    expect(within(systemDetails).getByRole("heading", { name: "Overlay 治理" })).toBeTruthy();
    expect(serviceMocks.getSkillOverlayDetail).toHaveBeenCalledWith({ skillId: "system-role", scope: "user", workspacePath: null });
    expect(within(systemDetails).getByText(/系统包为只读内容/)).toBeTruthy();
    expect(serviceMocks.previewSkill).not.toHaveBeenCalled();
    await user.click(within(systemDetails).getByRole("button", { name: "关闭" }));
    expect(document.activeElement).toBe(systemDetailsButton);

    const overrideRow = screen.getByText("Migrated Override").closest("article");
    if (!overrideRow) throw new Error("Override row not found");
    const detailsButton = within(overrideRow).getByRole("button", { name: "查看 Migrated Override 详情" });
    expect(detailsButton.getAttribute("aria-expanded")).toBe("false");
    await user.click(detailsButton);
    expect(detailsButton.getAttribute("aria-expanded")).toBe("true");
    const details = await screen.findByRole("dialog", { name: "Migrated Override 详情" });
    expect(within(details).getByText("项目 > 用户 > 注册表 > 系统")).toBeTruthy();
    expect(within(details).getByRole("heading", { name: "定义优先级" })).toBeTruthy();
    expect(within(details).getByText("当前生效")).toBeTruthy();
    expect(within(details).getByText("已被遮蔽")).toBeTruthy();
    expect(within(details).getByText(/本地修改已保留为用户层覆盖/)).toBeTruthy();
    await user.click(within(details).getByRole("button", { name: "关闭" }));

    const utilityRow = screen.getByText("Utility Skill").closest("article");
    if (!utilityRow) throw new Error("Utility row not found");
    expect(within(utilityRow).getByText("可委托执行")).toBeTruthy();
    expect(within(utilityRow).queryByText(/受支持的原生 API Agent/)).toBeNull();
    await user.click(within(utilityRow).getByRole("button", { name: "查看 Utility Skill 详情" }));
    expect(await screen.findByText(/受支持的原生 API Agent 可以委托/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭" }));
    await user.click(screen.getByRole("button", { name: /API Agent/ }));
    const agentUtilityRow = screen.getByText("Utility Skill").closest("article");
    if (!agentUtilityRow) throw new Error("Agent Utility row not found");
    expect(within(agentUtilityRow).queryByRole("button", { name: "分配给 API Agent" })).toBeNull();

    await user.click(screen.getByRole("button", { name: /^全部 Skill/ }));
    const previewButton = within(screen.getByText("System Role").closest("article")!).getByRole("button", { name: "预览 Skill" });
    await user.click(previewButton);
    const dialog = await screen.findByRole("dialog", { name: "system-role" });
    expect(within(dialog).getByText(/已索引 1 个资源/)).toBeTruthy();
    expect(within(dialog).getByText(/系统包为只读内容/)).toBeTruthy();
    await user.click(within(dialog).getByRole("button", { name: "关闭" }));
    expect(document.activeElement).toBe(previewButton);
  }, 30_000);

  it("switches the wide inspector and clears stale selection after filtering", async () => {
    setWideDetailsLayout(true);
    const otherSkill = {
      ...skill,
      id: "other-skill",
      metadata: { ...skill.metadata, id: "other-skill", name: "Other Skill" },
    } satisfies Skill;
    serviceMocks.getSkillOverview.mockResolvedValue({
      ...overview,
      skills: [skill, otherSkill],
      stats: { total: 2, enabled: 2, mounted: 0 },
      apiAgentBindings: { [skill.id]: [], [otherSkill.id]: [] },
    });
    const user = userEvent.setup();
    renderPage();

    const reliableRow = (await screen.findByText("Reliable Skill")).closest("article");
    const otherRow = screen.getByText("Other Skill").closest("article");
    if (!reliableRow || !otherRow) throw new Error("Skill rows not found");
    await user.click(within(reliableRow).getByRole("button", { name: "查看 Reliable Skill 详情" }));
    expect(await screen.findByRole("complementary", { name: "Reliable Skill 详情" })).toBeTruthy();
    expect(reliableRow.getAttribute("data-selected")).toBe("true");

    await user.click(within(otherRow).getByRole("button", { name: "查看 Other Skill 详情" }));
    expect(screen.getByRole("complementary", { name: "Other Skill 详情" })).toBeTruthy();
    expect(reliableRow.getAttribute("data-selected")).toBe("false");
    expect(otherRow.getAttribute("data-selected")).toBe("true");

    await user.type(screen.getByPlaceholderText("按 ID、名称、分类、触发词或来源搜索"), "reliable-skill");
    await waitFor(() => expect(screen.queryByRole("complementary")).toBeNull());
    expect(serviceMocks.setSkillEnabled).not.toHaveBeenCalled();
    expect(serviceMocks.bindSkillToCliAgent).not.toHaveBeenCalled();
  });

  it("shows deterministic panel counts and both focused empty states", async () => {
    serviceMocks.getSkillOverview.mockResolvedValue({
      ...overview,
      apiAgentBindings: { [skill.id]: ["api-agent"] },
    });
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Codex CLI/ }));
    const codexAssigned = screen.getByRole("heading", { name: "已分配" }).closest("section");
    const codexAvailable = screen.getByRole("heading", { name: "可分配" }).closest("section");
    if (!codexAssigned || !codexAvailable) throw new Error("Codex selection panels not found");
    expect(within(codexAssigned).getByText("0")).toBeTruthy();
    expect(within(codexAssigned).getByText("当前 Agent 尚未分配 Skill。")).toBeTruthy();
    expect(within(codexAvailable).getByText("1")).toBeTruthy();
    expect(within(codexAvailable).getByText("Reliable Skill")).toBeTruthy();

    await user.click(screen.getByRole("button", { name: /API Agent/ }));
    const apiAssigned = screen.getByRole("heading", { name: "已分配" }).closest("section");
    const apiAvailable = screen.getByRole("heading", { name: "可分配" }).closest("section");
    if (!apiAssigned || !apiAvailable) throw new Error("API selection panels not found");
    expect(within(apiAssigned).getByText("1")).toBeTruthy();
    expect(within(apiAssigned).getByText("Reliable Skill")).toBeTruthy();
    expect(within(apiAvailable).getByText("0")).toBeTruthy();
    expect(within(apiAvailable).getByText("没有更多可分配的 Skill。")).toBeTruthy();
  });

  it("isolates pending assignment feedback and supports explicit removal", async () => {
    const assignedSkill = {
      ...skill,
      id: "assigned-skill",
      metadata: { ...skill.metadata, id: "assigned-skill", name: "Assigned Skill" },
      boundAgentIds: ["codex-cli"],
      bindings: [{
        agentId: "codex-cli",
        mountPath: ".codex/skills",
        mountedPath: ".codex/skills/assigned-skill",
        mounted: true,
      }],
    } satisfies Skill;
    const otherSkill = {
      ...skill,
      id: "other-skill",
      metadata: { ...skill.metadata, id: "other-skill", name: "Other Skill" },
    } satisfies Skill;
    serviceMocks.getSkillOverview.mockResolvedValue({ ...overview, skills: [assignedSkill, skill, otherSkill] });
    let finishBinding: ((value: Skill) => void) | undefined;
    serviceMocks.bindSkillToCliAgent.mockReturnValueOnce(new Promise<Skill>((resolve) => {
      finishBinding = resolve;
    }));
    const user = userEvent.setup();
    renderPage();

    await user.click(await screen.findByRole("button", { name: /Codex CLI/ }));
    const reliableRow = screen.getByText("Reliable Skill").closest("article");
    if (!reliableRow) throw new Error("Reliable Skill row not found");
    await user.click(within(reliableRow).getByRole("button", { name: "分配给 Codex CLI" }));
    expect(await screen.findByText("分配中")).toBeTruthy();
    expect((screen.getByText("分配中").closest("button") as HTMLButtonElement).disabled).toBe(true);
    const otherRow = screen.getByText("Other Skill").closest("article");
    if (!otherRow) throw new Error("Other Skill row not found");
    expect((within(otherRow).getByRole("button", { name: "分配给 Codex CLI" }) as HTMLButtonElement).disabled).toBe(false);
    finishBinding?.(skill);
    await waitFor(() => expect(screen.queryByText("分配中")).toBeNull());

    const assignedRow = screen.getByText("Assigned Skill").closest("article");
    if (!assignedRow) throw new Error("Assigned Skill row not found");
    await user.click(within(assignedRow).getByRole("button", { name: "取消分配给 Codex CLI" }));
    await waitFor(() => expect(serviceMocks.unbindSkillFromCliAgent).toHaveBeenCalledWith(
      assignedSkill.id,
      { scope: "global", workspacePath: null },
      "codex-cli",
    ));
  });

  it("keeps a failed mount migration visible outside the collapsed disclosure", async () => {
    serviceMocks.updateSkillMountPath.mockResolvedValueOnce({
      agentId: "codex-cli", oldMountPath: ".codex/skills", newMountPath: ".codex/skills-next",
      migrated: [], removed: [], overwritten: [], backedUp: [], failed: [{ skillId: skill.id, reason: "locked" }],
    });
    const user = userEvent.setup();
    renderPage();
    await user.click(await screen.findByRole("button", { name: /Codex CLI/ }));
    const details = screen.getByText("Agent 挂载路径").closest("details");
    if (!details) throw new Error("Mount disclosure not found");
    await user.click(within(details).getByText("Agent 挂载路径"));
    await user.clear(within(details).getByLabelText("Codex CLI Agent 挂载路径"));
    await user.type(within(details).getByLabelText("Codex CLI Agent 挂载路径"), ".codex/skills-next");
    await user.click(within(details).getByRole("button", { name: "保存" }));
    await waitFor(() => expect(serviceMocks.updateSkillMountPath).toHaveBeenCalled());
    expect(screen.getAllByRole("alert").some((alert) => alert.textContent?.includes("失败：1"))).toBe(true);
    expect(details.hasAttribute("open")).toBe(true);
    await user.click(within(details).getByText("Agent 挂载路径"));
    expect(details.hasAttribute("open")).toBe(false);
    expect(screen.getByRole("alert").textContent).toContain("失败：1");
  });

  it("routes the create dialog through the service boundary", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText("Reliable Skill");

    await user.click(screen.getByRole("button", { name: "新建 Skill" }));
    const createModal = modalFor("新建 Skill");
    await user.type(within(createModal).getByLabelText("ID"), "new-skill");
    await user.type(within(createModal).getByLabelText("名称"), "New Skill");
    await user.type(within(createModal).getByLabelText("正文"), "New body");
    await user.click(within(createModal).getByRole("button", { name: "保存" }));
    await waitFor(() => expect(serviceMocks.createSkill).toHaveBeenCalledWith(expect.objectContaining({
      id: "new-skill",
      body: "New body",
      scope: "global",
      workspacePath: null,
      metadata: expect.objectContaining({ type: "role", delivery: "on-demand" }),
    })));
  });

  it("routes the import dialog through the service boundary", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText("Reliable Skill");

    await user.click(screen.getByRole("button", { name: "导入 Skill" }));
    const importModal = modalFor("导入 Skill");
    await user.type(within(importModal).getByPlaceholderText("外部 Skill 目录"), "D:/external/imported-skill");
    await user.click(within(importModal).getByRole("button", { name: "导入" }));
    await waitFor(() => expect(serviceMocks.importSkill).toHaveBeenCalledWith({
      sourcePath: "D:/external/imported-skill",
      enabled: true,
      boundAgentIds: [],
      scope: "global",
      workspacePath: null,
    }));
  });

  it("routes the built-in restore dialog through the service boundary", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText("Reliable Skill");

    await user.click(screen.getByRole("button", { name: "恢复内置" }));
    const restoreModal = modalFor("恢复内置 Skill");
    expect(within(restoreModal).getByRole("option", { name: "code-review" })).toBeTruthy();
    await user.click(within(restoreModal).getByRole("button", { name: "恢复" }));
    await waitFor(() => expect(serviceMocks.restoreBuiltinSkill).toHaveBeenCalledWith("code-review"));
  });

  it("loads previews and edit hashes while confirming destructive deletion in an application dialog", async () => {
    const user = userEvent.setup();
    renderPage();
    await screen.findByText("Reliable Skill");

    await user.click(screen.getByRole("button", { name: "预览 Skill" }));
    expect(await screen.findByText(/Current body/)).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "源内容" }));
    expect(screen.getByText(/id: reliable-skill/)).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭" }));

    await user.click(screen.getByRole("button", { name: "编辑 Skill" }));
    const editModal = await waitFor(() => modalFor("编辑 Skill"));
    expect((within(editModal).getByLabelText("ID") as HTMLInputElement).disabled).toBe(true);
    const body = within(editModal).getByLabelText("正文");
    expect((body as HTMLTextAreaElement).value).toBe("Current body");
    await user.clear(body);
    await user.type(body, "Updated body");
    await user.click(within(editModal).getByRole("button", { name: "保存" }));
    await waitFor(() => expect(serviceMocks.updateSkill).toHaveBeenCalledWith(skill.id, expect.objectContaining({
      body: "Updated body",
      expectedContentHash: "current-hash",
    })));

    await user.click(screen.getByRole("button", { name: "删除 Skill" }));
    expect(serviceMocks.deleteSkill).not.toHaveBeenCalled();
    const deleteModal = modalFor("删除 Skill");
    await user.click(within(deleteModal).getByRole("button", { name: "确认删除" }));
    await waitFor(() => expect(serviceMocks.deleteSkill).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
    ));
  }, 10_000);

  it("reports an error status for its nav entry when the overview query fails, and null once healthy again (task 12.16)", async () => {
    serviceMocks.getSkillOverview.mockRejectedValueOnce(new Error("overview failed"));
    const onStatusChange = vi.fn();
    const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const user = userEvent.setup();
    render(
      <QueryClientProvider client={queryClient}>
        <SkillsPage onStatusChange={onStatusChange} searchTerm="" />
      </QueryClientProvider>,
    );

    await screen.findByText("overview failed");
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith({
      kind: "error",
      labelKey: "skills.status.error",
    }));

    await user.click(screen.getByRole("button", { name: "重试" }));
    await screen.findByText("Reliable Skill");
    await waitFor(() => expect(onStatusChange).toHaveBeenLastCalledWith(null));
  });
});
