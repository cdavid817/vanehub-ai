// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook } from "@testing-library/react";
import type { ReactNode } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";

const service = vi.hoisted(() => ({ subscribeMessageEvents: vi.fn() }));
vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));

import { useSessionStreamEvents } from "./use-session-stream-events";

const messagesKey = ["messages", "session-1", 50] as const;
const streaming: ChatMessage = {
  content: "", createdAt: "2026-08-27T00:00:00Z", executionRunId: "run-1", id: "message-1",
  role: "assistant", sessionId: "session-1", sessionSequence: 1, status: "streaming",
  updatedAt: "2026-08-27T00:00:00Z",
};

/** Mounts the hook and returns the emitter plus a spy counting message-list rebuilds. */
async function mounted() {
  let emit: ((event: ChatStreamEvent) => void) | null = null;
  service.subscribeMessageEvents.mockImplementation(async (
    _sessionId: string,
    listener: (event: ChatStreamEvent) => void,
  ) => {
    emit = listener;
    return vi.fn();
  });
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  client.setQueryData(messagesKey, [streaming]);
  const wrapper = ({ children }: { children: ReactNode }) => (
    <QueryClientProvider client={client}>{children}</QueryClientProvider>
  );
  renderHook(() => useSessionStreamEvents({
    invalidateSessions: vi.fn(), messagesKey, onTurnStatus: vi.fn(), sessionId: "session-1",
  }), { wrapper });
  await act(async () => { await Promise.resolve(); });
  const rebuilds = vi.spyOn(client, "setQueryData");
  return { client, emit: emit as unknown as (event: ChatStreamEvent) => void, rebuilds };
}

function token(contentDelta: string): ChatStreamEvent {
  return { contentDelta, messageId: "message-1", sessionId: "session-1", type: "token" };
}

describe("useSessionStreamEvents pacing", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    service.subscribeMessageEvents.mockReset();
  });

  afterEach(() => vi.useRealTimers());

  it("rebuilds the message list once for a burst inside one render interval", async () => {
    const { emit, rebuilds } = await mounted();

    await act(async () => {
      for (let index = 0; index < 40; index += 1) emit(token(`${index} `));
      await vi.advanceTimersByTimeAsync(150);
    });

    expect(rebuilds).toHaveBeenCalledTimes(1);
  });

  it("paces a long stream far below one rebuild per animation frame", async () => {
    // A frame-paced stream would rebuild roughly 60 times a second, and every rebuild re-parses
    // the whole accumulated Markdown of the streaming row.
    const { emit, rebuilds } = await mounted();

    await act(async () => {
      for (let step = 0; step < 60; step += 1) {
        emit(token(`${step} `));
        await vi.advanceTimersByTimeAsync(16);
      }
    });

    expect(rebuilds.mock.calls.length).toBeLessThanOrEqual(12);
  });

  it("drops no delta while pacing", async () => {
    // Coalescing is only acceptable if it batches deltas rather than discarding them.
    const { client, emit } = await mounted();

    await act(async () => {
      for (let index = 0; index < 5; index += 1) {
        emit(token(`${index}`));
        await vi.advanceTimersByTimeAsync(16);
      }
      await vi.advanceTimersByTimeAsync(300);
    });

    expect(client.getQueryData<ChatMessage[]>(messagesKey)?.[0]?.content).toBe("01234");
  });

  it("flushes a terminal event without waiting for the interval", async () => {
    const { emit, rebuilds } = await mounted();

    await act(async () => {
      emit(token("last "));
      emit({ messageId: "message-1", sessionId: "session-1", type: "completed" });
    });

    expect(rebuilds).toHaveBeenCalledTimes(1);
  });
});
