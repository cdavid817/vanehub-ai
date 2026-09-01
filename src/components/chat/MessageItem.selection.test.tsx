// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
import { MessageItem } from "./MessageItem";

function message(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: "m1",
    sessionId: "s1",
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

describe("MessageItem selection", () => {
  it("selects the message when its bubble is clicked", () => {
    const onSelect = vi.fn();
    renderWithAppProviders(<MessageItem message={message()} onSelect={onSelect} />);
    fireEvent.click(screen.getByTestId("message-bubble"));
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("does not select the message when the click lands on a nested interactive control", () => {
    const onSelect = vi.fn();
    renderWithAppProviders(<MessageItem message={message()} onSelect={onSelect} />);
    // MessageFeedbackControls renders unconditionally for a completed assistant message.
    fireEvent.click(screen.getByRole("button", { name: "Helpful" }));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("does not select the message when the click lands inside the ToolUseBlock subtree", () => {
    const onSelect = vi.fn();
    renderWithAppProviders(
      <MessageItem
        message={message({ toolUse: [{ id: "t1", name: "shell", input: { command: "ls" }, status: "completed" }] })}
        onSelect={onSelect}
      />,
    );
    fireEvent.click(screen.getByTestId("tool-activity"));
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("renders a non-color-only marker only while selected", () => {
    const { rerender } = renderWithAppProviders(
      <MessageItem message={message()} onSelect={vi.fn()} selected={false} />,
    );
    expect(screen.queryByTestId("message-selected-indicator")).toBeNull();
    expect(screen.getByTestId("message-bubble").getAttribute("aria-current")).toBeNull();

    rerender(<MessageItem message={message()} onSelect={vi.fn()} selected />);
    expect(screen.getByTestId("message-selected-indicator")).toBeTruthy();
    expect(screen.getByTestId("message-bubble").getAttribute("aria-current")).toBe("true");
  });

  it("activates selection on Enter and Space when the bubble itself is focused", () => {
    const onSelect = vi.fn();
    renderWithAppProviders(<MessageItem message={message()} onSelect={onSelect} />);
    const bubble = screen.getByTestId("message-bubble");
    fireEvent.keyDown(bubble, { key: "Enter" });
    expect(onSelect).toHaveBeenCalledTimes(1);
    fireEvent.keyDown(bubble, { key: " " });
    expect(onSelect).toHaveBeenCalledTimes(2);
  });

  it("does not activate selection for an unrelated key, or for a keydown bubbled from a focused child", () => {
    const onSelect = vi.fn();
    renderWithAppProviders(<MessageItem message={message()} onSelect={onSelect} />);
    const bubble = screen.getByTestId("message-bubble");
    fireEvent.keyDown(bubble, { key: "Tab" });
    fireEvent.keyDown(screen.getByRole("button", { name: "Helpful" }), { key: "Enter" });
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("keeps the bubble a plain, non-interactive div when no selection handler is wired", () => {
    renderWithAppProviders(<MessageItem message={message()} />);
    const bubble = screen.getByTestId("message-bubble");
    expect(bubble.getAttribute("role")).toBeNull();
    expect(bubble.getAttribute("tabindex")).toBeNull();
    // Must not throw when clicked with no handler wired.
    fireEvent.click(bubble);
  });
});
