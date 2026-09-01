// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { workbenchSelectionKey } from "../types/workbench-selection";
import { ChatTab } from "./chat-tab";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "CLI work",
    agentId: "claude-code",
    interactionMode: "api",
    personalizationMode: "standard",
    lifecycleState: "running",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\code\\vanehub-ai",
    projectPath: "D:\\code\\vanehub-ai",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-27T00:00:00.000Z",
    updatedAt: "2026-08-27T00:00:00.000Z",
    ...overrides,
  };
}

function message(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: "m1",
    sessionId: "session-1",
    role: "assistant",
    content: "Done.",
    status: "completed",
    createdAt: "2026-08-06T10:00:00Z",
    updatedAt: "2026-08-06T10:00:00Z",
    sessionSequence: 1,
    executionRunId: null,
    ...overrides,
  };
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

// End-to-end check that ChatTab's new selection props reach MessageItem/ToolUseBlock through the
// real MessageList, not a mock -- the coordinator wires these three prop names directly from
// main-layout.tsx, so a break anywhere in that chain belongs here, not in a unit test of one link.
describe("ChatTab selection wiring", () => {
  it("marks the message matching currentSelectionKey as selected end to end", () => {
    renderWithAppProviders(
      <ChatTab
        activeSession={session()}
        composer={null}
        currentSelectionKey={workbenchSelectionKey({ kind: "message", sessionId: "session-1", messageId: "m1" })}
        messages={[message()]}
        onLoadEarlier={vi.fn()}
      />,
    );
    expect(screen.getByTestId("message-bubble").getAttribute("aria-current")).toBe("true");
  });

  it("reports a clicked message's id back through onSelectMessage", () => {
    const onSelectMessage = vi.fn();
    renderWithAppProviders(
      <ChatTab
        activeSession={session()}
        composer={null}
        messages={[message({ id: "m7" })]}
        onLoadEarlier={vi.fn()}
        onSelectMessage={onSelectMessage}
      />,
    );
    fireEvent.click(screen.getByTestId("message-bubble"));
    expect(onSelectMessage).toHaveBeenCalledWith("m7");
  });

  it("reports a clicked tool call back through onSelectTool with both the message and tool call id", () => {
    const onSelectTool = vi.fn();
    renderWithAppProviders(
      <ChatTab
        activeSession={session()}
        composer={null}
        messages={[
          message({
            id: "m7",
            toolUse: [{ id: "t9", name: "shell", input: { command: "npm test" }, status: "running" }],
          }),
        ]}
        onLoadEarlier={vi.fn()}
        onSelectTool={onSelectTool}
      />,
    );
    const row = document.querySelector('[data-tool-call-id="t9"]');
    fireEvent.click(row!.querySelector("summary")!);
    expect(onSelectTool).toHaveBeenCalledWith("m7", "t9");
  });
});
