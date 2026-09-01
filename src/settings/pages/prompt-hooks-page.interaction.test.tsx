// @vitest-environment jsdom

import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AgentService } from "../../services/agent-service";
import type { AgentRegistryEntry } from "../../types/agent";
import type {
  PromptHook,
  PromptHookDraft,
  PromptHookListResult,
  SavePromptHookDraftInput,
} from "../../types/prompt-hook";
import { createAgentServiceDouble, renderWithAppProviders } from "../../test/render";
import { PromptHooksPage } from "./prompt-hooks-page";

describe("PromptHooksPage interactions", () => {
  it("separates compact management from lazily loaded runtime records", async () => {
    const listTraces = vi.fn(async () => [{
      id: "trace-1",
      hookId: "user-review-focus",
      category: "dynamic" as const,
      stage: "per-turn" as const,
      status: "skipped" as const,
      reason: "not-bound",
      agentId: "codex-cli" as const,
      createdAt: "2026-08-21T00:00:00.000Z",
    }]);
    const previewAssembly = vi.fn(async () => ({
      agentId: "codex-cli" as const,
      renderedContent: "Assembled prompt",
      trace: [],
    }));
    const service = promptHookService(() => [userHook(), builtinHook()], vi.fn(), {
      listPromptHookTraces: listTraces,
      previewPromptAssembly: previewAssembly,
    });
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    expect(await screen.findByText("显示 2 / 2 · 已启用 2 · 自定义 1")).toBeTruthy();
    expect(listTraces).not.toHaveBeenCalled();
    await user.click(screen.getByRole("tab", { name: "运行记录" }));
    expect(await screen.findByText("not-bound")).toBeTruthy();
    expect(listTraces).toHaveBeenCalledOnce();
    await user.click(screen.getByRole("button", { name: "预览组装" }));
    expect(await screen.findByText("Assembled prompt")).toBeTruthy();
    expect(previewAssembly).toHaveBeenCalledOnce();
  });

  it("uses progressive filters and accessible category expansion", async () => {
    const service = promptHookService(() => [userHook(), builtinHook()], vi.fn());
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    await screen.findByRole("button", { name: "打开 Review Focus 的详情" });
    await user.click(screen.getByText("更多筛选"));
    await user.selectOptions(screen.getByLabelText("全部来源"), "user");
    expect(screen.getByText("已启用 1 项")).toBeTruthy();
    expect(screen.queryByRole("button", { name: "打开 Runtime Boundary 的详情" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "清除更多筛选" }));
    expect(await screen.findByRole("button", { name: "打开 Runtime Boundary 的详情" })).toBeTruthy();

    const collapse = screen.getByRole("button", { name: "折叠 Dynamic，2 个 Hook" });
    await user.click(collapse);
    expect(screen.queryByRole("button", { name: "打开 Review Focus 的详情" })).toBeNull();
    await user.click(screen.getByRole("button", { name: "展开 Dynamic，2 个 Hook" }));
    expect(await screen.findByRole("button", { name: "打开 Review Focus 的详情" })).toBeTruthy();
  });

  it("updates stable CLI bindings from the unified overview", async () => {
    const setBindings = vi.fn(async () => ({ ...userHook(), cliBindings: [] }));
    const service = promptHookService(() => [userHook()], vi.fn(), { setPromptHookCliBindings: setBindings });
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const row = await hookRow("Review Focus");
    await user.click(within(row).getByRole("button", { name: "打开 Review Focus 的详情" }));
    await user.click(screen.getByRole("checkbox", { name: "Codex CLI" }));

    expect(setBindings).toHaveBeenCalledWith("user-review-focus", []);
  });

  it("previews and updates a user Prompt Hook through the service boundary", async () => {
    const hooks = [userHook(), builtinHook()];
    const saveDraft = vi.fn(async (input: SavePromptHookDraftInput) => {
      return {
        hookId: input.hookId,
        revision: 1,
        input: input.draft,
        createdAt: "2026-07-23T01:00:00.000Z",
        updatedAt: "2026-07-23T01:00:00.000Z",
      };
    });
    const service = promptHookService(() => hooks, saveDraft);
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookRow("Review Focus");
    await user.click(within(card).getByLabelText("Review Focus 的更多操作"));
    await user.click(within(card).getByRole("button", { name: "预览 Hook 内容" }));
    expect(await screen.findByText("Rendered preview")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭" }));

    await user.click(within(card).getByRole("button", { name: "打开 Review Focus 的详情" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "Updated Review Focus");
    await user.click(screen.getByRole("tab", { name: "内容与发布" }));
    await user.clear(screen.getByLabelText("模板正文"));
    await user.click(screen.getByLabelText("模板正文"));
    await user.paste("Updated {{sampleInput}}");
    await user.click(screen.getByRole("button", { name: "保存草稿" }));

    expect(await screen.findByText("Review Focus")).toBeTruthy();
    expect(saveDraft).toHaveBeenCalledWith({
      hookId: "user-review-focus",
      expectedRevision: null,
      draft: expect.objectContaining({
        name: "Updated Review Focus",
        templateBody: "Updated {{sampleInput}}",
      }),
    });
  }, 30_000);

  it("keeps edited values visible when the service rejects a save", async () => {
    const saveDraft = vi.fn(async () => {
      throw new Error("service unavailable");
    });
    const service = promptHookService(() => [userHook()], saveDraft);
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookRow("Review Focus");
    await user.click(within(card).getByRole("button", { name: "打开 Review Focus 的详情" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "Unsaved Review");
    await user.click(screen.getByRole("tab", { name: "内容与发布" }));
    await user.click(screen.getByRole("button", { name: "保存草稿" }));

    expect(await screen.findByText("请检查输入后重试。")).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "基本设置" }));
    expect(screen.getByLabelText("名称")).toHaveProperty("value", "Unsaved Review");
    expect(saveDraft).toHaveBeenCalledOnce();
  }, 20_000);

  it("localizes validation errors returned by the service boundary", async () => {
    const saveDraft = vi.fn(async () => {
      throw new Error("Prompt Hook name is required");
    });
    const service = promptHookService(() => [userHook()], saveDraft);
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookRow("Review Focus");
    await user.click(within(card).getByRole("button", { name: "打开 Review Focus 的详情" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.click(screen.getByRole("tab", { name: "内容与发布" }));
    await user.click(screen.getByRole("button", { name: "保存草稿" }));

    expect(await screen.findByText("请填写 Hook 名称。")).toBeTruthy();
    await user.click(screen.getByRole("tab", { name: "基本设置" }));
    expect(screen.getByLabelText("名称")).toHaveProperty("value", "");
  }, 20_000);

  it("flags the shell status once the runtime trace query fails", async () => {
    const onStatusChange = vi.fn();
    const service = promptHookService(() => [userHook()], vi.fn(), {
      listPromptHookTraces: vi.fn(async () => {
        throw new Error("trace fetch failed");
      }),
    });
    const { user } = renderWithAppProviders(
      <PromptHooksPage onStatusChange={onStatusChange} searchTerm="" service={service} />,
    );

    await waitFor(() => {
      expect(onStatusChange).toHaveBeenCalledWith(null);
    });

    await user.click(screen.getByRole("tab", { name: "运行记录" }));

    await waitFor(() => {
      expect(onStatusChange).toHaveBeenCalledWith({ kind: "error", labelKey: "promptHooks.status.error" });
    });
  });

  it("does not expose mutation controls for an immutable built-in Prompt Hook", async () => {
    const service = promptHookService(() => [builtinHook()], vi.fn());
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookRow("Runtime Boundary");
    expect(within(card).getByRole("checkbox", { name: "已启用" })).toHaveProperty("disabled", true);
    await user.click(within(card).getByRole("button", { name: "打开 Runtime Boundary 的详情" }));
    expect(screen.queryByRole("button", { name: "删除" })).toBeNull();
    expect(screen.queryByRole("tab", { name: "版本历史" })).toBeNull();
  }, 20_000);
});

async function hookRow(name: string) {
  const trigger = await screen.findByRole("button", { name: `打开 ${name} 的详情` }, { timeout: 5_000 });
  const row = trigger.closest("article");
  if (!row) throw new Error(`Missing Prompt Hook row: ${name}`);
  return row;
}

function promptHookService(
  readHooks: () => PromptHook[],
  savePromptHookDraft: (input: SavePromptHookDraftInput) => Promise<PromptHookDraft>,
  overrides: Partial<AgentService> = {},
) {
  return createAgentServiceDouble({
    listAgents: async () => [agent],
    listPromptHooks: async (): Promise<PromptHookListResult> => {
      const hooks = readHooks();
      return {
        hooks,
        stats: {
          total: hooks.length,
          enabled: hooks.filter((hook) => hook.enabled).length,
          builtin: hooks.filter((hook) => hook.source === "builtin").length,
          user: hooks.filter((hook) => hook.source === "user").length,
        },
      };
    },
    listPromptHookTraces: async () => [],
    previewPromptHook: async (input) => ({
      hookId: input.hookId,
      agentId: input.agentId,
      renderedContent: "Rendered preview",
      trace: [],
    }),
    getPromptHookVersionHistory: async (hookId) => ({
      hookId,
      publishedVersion: readHooks().find((hook) => hook.id === hookId)?.version ?? null,
      draft: null,
      versions: [],
      evaluations: [],
    }),
    savePromptHookDraft,
    ...overrides,
  });
}

const agent: AgentRegistryEntry = {
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
  launch: { kind: "cli", executableName: "codex" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: [],
  agentOrigin: "builtin",
};

function userHook(): PromptHook {
  return {
    id: "user-review-focus",
    name: "Review Focus",
    description: "Focus review output.",
    category: "dynamic",
    stage: "per-turn",
    order: 500,
    version: 1,
    source: "user",
    enabled: true,
    disableable: true,
    cliBindings: ["codex-cli"],
    governance: { safetyTier: "editable", transparencyTier: "opt-in-view", governanceTier: "human-gated" },
    templateBody: "Review {{sampleInput}}",
    createdAt: "2026-07-23T00:00:00.000Z",
    updatedAt: "2026-07-23T00:00:00.000Z",
  };
}

function builtinHook(): PromptHook {
  return {
    ...userHook(),
    id: "law-runtime-boundary",
    name: "Runtime Boundary",
    source: "builtin",
    disableable: false,
    governance: { safetyTier: "readonly", transparencyTier: "visible-by-default", governanceTier: "immutable" },
  };
}
