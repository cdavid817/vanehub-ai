// @vitest-environment jsdom

import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { ChatMessage } from "../../types/chat";
import { cn } from "../../lib/utils";
import { MessageItem } from "./MessageItem";

// `cn(...)` runs multiple times per actual render of `MessageItem` (once per className it
// computes) and never when React bails via `memo()` without re-executing the function body --
// unlike `Profiler.onRender`, which fires on every commit at its boundary regardless of whether
// the memoized child inside it actually re-rendered (confirmed empirically: a first draft of this
// test used Profiler and got a false "rendered" signal on a pure memo bail). Counting `cn` calls
// is a direct, reliable proxy for "did MessageItem's render function actually execute."
vi.mock("../../lib/utils", async (importOriginal) => {
  const actual = await importOriginal<typeof import("../../lib/utils")>();
  return { ...actual, cn: vi.fn(actual.cn) };
});

function message(overrides: Partial<ChatMessage> = {}): ChatMessage {
  return {
    id: "m1",
    sessionId: "s1",
    role: "assistant",
    content: "Hello",
    status: "completed",
    createdAt: "2026-08-06T10:00:00Z",
    updatedAt: "2026-08-06T10:00:00Z",
    sessionSequence: 1,
    executionRunId: null,
    ...overrides,
  };
}

const cnSpy = vi.mocked(cn);

beforeAll(async () => {
  await activateAppLanguage("en");
});

// Task 10.13: MessageItem is memoized so a streaming sibling does not rerender unrelated history.
// That guarantee is only real if every prop MessageList hands it is referentially stable when the
// underlying message hasn't changed -- these tests exercise the actual memo bail-out, not just
// that `memo()` appears in the source.
describe("MessageItem memoization", () => {
  it("does not re-execute when rerendered with referentially identical props (the memo actually bails)", () => {
    cnSpy.mockClear();
    // Captured once, outside both render calls: this is what MessageList now does after task
    // 10.13's fix -- pass the same onSelectMessage/onSelectTool reference to every row instead of
    // minting a new closure per row per render.
    const props = { message: message(), onSelect: () => undefined, selected: false };
    const { rerender } = renderWithAppProviders(<MessageItem {...props} />);
    const callsAfterFirstRender = cnSpy.mock.calls.length;
    expect(callsAfterFirstRender).toBeGreaterThan(0);

    rerender(<MessageItem {...props} />);
    expect(cnSpy.mock.calls.length).toBe(callsAfterFirstRender);
  });

  it("does re-execute when the message's own content changes (the memo is not over-eager)", () => {
    cnSpy.mockClear();
    const { rerender } = renderWithAppProviders(<MessageItem message={message({ content: "Hello" })} />);
    const callsAfterFirstRender = cnSpy.mock.calls.length;

    rerender(<MessageItem message={message({ content: "Hello, updated" })} />);
    expect(cnSpy.mock.calls.length).toBeGreaterThan(callsAfterFirstRender);
  });

  it("would have re-executed every render before the fix -- a fresh per-render closure defeats the same memo", () => {
    cnSpy.mockClear();
    const stableMessage = message();
    const { rerender } = renderWithAppProviders(<MessageItem message={stableMessage} onSelect={() => undefined} />);
    const callsAfterFirstRender = cnSpy.mock.calls.length;

    // A fresh arrow function every render, exactly what `MessageList` passed before task 10.13's
    // fix -- reproduces the bug this task fixed, as a regression guard on the fix itself rather
    // than only on the new code path.
    rerender(<MessageItem message={stableMessage} onSelect={() => undefined} />);
    expect(cnSpy.mock.calls.length).toBeGreaterThan(callsAfterFirstRender);
  });
});
