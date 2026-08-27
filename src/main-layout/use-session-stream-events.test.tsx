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
});
