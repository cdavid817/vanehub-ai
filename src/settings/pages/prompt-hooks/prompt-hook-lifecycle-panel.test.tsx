// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { createAgentServiceDouble, renderWithAppProviders } from "../../../test/render";
import type { PromptHook, PromptHookVersionHistory } from "../../../types/prompt-hook";
import { PromptHookLifecyclePanel } from "./prompt-hook-lifecycle-panel";

describe("PromptHookLifecyclePanel", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows variables, draft state, version metrics, publishing, and confirmed rollback", async () => {
    const publish = vi.fn(async () => history.versions[0]);
    const rollback = vi.fn(async () => ({
      ...history.versions[0],
      version: 3,
      publicationKind: "rollback" as const,
      rollbackFromVersion: 1,
    }));
    const service = createAgentServiceDouble({
      getPromptHookVersionHistory: async () => history,
      listPromptHookVariables: async () => [{
        name: "agent_name",
        token: "{{agent_name}}",
        descriptionKey: "promptHooks.variables.agentName",
        availabilityKey: "promptHooks.variables.availability.agent",
        example: "Codex CLI",
        aliases: [],
      }],
      publishPromptHook: publish,
      rollbackPromptHook: rollback,
    });
    vi.spyOn(window, "confirm").mockReturnValue(true);
    const { user } = renderWithAppProviders(
      <PromptHookLifecyclePanel
        hook={hook}
        onChanged={() => undefined}
        onClose={() => undefined}
        service={service}
      />,
    );

    expect(await screen.findByText("草稿修订 2", {}, { timeout: 20_000 })).toBeTruthy();
    expect(screen.getByText("v2 · 当前版本")).toBeTruthy();
    expect(screen.getByText("50%")).toBeTruthy();
    expect(screen.getByText("200 ms")).toBeTruthy();
    expect(screen.getByText("1/1")).toBeTruthy();
    expect(screen.getByText("当前 Agent 的显示名称")).toBeTruthy();
    expect(screen.getByText("选择 Agent 后可用")).toBeTruthy();
    expect(screen.getByText("Codex CLI")).toBeTruthy();
    await user.click(screen.getByRole("button", { name: "{{agent_name}}" }));
    expect(screen.getByLabelText("模板正文")).toHaveProperty(
      "value",
      "Draft {{sample_input}} {{agent_name}}",
    );

    await user.click(screen.getByRole("button", { name: "发布" }));
    expect(publish).toHaveBeenCalledWith({
      hookId: hook.id,
      expectedDraftRevision: 2,
      expectedPublishedVersion: 2,
    });
    await user.click(screen.getByRole("button", { name: "回滚" }));
    expect(window.confirm).toHaveBeenCalled();
    expect(rollback).toHaveBeenCalledWith({
      hookId: hook.id,
      version: 1,
      expectedPublishedVersion: 2,
    });
  }, 30_000);

  it("clears a stale publication error when the draft changes and saves", async () => {
    const save = vi.fn(async () => history.draft!);
    const service = createAgentServiceDouble({
      getPromptHookVersionHistory: async () => history,
      listPromptHookVariables: async () => [],
      publishPromptHook: async () => {
        throw new Error("Unsupported Prompt Hook variables: stale_variable");
      },
      savePromptHookDraft: save,
    });
    const { user } = renderWithAppProviders(
      <PromptHookLifecyclePanel
        hook={hook}
        onChanged={() => undefined}
        onClose={() => undefined}
        service={service}
      />,
    );

    await screen.findByText("草稿修订 2", {}, { timeout: 20_000 });
    await user.click(screen.getByRole("button", { name: "发布" }));
    expect(await screen.findByText(/stale_variable/)).toBeTruthy();

    const template = screen.getByLabelText("模板正文");
    await user.clear(template);
    await user.type(template, "Valid {{agent_name}}");
    await user.click(screen.getByRole("button", { name: "保存草稿" }));

    await waitFor(() => expect(save).toHaveBeenCalled());
    expect(screen.queryByText(/stale_variable/)).toBeNull();
  }, 30_000);
});

const hook: PromptHook = {
  id: "user-review-focus",
  name: "Review Focus",
  description: "Focus review output.",
  category: "dynamic",
  stage: "per-turn",
  order: 500,
  version: 2,
  source: "user",
  enabled: true,
  disableable: true,
  cliBindings: ["codex-cli"],
  governance: {
    safetyTier: "editable",
    transparencyTier: "visible-by-default",
    governanceTier: "human-gated",
  },
  templateBody: "Published {{sample_input}}",
  createdAt: "2026-07-23T00:00:00.000Z",
  updatedAt: "2026-07-23T00:00:00.000Z",
};

const history: PromptHookVersionHistory = {
  hookId: hook.id,
  publishedVersion: 2,
  draft: {
    hookId: hook.id,
    revision: 2,
    input: {
      id: hook.id,
      name: hook.name,
      description: hook.description,
      category: hook.category,
      stage: hook.stage,
      order: hook.order,
      templateBody: "Draft {{sample_input}}",
      enabled: true,
      cliBindings: ["codex-cli"],
      governance: hook.governance,
    },
    createdAt: "2026-07-23T00:00:00.000Z",
    updatedAt: "2026-07-23T01:00:00.000Z",
  },
  versions: [
    {
      hookId: hook.id,
      version: 2,
      contentHash: "hash-2",
      publicationKind: "publish",
      rollbackFromVersion: null,
      publishedAt: "2026-07-23T02:00:00.000Z",
    },
    {
      hookId: hook.id,
      version: 1,
      contentHash: "hash-1",
      publicationKind: "publish",
      rollbackFromVersion: null,
      publishedAt: "2026-07-23T01:00:00.000Z",
    },
  ],
  evaluations: [{
    hookId: hook.id,
    version: 2,
    executionCount: 3,
    succeededCount: 1,
    failedCount: 1,
    cancelledCount: 1,
    successRate: 0.5,
    averageElapsedMs: 200,
    minimumElapsedMs: 100,
    maximumElapsedMs: 300,
  }],
};
