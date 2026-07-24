import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PromptHookMutationInput } from "../types/prompt-hook";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));

import { tauriAgentClient } from "./tauri-agent-client";

const draft: PromptHookMutationInput = {
  id: "advanced-hook",
  name: "Advanced Hook",
  description: "Fixture",
  category: "dynamic",
  stage: "per-turn",
  order: 480,
  templateBody: "{{agent_name}} at {{current_time}}",
  enabled: true,
  cliBindings: ["codex-cli"],
  governance: {
    safetyTier: "editable",
    transparencyTier: "opt-in-view",
    governanceTier: "human-gated",
  },
};

describe("Tauri advanced Prompt Hook adapter", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    invokeMock.mockResolvedValue({});
  });

  it("maps every advanced operation to one bounded command", async () => {
    const save = { hookId: draft.id, expectedRevision: null, draft };
    const publish = { hookId: draft.id, expectedDraftRevision: 1, expectedPublishedVersion: 1 };
    const rollback = { hookId: draft.id, version: 1, expectedPublishedVersion: 2 };

    await tauriAgentClient.listPromptHookVariables();
    await tauriAgentClient.savePromptHookDraft(save);
    await tauriAgentClient.publishPromptHook(publish);
    await tauriAgentClient.getPromptHookVersionHistory(draft.id);
    await tauriAgentClient.rollbackPromptHook(rollback);

    expect(invokeMock.mock.calls).toEqual([
      ["list_prompt_hook_variables"],
      ["save_prompt_hook_draft", { input: save }],
      ["publish_prompt_hook", { input: publish }],
      ["get_prompt_hook_version_history", { hookId: draft.id }],
      ["rollback_prompt_hook", { input: rollback }],
    ]);
  });
});
