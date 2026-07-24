// @vitest-environment jsdom

import { screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
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

    const card = await hookCard("Review Focus");
    await user.click(within(card).getByRole("button", { name: "预览 Hook 内容" }));
    expect(await screen.findByText("Rendered preview")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "关闭" }));

    await user.click(within(card).getByRole("button", { name: "编辑 Hook" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "Updated Review Focus");
    await user.clear(screen.getByLabelText("模板正文"));
    await user.click(screen.getByLabelText("模板正文"));
    await user.paste("Updated {{sampleInput}}");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(await screen.findByText("Review Focus")).toBeTruthy();
    expect(saveDraft).toHaveBeenCalledWith({
      hookId: "user-review-focus",
      expectedRevision: null,
      draft: expect.objectContaining({
        name: "Updated Review Focus",
        templateBody: "Updated {{sampleInput}}",
        version: 1,
      }),
    });
  }, 30_000);

  it("keeps edited values visible when the service rejects a save", async () => {
    const saveDraft = vi.fn(async () => {
      throw new Error("service unavailable");
    });
    const service = promptHookService(() => [userHook()], saveDraft);
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookCard("Review Focus");
    await user.click(within(card).getByRole("button", { name: "编辑 Hook" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.type(screen.getByLabelText("名称"), "Unsaved Review");
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(await screen.findByText("请检查输入后重试。")).toBeTruthy();
    expect(screen.getByLabelText("名称")).toHaveProperty("value", "Unsaved Review");
    expect(saveDraft).toHaveBeenCalledOnce();
  }, 20_000);

  it("localizes validation errors returned by the service boundary", async () => {
    const saveDraft = vi.fn(async () => {
      throw new Error("Prompt Hook name is required");
    });
    const service = promptHookService(() => [userHook()], saveDraft);
    const { user } = renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookCard("Review Focus");
    await user.click(within(card).getByRole("button", { name: "编辑 Hook" }));
    await user.clear(screen.getByLabelText("名称"));
    await user.click(screen.getByRole("button", { name: "保存" }));

    expect(await screen.findByText("请填写 Hook 名称。")).toBeTruthy();
    expect(screen.getByLabelText("名称")).toHaveProperty("value", "");
  }, 20_000);

  it("does not expose mutation controls for an immutable built-in Prompt Hook", async () => {
    const service = promptHookService(() => [builtinHook()], vi.fn());
    renderWithAppProviders(<PromptHooksPage searchTerm="" service={service} />);

    const card = await hookCard("Runtime Boundary");
    expect(within(card).queryByRole("button", { name: "编辑 Hook" })).toBeNull();
    expect(within(card).queryByRole("button", { name: "删除 Hook" })).toBeNull();
    expect(within(card).getByRole("checkbox", { name: "已启用" })).toHaveProperty("disabled", true);
  }, 20_000);
});

async function hookCard(name: string) {
  const title = await screen.findByRole("heading", { name }, { timeout: 5_000 });
  const card = title.closest("section");
  if (!card) throw new Error(`Missing Prompt Hook card: ${name}`);
  return card;
}

function promptHookService(
  readHooks: () => PromptHook[],
  savePromptHookDraft: (input: SavePromptHookDraftInput) => Promise<PromptHookDraft>,
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
