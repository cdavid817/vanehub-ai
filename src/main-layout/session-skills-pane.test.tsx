// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { activateAppLanguage } from "../i18n";
import type { Session } from "../types/agent";
import type { Skill, SkillOverview, SkillScopeInput } from "../types/skill";

const service = vi.hoisted(() => ({
  bindSkillToApiAgent: vi.fn(), bindSkillToCliAgent: vi.fn(), createSkill: vi.fn(), deleteSkill: vi.fn(),
  getSkillOverview: vi.fn(), importSkill: vi.fn(), previewSkill: vi.fn(), restoreBuiltinSkill: vi.fn(),
  loadSkill: vi.fn(),
  setSkillEnabled: vi.fn(), syncSkillDrift: vi.fn(), unbindSkillFromApiAgent: vi.fn(),
  unbindSkillFromCliAgent: vi.fn(), updateSkill: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));
import { SessionSkillsPane } from "./session-skills-pane";

const projectPath = "D:/repo-worktree";

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: "session", title: "Session", agentId: "api-agent", interactionMode: "api", lifecycleState: "running",
    folder: null, projectPath: "D:/repo", worktreePath: projectPath, worktreeName: "feature", worktreeBranch: "feature/skill",
    remoteWorkspace: null, remoteSshConnectionId: null, remoteSshConnectionRevision: null, runtimeSessionId: null,
    categoryId: null, pinned: false, archived: false, createdAt: "now", updatedAt: "now", ...overrides,
  };
}

function makeSkill(id: string, scope: Skill["scope"]): Skill {
  return {
    id, scope, workspacePath: scope === "workspace" ? projectPath : null, source: "user", enabled: true,
    skillDir: id, skillMdPath: `${id}/SKILL.md`, contentHash: "hash", boundAgentIds: [], bindings: [],
    metadata: { id, name: id, description: `${id} description`, category: "testing", version: "1.0.0", triggers: [] },
    createdAt: "now", updatedAt: "now",
    layer: scope === "workspace" ? "project" : "user", origin: "created", trust: "trusted",
    availability: "available", immutable: false, shadowedDefinitions: [],
    usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
  };
}

const globalSkill = makeSkill("global-api", "global");
const projectSkill = makeSkill("project-api", "workspace");

function overview(scope: SkillScopeInput): SkillOverview {
  const workspace = scope.scope === "workspace";
  return {
    skills: workspace ? [projectSkill] : [globalSkill], stats: { total: 1, enabled: 1, mounted: 0 },
    agents: [{ id: "api-agent", displayName: "API Agent", kind: "api" }],
    apiAgentBindings: { [workspace ? projectSkill.id : globalSkill.id]: ["api-agent"] },
    mountPaths: [], restoreCandidates: [],
    drift: { ...scope, issues: [], driftHash: "clean" },
  };
}

function renderPane(session: Session | null = makeSession(), onOpenSkillSettings = vi.fn()) {
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  return {
    ...render(<QueryClientProvider client={client}><SessionSkillsPane activeSession={session} onOpenSkillSettings={onOpenSkillSettings} /></QueryClientProvider>),
    onOpenSkillSettings,
  };
}

beforeEach(async () => {
  vi.clearAllMocks();
  await activateAppLanguage("en");
  service.getSkillOverview.mockImplementation(async (scope: SkillScopeInput) => overview(scope));
  service.previewSkill.mockResolvedValue({
    id: projectSkill.id, scope: "workspace", workspacePath: projectPath, path: projectSkill.skillMdPath,
    content: "# project-api\n\nBody", layer: "project", origin: "created", availability: "available",
    immutable: false, shadowedDefinitions: [],
  });
  service.loadSkill.mockResolvedValue({ status: "refused", refusal: {
    requested: projectSkill.id, canonicalId: projectSkill.id, reason: "not-found", conflictingIds: [],
  } });
  service.setSkillEnabled.mockResolvedValue(projectSkill);
  service.bindSkillToApiAgent.mockResolvedValue(undefined);
  service.createSkill.mockResolvedValue(projectSkill);
  service.importSkill.mockResolvedValue(projectSkill);
  service.deleteSkill.mockResolvedValue(undefined);
  service.updateSkill.mockResolvedValue(projectSkill);
});

