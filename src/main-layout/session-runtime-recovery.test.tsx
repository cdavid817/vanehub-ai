// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { SessionContextPanel } from "./session-context-panel";
import { SessionRuntimeFailureNotice } from "../session-workspace/session-runtime-failure-notice";

const session = (overrides: Partial<Session> = {}): Session => ({
  id: "session-1",
  title: "发布会话",
  agentId: "claude",
  interactionMode: "cli",
  lifecycleState: "failed",
  archived: false,
  pinned: false,
  categoryId: null,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  ...overrides,
} as Session);

const failedMessage = (error: string): ChatMessage => ({
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "",
  status: "failed",
  error,
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  sessionSequence: 1,
  executionRunId: null,
});

describe("SessionRuntimeFailureNotice", () => {
  it("offers recovery and keeps the reported failure reason on screen", () => {
    const onRecover = vi.fn();
    render(
      <SessionRuntimeFailureNotice
        messages={[failedMessage("claude command failed: exit code 1")]}
        onRecover={onRecover}
        recovering={false}
        session={session()}
      />,
    );

    // The reason stays beside the button because recovery restores a usable state without
    // fixing whatever failed.
    expect(screen.getByText("claude command failed: exit code 1")).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: /恢复会话/ }));
    expect(onRecover).toHaveBeenCalledTimes(1);
  });

  it("stays out of the way unless the runtime actually failed", () => {
    const { container, rerender } = render(
      <SessionRuntimeFailureNotice messages={[]} onRecover={vi.fn()} recovering={false} session={session({ lifecycleState: "idle" })} />,
    );
    expect(container.firstChild).toBeNull();

    rerender(
      <SessionRuntimeFailureNotice messages={[]} onRecover={vi.fn()} recovering={false} session={session({ archived: true })} />,
    );
    expect(container.firstChild).toBeNull();
  });

  it("blocks a second request while one is already running", () => {
    render(<SessionRuntimeFailureNotice messages={[]} onRecover={vi.fn()} recovering session={session()} />);
    expect(screen.getByRole("button", { name: /正在恢复/ }).hasAttribute("disabled")).toBe(true);
  });
});

describe("SessionContextPanel recovery entry", () => {
  const panelProps = {
    categories: [],
    onArchive: vi.fn(),
    onAssignCategory: vi.fn(),
    onChange: vi.fn(),
    onCreateCategory: vi.fn(),
    onDelete: vi.fn(),
    onDismiss: vi.fn(),
    onExport: vi.fn(),
    onPin: vi.fn(),
    onRename: vi.fn(),
  };

  it("recovers the menu's own session rather than the active one", () => {
    const onRecover = vi.fn();
    const target = session({ id: "session-2", title: "另一个会话" });
    render(
      <SessionContextPanel
        {...panelProps}
        onRecover={onRecover}
        value={{ session: target, mode: "menu", draftTitle: target.title }}
      />,
    );

    // `role="menuitem"` (not "button") since this recover entry lives inside the panel's
    // `role="menu"` -- an explicit role overrides a `<button>`'s implicit one.
    fireEvent.click(screen.getByRole("menuitem", { name: /恢复会话/ }));
    expect(onRecover).toHaveBeenCalledWith(target);
  });

  it("does not offer recovery for an archived session", () => {
    const archived = session({ archived: true });
    render(
      <SessionContextPanel
        {...panelProps}
        onRecover={vi.fn()}
        value={{ session: archived, mode: "menu", draftTitle: archived.title }}
      />,
    );

    expect(screen.queryByRole("menuitem", { name: /恢复会话/ })).toBeNull();
  });
});
