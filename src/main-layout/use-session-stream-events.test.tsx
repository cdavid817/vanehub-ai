// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";

const service = vi.hoisted(() => ({ subscribeMessageEvents: vi.fn() }));
vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));

import { useSessionStreamEvents } from "./use-session-stream-events";

const messagesKey = ["messages", "session-1", 50] as const;
const seatMessage: ChatMessage = {
  id: "seat-message-1", sessionId: "session-1", role: "assistant", speakerSeatId: "seat-2",
  content: "", status: "streaming", createdAt: "2026-08-27T00:00:00Z",
  updatedAt: "2026-08-27T00:00:00Z", sessionSequence: 2, executionRunId: "run-1",
};

describe("useSessionStreamEvents", () => {
  beforeEach(() => service.subscribeMessageEvents.mockReset());

  it("reconciles a new seat message and replays its first token burst", async () => {
    let emit: ((event: ChatStreamEvent) => void) | null = null;
    service.subscribeMessageEvents.mockImplementation(async (
      _sessionId: string,
      listener: (event: ChatStreamEvent) => void,
    ) => {
      emit = listener;
      return vi.fn();
    });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    client.setQueryData(messagesKey, []);
    vi.spyOn(client, "invalidateQueries").mockImplementation(async () => {
      client.setQueryData(messagesKey, [seatMessage]);
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    renderHook(() => useSessionStreamEvents({
      invalidateSessions: vi.fn(), messagesKey, onTurnStatus: vi.fn(), sessionId: "session-1",
    }), { wrapper });
    await waitFor(() => expect(emit).not.toBeNull());
    act(() => {
      emit?.({ type: "started", sessionId: "session-1", messageId: "seat-message-1" });
      emit?.({ type: "token", sessionId: "session-1", messageId: "seat-message-1", contentDelta: "live" });
    });

    await waitFor(() => expect(client.getQueryData<ChatMessage[]>(messagesKey)?.[0]).toMatchObject({
      content: "live", speakerSeatId: "seat-2", status: "streaming",
    }));
  });

  // Task 21.8 streaming update-batch budget. The hook's own doc comment: "Token events arrive in
  // the thousands per turn; buffer them and flush on an animation frame so the message array is
  // rebuilt once per frame instead of once per token" -- genuinely untested before this: the one
  // pre-existing test above only ever emits 2 events total and never inspects how many times the
  // query cache itself was actually rebuilt relative to how many events were emitted.
  it("batches a rapid burst of token events into exactly one query-cache rebuild, not one per event", () => {
    let emit: ((event: ChatStreamEvent) => void) | null = null;
    service.subscribeMessageEvents.mockImplementation(async (
      _sessionId: string,
      listener: (event: ChatStreamEvent) => void,
    ) => {
      emit = listener;
      return vi.fn();
    });
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    // Seeded already-known, unlike the test above: this test is only about batching *known*-message
    // token events, not the separate unknown-message reconciliation path that test covers.
    client.setQueryData(messagesKey, [seatMessage]);
    const setQueryDataSpy = vi.spyOn(client, "setQueryData");
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={client}>{children}</QueryClientProvider>
    );

    // Controls exactly when the buffered batch flushes, instead of racing a real animation frame.
    let rafCallback: FrameRequestCallback | null = null;
    vi.stubGlobal("requestAnimationFrame", (callback: FrameRequestCallback) => {
      rafCallback = callback;
      return 1;
    });
    vi.stubGlobal("cancelAnimationFrame", () => { rafCallback = null; });

    renderHook(() => useSessionStreamEvents({
      invalidateSessions: vi.fn(), messagesKey, onTurnStatus: vi.fn(), sessionId: "session-1",
    }), { wrapper });
    expect(emit).not.toBeNull();
    setQueryDataSpy.mockClear(); // isolate to activity after the subscription itself settles

    // 200 stands in for the "thousands per turn" the hook's own doc comment names -- a
    // representative rapid burst, all emitted before any animation frame fires.
    act(() => {
      for (let index = 0; index < 200; index += 1) {
        emit?.({ type: "token", sessionId: "session-1", messageId: "seat-message-1", contentDelta: "x" });
      }
    });
    expect(setQueryDataSpy).not.toHaveBeenCalled(); // still buffered -- nothing rebuilt yet

    act(() => { rafCallback?.(0); });

    expect(setQueryDataSpy).toHaveBeenCalledTimes(1); // all 200 events landed in exactly one rebuild
    // Every one of the 200 deltas is present, in order -- batching did not silently drop any.
    expect(client.getQueryData<ChatMessage[]>(messagesKey)?.[0].content).toBe("x".repeat(200));

    vi.unstubAllGlobals();
  });
});
