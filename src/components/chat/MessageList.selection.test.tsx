// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
import { workbenchSelectionKey } from "../../types/workbench-selection";
import { MessageList } from "./MessageList";

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

describe("MessageList selection", () => {
  const messages = [
    message({ id: "m1" }),
    message({ id: "m2", toolUse: [{ id: "t1", name: "shell", input: { command: "ls" }, status: "completed" }] }),
  ];

  it("resolves currentSelectionKey to the matching message or tool call, and to neither by default", () => {
    const { rerender } = renderWithAppProviders(
      <MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    const bubbles = () => screen.getAllByTestId("message-bubble");
    expect(bubbles()[0].getAttribute("aria-current")).toBeNull();
    expect(bubbles()[1].getAttribute("aria-current")).toBeNull();
    expect(document.querySelector('[data-tool-call-id="t1"]')?.getAttribute("aria-current")).toBeNull();

    rerender(
      <MessageList
        currentSelectionKey={workbenchSelectionKey({ kind: "message", sessionId: "s1", messageId: "m1" })}
        hasActiveSession
        hasMore={false}
        messages={messages}
        onLoadEarlier={vi.fn()}
      />,
    );
    expect(bubbles()[0].getAttribute("aria-current")).toBe("true");
    expect(bubbles()[1].getAttribute("aria-current")).toBeNull();

    rerender(
      <MessageList
        currentSelectionKey={workbenchSelectionKey({ kind: "tool", sessionId: "s1", messageId: "m2", toolCallId: "t1" })}
        hasActiveSession
        hasMore={false}
        messages={messages}
        onLoadEarlier={vi.fn()}
      />,
    );
    // The tool call is selected, but the message that contains it is not.
    expect(bubbles()[0].getAttribute("aria-current")).toBeNull();
    expect(bubbles()[1].getAttribute("aria-current")).toBeNull();
    expect(document.querySelector('[data-tool-call-id="t1"]')?.getAttribute("aria-current")).toBe("true");
  });

  it("routes a message click back through onSelectMessage with that message's own id", () => {
    const onSelectMessage = vi.fn();
    renderWithAppProviders(
      <MessageList
        hasActiveSession
        hasMore={false}
        messages={messages}
        onLoadEarlier={vi.fn()}
        onSelectMessage={onSelectMessage}
      />,
    );
    fireEvent.click(screen.getAllByTestId("message-bubble")[0]);
    expect(onSelectMessage).toHaveBeenCalledWith("m1");
  });
});
