// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import type { AgentRegistryEntry } from "../../types/agent";
import { ChatInputBox } from "./ChatInputBox";

const agent: AgentRegistryEntry = {
  id: "onepiece", displayName: "OnePiece", provider: "OnePiece", launch: { kind: "api" },
  supportedInteractionModes: ["api"], availabilityState: "available", capabilityTags: [], agentOrigin: "builtin",
};

describe("associated Plan navigation", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("exposes one keyboard-operable navigation action when an association exists", () => {
    const openPlan = vi.fn();
    render(<ChatInputBox agents={[agent]} availableModes={["plan", "execute"]} availableModels={[]} availableReasoning={[]} config={{ agentId: "onepiece", interactionMode: "api", executionMode: "execute", providerId: "onepiece", streaming: true, thinking: true, longContext: true }} fileReferenceCandidates={[]} fileReferences={[]} isStreaming={false} onAddFileReference={vi.fn()} onChange={vi.fn()} onClear={vi.fn()} onConfigAgentChange={vi.fn()} onConfigLongContextChange={vi.fn()} onConfigModeChange={vi.fn()} onConfigModelChange={vi.fn()} onConfigProviderChange={vi.fn()} onConfigReasoningChange={vi.fn()} onConfigStreamingChange={vi.fn()} onConfigThinkingChange={vi.fn()} onOpenPlan={openPlan} onRemoveFileReference={vi.fn()} onStop={vi.fn()} onSubmit={vi.fn()} value="" />);
    const button = screen.getByRole("button", { name: "Open Plan" });
    fireEvent.click(button);
    expect(openPlan).toHaveBeenCalledOnce();
  });
});
