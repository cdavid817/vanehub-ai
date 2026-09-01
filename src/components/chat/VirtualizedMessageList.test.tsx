// @vitest-environment jsdom

import { forwardRef, useImperativeHandle } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
import { workbenchSelectionKey } from "../../types/workbench-selection";
import { VirtualizedMessageList } from "./VirtualizedMessageList";

// `@tanstack/react-virtual` measures against a real layout (clientHeight etc.), which jsdom never
// provides — a real MeasuredVirtualList renders zero rows here regardless of `items.length`.
// Replaced with a component that honors the same items/getItemKey/renderItem/imperative-handle
// contract without the real measurement machinery — the exact fix this codebase already uses for
// `session-row-list.test.tsx`'s identical problem, applied here rather than inventing a second one.
const scrollToIndex = vi.fn();
vi.mock("../measured-virtual-list", () => ({
  MeasuredVirtualList: forwardRef(({ ariaLabel, getItemKey, items, onAtEndChange, renderItem, testId }: {
    ariaLabel: string; getItemKey: (item: unknown, index: number) => string;
    items: readonly unknown[]; onAtEndChange?: (atEnd: boolean) => void;
    renderItem: (item: unknown, index: number) => React.ReactNode; testId?: string;
  }, ref: React.Ref<{ scrollToIndex: typeof scrollToIndex }>) => {
    useImperativeHandle(ref, () => ({ scrollToIndex }));
    return (
      <div
        aria-label={ariaLabel}
        data-testid={testId}
        onScroll={onAtEndChange ? (event) => onAtEndChange((event.target as HTMLElement).scrollTop > 500) : undefined}
      >
        {items.map((item, index) => <div key={getItemKey(item, index)}>{renderItem(item, index)}</div>)}
      </div>
    );
  }),
}));

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

describe("VirtualizedMessageList wiring", () => {
  it("renders every message through the virtual list, keyed by message id", () => {
    const messages = [message({ id: "m1" }), message({ id: "m2" })];
    renderWithAppProviders(
      <VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    expect(screen.getAllByTestId("message-bubble")).toHaveLength(2);
  });

  it("adds a load-more row at index 0 when hasMore is true, without shifting message indices used for grouping", () => {
    const onLoadEarlier = vi.fn();
    const messages = [
      message({ id: "m1", speakerSeatId: "seat-a" }),
      message({ id: "m2", speakerSeatId: "seat-a" }),
    ];
    render(
      <VirtualizedMessageList hasMore messages={messages} onLoadEarlier={onLoadEarlier} />,
    );
    fireEvent.click(screen.getByText("Load earlier messages"));
    expect(onLoadEarlier).toHaveBeenCalledTimes(1);

    // m2 still groups under m1 (same seat, consecutive) even though the load-more row sits ahead
    // of both in the underlying virtual item list — proves messageIndex is corrected for the
    // pseudo-item offset, not just passed through as the raw virtual index.
    const rows = screen.getAllByTestId("message-bubble").map((bubble) => bubble.closest("article"));
    expect(rows[1]?.getAttribute("data-message-header")).toBe("collapsed");
  });

  it("marks the message matching currentSelectionKey as selected", () => {
    const messages = [message({ id: "m1" }), message({ id: "m2" })];
    renderWithAppProviders(
      <VirtualizedMessageList
        currentSelectionKey={workbenchSelectionKey({ kind: "message", sessionId: "s1", messageId: "m2" })}
        hasMore={false}
        messages={messages}
        onLoadEarlier={vi.fn()}
      />,
    );
    const bubbles = screen.getAllByTestId("message-bubble");
    expect(bubbles[0].getAttribute("aria-current")).toBeNull();
    expect(bubbles[1].getAttribute("aria-current")).toBe("true");
  });

  it("routes a message click back through onSelectMessage with that message's own id", () => {
    const onSelectMessage = vi.fn();
    const messages = [message({ id: "m7" })];
    renderWithAppProviders(
      <VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} onSelectMessage={onSelectMessage} />,
    );
    fireEvent.click(screen.getByTestId("message-bubble"));
    expect(onSelectMessage).toHaveBeenCalledWith("m7");
  });

  it("scrolls to the last message on mount and again when the messages array reference changes", () => {
    scrollToIndex.mockClear();
    const messages = [message({ id: "m1" })];
    const { rerender } = renderWithAppProviders(
      <VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    expect(scrollToIndex).toHaveBeenLastCalledWith(0, "end");

    const grown = [...messages, message({ id: "m2" })];
    rerender(<VirtualizedMessageList hasMore={false} messages={grown} onLoadEarlier={vi.fn()} />);
    expect(scrollToIndex).toHaveBeenLastCalledWith(1, "end");
  });

  it("stops auto-following once the fake reports the viewport left the bottom edge, and ScrollControl resumes it", async () => {
    scrollToIndex.mockClear();
    const messages = [message({ id: "m1" })];
    const { rerender, user } = renderWithAppProviders(
      <VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    const scrollRegion = screen.getByTestId("message-scroll-region");
    fireEvent.scroll(scrollRegion, { target: { scrollTop: 0 } });

    const grown = [...messages, message({ id: "m2" })];
    scrollToIndex.mockClear();
    rerender(<VirtualizedMessageList hasMore={false} messages={grown} onLoadEarlier={vi.fn()} />);
    // No longer following: a new message must not force the viewport back to the bottom.
    expect(scrollToIndex).not.toHaveBeenCalled();

    await user.click(screen.getByRole("button", { name: "Bottom" }));
    expect(scrollToIndex).toHaveBeenLastCalledWith(1, "end");
  });
});
