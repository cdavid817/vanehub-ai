// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
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

// Task 10.4's continuous transcript hierarchy: a run of messages from the same speaker collapses
// its repeated avatar/name/timestamp header, but never at the cost of task 10.5's failure/status
// prominence.
describe("MessageList consecutive-run grouping", () => {
  it("collapses the header only for a second completed message from the same assistant seat", () => {
    const messages = [
      message({ id: "m1", speakerSeatId: "seat-a" }),
      message({ id: "m2", speakerSeatId: "seat-a" }),
    ];
    renderWithAppProviders(<MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={() => undefined} />);
    const rows = screen.getAllByTestId("message-bubble").map((bubble) => bubble.closest("article"));
    expect(rows[0]?.getAttribute("data-message-header")).toBe("shown");
    expect(rows[1]?.getAttribute("data-message-header")).toBe("collapsed");
  });

  it("does not collapse the header when the speaker seat changes between messages", () => {
    const messages = [
      message({ id: "m1", speakerSeatId: "seat-a" }),
      message({ id: "m2", speakerSeatId: "seat-b" }),
    ];
    renderWithAppProviders(<MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={() => undefined} />);
    const rows = screen.getAllByTestId("message-bubble").map((bubble) => bubble.closest("article"));
    expect(rows[0]?.getAttribute("data-message-header")).toBe("shown");
    expect(rows[1]?.getAttribute("data-message-header")).toBe("shown");
  });

  it("does not collapse the header across a role change (user then assistant)", () => {
    const messages = [
      message({ id: "m1", role: "user", speakerSeatId: undefined }),
      message({ id: "m2", role: "assistant", speakerSeatId: "seat-a" }),
    ];
    renderWithAppProviders(<MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={() => undefined} />);
    const rows = screen.getAllByTestId("message-bubble").map((bubble) => bubble.closest("article"));
    expect(rows[0]?.getAttribute("data-message-header")).toBe("shown");
    expect(rows[1]?.getAttribute("data-message-header")).toBe("shown");
  });

  it("forces its own header for a non-completed message even in a same-speaker run (task 10.5)", () => {
    const messages = [
      message({ id: "m1", speakerSeatId: "seat-a" }),
      message({ id: "m2", speakerSeatId: "seat-a", status: "failed" }),
    ];
    renderWithAppProviders(<MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={() => undefined} />);
    const rows = screen.getAllByTestId("message-bubble").map((bubble) => bubble.closest("article"));
    expect(rows[1]?.getAttribute("data-message-header")).toBe("shown");
  });
});
