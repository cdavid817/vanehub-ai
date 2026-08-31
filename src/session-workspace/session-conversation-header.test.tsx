// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { SessionConversationHeader } from "./session-conversation-header";
import type { Session } from "../types/agent";

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    title: "CLI work",
    agentId: "claude-code",
    interactionMode: "cli",
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

describe("SessionConversationHeader's primary action", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("shows no stop button while nothing is streaming, even with a handler available", () => {
    render(<SessionConversationHeader isStreaming={false} onStop={vi.fn()} session={session()} />);
    expect(screen.queryByRole("button", { name: "Stop generation" })).toBeNull();
  });

  it("shows no stop button while streaming if the caller has nothing to call", () => {
    render(<SessionConversationHeader isStreaming session={session()} />);
    expect(screen.queryByRole("button", { name: "Stop generation" })).toBeNull();
  });

  it("shows the stop button as the header's one primary action while streaming", () => {
    render(<SessionConversationHeader isStreaming onStop={vi.fn()} session={session()} />);
    expect(screen.getByRole("button", { name: "Stop generation" })).toBeTruthy();
  });

  it("calls onStop when the header's stop button is clicked", () => {
    const onStop = vi.fn();
    render(<SessionConversationHeader isStreaming onStop={onStop} session={session()} />);
    fireEvent.click(screen.getByRole("button", { name: "Stop generation" }));
    expect(onStop).toHaveBeenCalledTimes(1);
  });

  it("never shows the stop button for a session that is not currently open", () => {
    render(<SessionConversationHeader isStreaming={false} onStop={vi.fn()} session={null} />);
    expect(screen.queryByRole("button", { name: "Stop generation" })).toBeNull();
  });
});
