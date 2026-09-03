// @vitest-environment jsdom

import { cleanup } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { cn } from "../../lib/utils";
import type { ChatMessage } from "../../types/chat";
import { VirtualizedMessageList } from "./VirtualizedMessageList";

/**
 * 21.8: `VirtualizedMessageList.test.tsx` already proves wiring (message-id keying, load-more,
 * selection, scroll-to-last) against a `MeasuredVirtualList` mock that renders every item -- by
 * design, to prove wiring rather than DOM bounds ("the real one would window them," that file's own
 * comment; `MessageList.virtualization.test.tsx`'s own third test explicitly declines a real
 * 5,000-item render for the identical reason). This file instead mocks one level lower --
 * `@tanstack/react-virtual`'s own `useVirtualizer` -- so the real `VirtualizedMessageList` -> real
 * `MeasuredVirtualList` path runs and performs its actual `virtualItems.map(...)` windowing and
 * `anchorTo` wiring, against a fake but bounded/inspectable measurement result standing in for
 * jsdom's real but useless zero-clientHeight one (same standing limitation `session-sidebar.large-
 * scale.test.tsx` and `work-board-item-list.large-scale.test.tsx` both already document for the
 * identical primitive).
 */
const FAKE_VIRTUAL_WINDOW = 20; // Stands in for "visible messages + overscan" in a real viewport --
// comfortably below this file's 5,000-message fixture and the real overscan=8 the component
// requests, so a bounded DOM count here can only come from real windowing logic actually running.

interface FakeVirtualizerOptions {
  anchorTo?: "start" | "end";
  count: number;
  getItemKey: (index: number) => string;
}

const mocks = vi.hoisted(() => ({
  capturedOptions: [] as FakeVirtualizerOptions[],
}));

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (options: FakeVirtualizerOptions) => {
    mocks.capturedOptions.push(options);
    const windowSize = Math.min(FAKE_VIRTUAL_WINDOW, options.count);
    const virtualItems = Array.from({ length: windowSize }, (_unused, index) => ({
      key: options.getItemKey(index),
      index,
      start: index * 96,
    }));
    return {
      getVirtualItems: () => virtualItems,
      getTotalSize: () => options.count * 96,
      measure: () => undefined,
      measureElement: () => undefined,
      scrollToIndex: () => undefined,
      scrollToOffset: () => undefined,
    };
  },
}));

// Counts MessageItem's own real render executions, the same established technique
// MessageItem.memoization.test.tsx already uses ("counting `cn` calls is a direct, reliable proxy
// for 'did MessageItem's render function actually execute'" -- that file's own comment). Reused
// here rather than reinvented, at the full-list integration level that file deliberately stays
// below (it renders `MessageItem` directly, never through a real list).
vi.mock("../../lib/utils", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/utils")>();
  return { ...actual, cn: vi.fn(actual.cn) };
});
const cnSpy = vi.mocked(cn);

function message(id: string, content = `Message ${id}`): ChatMessage {
  return {
    id,
    sessionId: "s1",
    role: "assistant",
    content,
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

describe("VirtualizedMessageList at 5,000-message scale (21.8)", () => {
  it("keeps rendered message rows bounded regardless of the underlying message count", () => {
    const messages = Array.from({ length: 5000 }, (_unused, index) => message(String(index)));

    const { container } = renderWithAppProviders(
      <VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />,
    );

    const rendered = container.querySelectorAll("[data-testid='message-bubble']");
    expect(rendered.length).toBe(FAKE_VIRTUAL_WINDOW);
    expect(rendered.length).toBeLessThan(messages.length);
  });

  it("opts into @tanstack/react-virtual's own edge-anchored repositioning so prepending older messages does not jump the viewport", () => {
    mocks.capturedOptions.length = 0;
    const messages = Array.from({ length: 5000 }, (_unused, index) => message(String(index)));

    renderWithAppProviders(<VirtualizedMessageList hasMore messages={messages} onLoadEarlier={vi.fn()} />);

    // The library's own anchor-preserving math (verified by reading @tanstack/virtual-core's
    // source directly: `setOptions` recomputes `scrollOffset` from the edge item's own key whenever
    // `anchorTo === "end"` and the edge keys change, e.g. from a prepend) is that library's own,
    // independently maintained responsibility -- not re-verified pixel-by-pixel here, since jsdom's
    // own lack of real layout makes that unreachable from a unit test regardless. What this codebase
    // owns, and what was genuinely missing before this task, is opting into it at all: before this
    // change `measured-virtual-list.tsx` never passed `anchorTo`, so it silently defaulted to the
    // library's own `"start"` (off) and a "load earlier" prepend while mid-history would jump.
    expect(mocks.capturedOptions.at(-1)?.anchorTo).toBe("end");
  });

  it("rerender cost is additive per changed message, not the 5,000-message total", () => {
    const messages = Array.from({ length: 5000 }, (_unused, index) => message(String(index)));
    // Both indices sit inside the fake virtualizer's own rendered window (< FAKE_VIRTUAL_WINDOW).
    const changeContentAt = (source: ChatMessage[], index: number, suffix: string) =>
      source.map((candidate, candidateIndex) => (candidateIndex === index ? message(candidate.id, `${candidate.content}-${suffix}`) : candidate));

    // Three independent fresh-mount experiments (not chained rerenders off one mount, which would
    // each only measure a delta from the *immediately preceding* render): proves additivity --
    // changing two rows together costs exactly the sum of changing each alone, never more --
    // without assuming every row's own render costs the same number of `cn()` calls (it does not;
    // an earlier draft of this test asserted a flat 2x multiple and found that assumption false).
    function costOfChanging(...indices: number[]): number {
      const { rerender } = renderWithAppProviders(<VirtualizedMessageList hasMore={false} messages={messages} onLoadEarlier={vi.fn()} />);
      cnSpy.mockClear();
      const next = indices.reduce((current, index) => changeContentAt(current, index, "streamed"), messages);
      rerender(<VirtualizedMessageList hasMore={false} messages={next} onLoadEarlier={vi.fn()} />);
      const cost = cnSpy.mock.calls.length;
      cleanup();
      return cost;
    }

    // `MeasuredVirtualList` itself is not memoized: every render re-creates its own per-virtual-item
    // wrapper `<div className={cn(...)}>` for all `FAKE_VIRTUAL_WINDOW` rows regardless of whether
    // the `MessageItem` nested inside each one actually re-executes -- a real, fixed cost of *that*
    // wrapper, not of any message row. Measured directly (a rerender with zero messages changed)
    // rather than assumed, then netted out of each figure below so what remains isolates
    // `MessageItem`'s own execution cost specifically.
    const fixedWrapperOverhead = costOfChanging();
    const costOfRow5 = costOfChanging(5) - fixedWrapperOverhead;
    const costOfRow10 = costOfChanging(10) - fixedWrapperOverhead;
    const costOfBoth = costOfChanging(5, 10) - fixedWrapperOverhead;
    expect(costOfRow5).toBeGreaterThan(0);
    expect(costOfRow10).toBeGreaterThan(0);

    // Proof the other ~4,998 messages (including the other ~18 rows inside the fake virtualizer's
    // own rendered window) never re-executed at all, for any of the three transitions, regardless
    // of the 5,000-message total behind them.
    expect(costOfBoth).toBe(costOfRow5 + costOfRow10);
  });
});