describe("SessionSkillsPane", () => {
  it("uses worktree context and keeps effective, global, and project views available", async () => {
    const { onOpenSkillSettings } = renderPane();
    expect((await screen.findAllByText("global-api")).length).toBeGreaterThan(0);
    expect(screen.getAllByText("project-api").length).toBeGreaterThan(0);
    expect(screen.getByRole("tab", { name: "Effective" }).textContent).toContain("(2)");
    expect(screen.getByRole("tab", { name: "Global" }).textContent).toContain("(1)");
    expect(screen.getByRole("tab", { name: "Project" }).textContent).toContain("(1)");
    expect(service.getSkillOverview).toHaveBeenCalledWith({ scope: "workspace", workspacePath: projectPath });

    const user = userEvent.setup();
    await user.click(screen.getByRole("tab", { name: "Global" }));
    await user.click(screen.getByRole("button", { name: "Manage global" }));
    expect(onOpenSkillSettings).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("tab", { name: "Project" }));
    expect(screen.getByText(projectPath)).toBeTruthy();
  });

  it("routes project lifecycle and API prompt assignment through workspace-scoped service calls", async () => {
    const user = userEvent.setup();
    renderPane();
    await user.click(await screen.findByRole("tab", { name: "Project" }));
    expect((await screen.findAllByText("project-api")).length).toBeGreaterThan(0);

    await user.click(screen.getByRole("checkbox", { name: "Enabled" }));
    await waitFor(() => expect(service.setSkillEnabled).toHaveBeenCalledWith(projectSkill.id, { scope: "workspace", workspacePath: projectPath }, false));
    await user.click(screen.getByRole("button", { name: "Unassign" }));
    await waitFor(() => expect(service.unbindSkillFromApiAgent).toHaveBeenCalledWith(projectSkill.id, { scope: "workspace", workspacePath: projectPath }, "api-agent"));
    expect(service.unbindSkillFromCliAgent).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Preview Skill" }));
    expect(await screen.findByText("Body")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "Close" }));
    await user.click(screen.getByRole("button", { name: "Delete Skill" }));
    const dialog = screen.getByRole("dialog");
    await user.click(within(dialog).getByRole("button", { name: "Delete" }));
    await waitFor(() => expect(service.deleteSkill).toHaveBeenCalledWith(projectSkill.id, { scope: "workspace", workspacePath: projectPath }));
  });

  it("provides project create and import dialogs without a manual workspace selector", async () => {
    const user = userEvent.setup();
    renderPane();
    await user.click(await screen.findByRole("tab", { name: "Project" }));
    await user.click(screen.getByRole("button", { name: "Create Skill" }));
    const create = screen.getByRole("dialog");
    await user.type(within(create).getByLabelText("ID"), "new-project");
    await user.type(within(create).getByLabelText("Name"), "New Project");
    await user.click(within(create).getByRole("button", { name: "Save" }));
    await waitFor(() => expect(service.createSkill).toHaveBeenCalledWith(expect.objectContaining({ id: "new-project", scope: "workspace", workspacePath: projectPath })));

    await user.click(screen.getByRole("button", { name: "Import Skill" }));
    const imported = screen.getByRole("dialog");
    await user.type(within(imported).getByPlaceholderText("External Skill directory"), "D:/external");
    await user.click(within(imported).getByRole("button", { name: "Import" }));
    await waitFor(() => expect(service.importSkill).toHaveBeenCalledWith(expect.objectContaining({ sourcePath: "D:/external", scope: "workspace", workspacePath: projectPath })));
    expect(screen.queryByPlaceholderText("Select a local project directory")).toBeNull();
  });

  it("blocks project mutations when the session has no project context", async () => {
    const user = userEvent.setup();
    renderPane(makeSession({ projectPath: null, worktreePath: null }));
    await user.click(screen.getByRole("tab", { name: "Project" }));
    expect(screen.getByText("No project selected, so no project Skills are available")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "Create Skill" })).toBeNull();
    expect(service.getSkillOverview).not.toHaveBeenCalledWith({ scope: "workspace", workspacePath: null });
  });

  it("uses CLI binding operations for a CLI session", async () => {
    const user = userEvent.setup();
    renderPane(makeSession({ agentId: "codex-cli", interactionMode: "cli" }));
    await user.click(await screen.findByRole("tab", { name: "Project" }));
    await user.click(await screen.findByRole("button", { name: "Assign" }));
    await waitFor(() => expect(service.bindSkillToCliAgent).toHaveBeenCalledWith(
      projectSkill.id,
      { scope: "workspace", workspacePath: projectPath },
      "codex-cli",
    ));
    expect(service.bindSkillToApiAgent).not.toHaveBeenCalled();
  });
});
