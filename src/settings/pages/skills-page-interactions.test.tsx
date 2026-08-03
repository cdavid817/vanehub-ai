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
  getSkillOverview: vi.fn(),
  importSkill: vi.fn(),
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

function modalFor(title: string): HTMLElement {
  const modal = screen.getByRole("heading", { level: 3, name: title }).closest("section");
  if (!modal) throw new Error(`Modal not found: ${title}`);
  return modal;
}

function resolvedSkillMocks() {
  serviceMocks.setSkillEnabled.mockResolvedValue(skill);
  serviceMocks.bindSkillToCliAgent.mockResolvedValue(skill);
  serviceMocks.bindSkillToApiAgent.mockResolvedValue(undefined);
  serviceMocks.createSkill.mockResolvedValue(skill);
  serviceMocks.importSkill.mockResolvedValue(skill);
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
  serviceMocks.getSkillOverview.mockResolvedValue(overview);
  serviceMocks.previewSkill.mockResolvedValue({
    id: skill.id,
    scope: "global",
    workspacePath: null,
    path: skill.skillMdPath,
    content: "---\nid: reliable-skill\n---\n# Reliable Skill\n\nCurrent body",
  });
  resolvedSkillMocks();
});

describe("SkillsPage interactions", () => {
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

  it("separates Agent types and shows the Skill source", async () => {
    const user = userEvent.setup();
    renderPage();

    expect(await screen.findByText("Reliable Skill")).toBeTruthy();
    expect(screen.getAllByText("用户").length).toBeGreaterThan(0);
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
});
