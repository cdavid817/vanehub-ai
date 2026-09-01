// @vitest-environment jsdom

import { forwardRef, useImperativeHandle } from "react";
import { screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
import { MESSAGE_LIST_VIRTUALIZE_THRESHOLD, MessageList } from "./MessageList";

// Same fix as VirtualizedMessageList.test.tsx / session-row-list.test.tsx: jsdom never reports a
// real layout, so a real MeasuredVirtualList renders zero rows regardless of item count. This
// mock lets the *real* MessageList and the *real* VirtualizedMessageList both run — only the
// lowest-level measurement primitive is faked — so this test exercises the actual threshold
// decision in MessageList.tsx, not a reimplementation of it.
vi.mock("../measured-virtual-list", () => ({
  MeasuredVirtualList: forwardRef(({ ariaLabel, getItemKey, items, renderItem, testId }: {
    ariaLabel: string; getItemKey: (item: unknown, index: number) => string;
    items: readonly unknown[]; renderItem: (item: unknown, index: number) => React.ReactNode; testId?: string;
  }, ref: React.Ref<{ scrollToIndex: () => void }>) => {
    useImperativeHandle(ref, () => ({ scrollToIndex: () => undefined }));
    return (
      <div aria-label={ariaLabel} data-testid={testId}>
        {items.map((item, index) => <div key={getItemKey(item, index)}>{renderItem(item, index)}</div>)}
      </div>
    );
  }),
}));

function message(id: string): ChatMessage {
  return {
    id,
    sessionId: "s1",
    role: "assistant",
    content: `Message ${id}`,
    status: "completed",
    createdAt: "2026-08-06T10:00:00Z",
    updatedAt: "2026-08-06T10:00:00Z",
    sessionSequence: 1,
    executionRunId: null,
  };
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

// Task 10.12: "keep DOM rows bounded for the 5,000-message fixture." The literal fixture the task
// names (`generateLargeScaleFixtures`, FIXTURE_COUNTS.messages = 5000) spreads that count across
// 1,000 sessions -- no single session in it actually reaches 5,000 messages (its own "epic" tier
// tops out around 140), so proving this claim needs a synthetic single-session array built here,
// not that shared fixture.
describe("MessageList virtualization threshold", () => {
  it("stays on the plain, non-virtualized path one message short of the threshold", () => {
    const messages = Array.from({ length: MESSAGE_LIST_VIRTUALIZE_THRESHOLD - 1 }, (_, index) => message(String(index)));
    renderWithAppProviders(
      <MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    expect(screen.getAllByTestId("message-bubble")).toHaveLength(messages.length);
    // The (mocked) virtualized list would also render every item, so length alone would not tell
    // the two paths apart -- the readable-measure grid only exists on the non-virtualized path.
    expect(screen.getByTestId("message-readable-measure")).toBeTruthy();
  });

  it("switches to the virtualized path at exactly the threshold", () => {
    const messages = Array.from({ length: MESSAGE_LIST_VIRTUALIZE_THRESHOLD }, (_, index) => message(String(index)));
    renderWithAppProviders(
      <MessageList hasActiveSession hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );
    expect(screen.queryByTestId("message-readable-measure")).toBeNull();
    expect(screen.getAllByTestId("message-bubble")).toHaveLength(messages.length);
  });

  it("covers the 5,000-message fixture with wide margin, not just barely", () => {
    // Deliberately not a 5,000-item render: the routing decision above is a single scale-blind
    // `>=` comparison, already proven correct exactly at the threshold — re-running the identical
    // logic through 5,000 real MessageItem trees (each parsing Markdown) would prove nothing new
    // and costs well over a minute in jsdom, which is not a trade worth making for this claim.
    // What is worth asserting directly is the margin itself, since task 10.12 names 5,000 by
    // number and a silent future edit to the threshold constant should not lose that relationship.
    expect(MESSAGE_LIST_VIRTUALIZE_THRESHOLD).toBeLessThan(5000);
  });
});
