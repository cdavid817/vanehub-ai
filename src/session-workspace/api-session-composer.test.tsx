// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { screen } from "@testing-library/react";
import { activateAppLanguage } from "../i18n";
import { NotificationProvider } from "../notifications/notification-provider";
import { renderWithAppProviders } from "../test/render";
import type { MainLayoutModel } from "../main-layout/use-main-layout-model";
import { ApiSessionComposer } from "./api-session-composer";

// `canSendToSession` gates the send button on a clean, idle session, so a fixture missing these
// two fields renders every case with the button disabled and no click reaching the composer.
const sendable = { recoveryStatus: "clean", activeExecutionRunId: null, archived: false };

function model(overrides: Record<string, unknown> = {}) {
  const submit = vi.fn();
  const base = {
    activeSession: { id: "session-1", title: "S", agentId: "onepiece", interactionMode: "api", lifecycleState: "idle", ...sendable },
    agents: [], draft: "", fileReferenceCandidates: [], fileReferences: [],
    isSending: false, isStreaming: false, messages: [],
    chatConfig: {
      availableAgents: [], availableModes: ["inherit"], availableModels: [], availableReasoning: ["low"],
      config: { agentId: "onepiece", interactionMode: "api", executionMode: "inherit", streaming: true, thinking: false, longContext: false },
      changeAgent: vi.fn(), changeModel: vi.fn(), changeProvider: vi.fn(),
      setLongContext: vi.fn(), setReasoningDepth: vi.fn(), setSessionExecutionMode: vi.fn(),
      setStreaming: vi.fn(), setThinking: vi.fn(),
    },
    addFileReference: vi.fn(), removeFileReference: vi.fn(), exportSession: vi.fn(),
    setDraft: vi.fn(), stop: vi.fn(), submit, submitWithRunner: submit,
    ...overrides,
  };
  return base as unknown as MainLayoutModel;
}

function renderComposer(target: MainLayoutModel) {
  return renderWithAppProviders(
    <NotificationProvider>
      <ApiSessionComposer model={target} />
    </NotificationProvider>,
  );
}

describe("ApiSessionComposer slash dispatch", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("sends ordinary prose to the model", async () => {
    const target = model({ draft: "hello" });
    const { user } = renderComposer(target);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).toHaveBeenCalled();
  });

  it("runs a command instead of sending it", async () => {
    const target = model({ draft: "/mode execute" });
    const { user } = renderComposer(target);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).not.toHaveBeenCalled();
    expect(target.chatConfig.setSessionExecutionMode).toHaveBeenCalledWith("execute");
    expect(target.setDraft).toHaveBeenCalledWith("");
  });

  it("does not intercept in a non-OnePiece session", async () => {
    const target = model({
      draft: "/mode execute",
      activeSession: { id: "s", title: "S", agentId: "claude-code", interactionMode: "cli", lifecycleState: "idle", ...sendable },
    });
    const { user } = renderComposer(target);
    await user.click(screen.getByRole("button", { name: /Send/i }));
    expect(target.submit).toHaveBeenCalled();
  });
});
