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
    { id: "api-agent", displayName: "API Agent", kind: "api" },
  ],
  mountPaths: [{ agentId: "codex-cli", mountPath: ".codex/skills", isDefault: true }],
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
    renderPage();

    expect(await screen.findByText("overview failed")).toBeTruthy();
    expect(screen.queryByText("没有匹配的 Skill。")).toBeNull();
    expect(screen.queryByText("Skill 配置已同步。")).toBeNull();
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
    await user.click(screen.getByRole("checkbox", { name: "Codex CLI" }));
    await waitFor(() => expect(serviceMocks.bindSkillToCliAgent).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
      "codex-cli",
    ));
    await user.click(screen.getByRole("checkbox", { name: "API Agent" }));
    await waitFor(() => expect(serviceMocks.bindSkillToApiAgent).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
      "api-agent",
    ));
    expect(serviceMocks.updateSkillMountPath).not.toHaveBeenCalled();
  });

  it("separates Agent types and shows the Skill source", async () => {
    renderPage();

    expect(await screen.findByText("Reliable Skill")).toBeTruthy();
    expect(screen.getByText("用户")).toBeTruthy();
    const mountPanel = screen.getByText("Agent 挂载路径").closest("section");
    if (!mountPanel) throw new Error("Mount panel not found");
    expect(within(mountPanel).getByText("Codex CLI")).toBeTruthy();
    expect(within(mountPanel).queryByText("API Agent")).toBeNull();
    expect(screen.getByRole("checkbox", { name: "API Agent" })).toBeTruthy();
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

  it("loads previews and edit hashes while confirming destructive deletion", async () => {
    const user = userEvent.setup();
    const confirm = vi.spyOn(globalThis, "confirm").mockReturnValueOnce(false).mockReturnValueOnce(true);
    renderPage();
    await screen.findByText("Reliable Skill");

    await user.click(screen.getByRole("button", { name: "预览 Skill" }));
    expect(await screen.findByText(/Current body/)).toBeTruthy();
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
    expect(confirm).toHaveBeenCalledTimes(1);
    expect(serviceMocks.deleteSkill).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: "删除 Skill" }));
    await waitFor(() => expect(serviceMocks.deleteSkill).toHaveBeenCalledWith(
      skill.id,
      { scope: "global", workspacePath: null },
    ));
  });
});
