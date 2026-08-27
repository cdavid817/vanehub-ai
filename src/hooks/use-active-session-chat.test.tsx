// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, waitFor } from "@testing-library/react";
import type { PropsWithChildren } from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";

const service = vi.hoisted(() => ({
  stopGeneration: vi.fn(),
  subscribeMessageEvents: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));

import { useSessionMessageEvents } from "./use-active-session-chat";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";

function Subscriber({ sessionId }: { sessionId: string }) {
  useSessionMessageEvents({ queryKey: ["messages", sessionId], sessionId });
  return null;
}

function wrapper({ children }: PropsWithChildren) {
  return (
    <QueryClientProvider client={new QueryClient()}>
      {children}
    </QueryClientProvider>
  );
}

describe("background session message subscription", () => {
  beforeEach(() => {
    service.stopGeneration.mockReset();
    service.subscribeMessageEvents.mockReset();
  });

  it("removes the renderer listener on route cleanup without cancelling the native run", async () => {
    const unsubscribe = vi.fn();
    service.subscribeMessageEvents.mockResolvedValue(unsubscribe);
    const view = render(<Subscriber sessionId="session-1" />, { wrapper });
    await waitFor(() => expect(service.subscribeMessageEvents).toHaveBeenCalledWith("session-1", expect.any(Function)));

    view.unmount();

    expect(unsubscribe).toHaveBeenCalledOnce();
    expect(service.stopGeneration).not.toHaveBeenCalled();
  });

  it("cleans a listener that resolves after navigation without cancelling the accepted run", async () => {
    const unsubscribe = vi.fn();
    let resolveSubscription: ((cleanup: () => void) => void) | undefined;
    service.subscribeMessageEvents.mockReturnValue(new Promise((resolve) => { resolveSubscription = resolve; }));
    const view = render(<Subscriber sessionId="session-2" />, { wrapper });
    await waitFor(() => expect(service.subscribeMessageEvents).toHaveBeenCalledOnce());

    view.unmount();
    resolveSubscription?.(unsubscribe);
    await waitFor(() => expect(unsubscribe).toHaveBeenCalledOnce());

    expect(service.stopGeneration).not.toHaveBeenCalled();
  });

  it("refetches the list when events target a message the cache has never seen", async () => {
    // A programmatic send, an IM message, or a seat turn creates rows behind this client's
    // back; their stream events cannot create cache rows, so the hook must refetch the list.
    let handler: ((event: ChatStreamEvent) => void) | undefined;
    service.subscribeMessageEvents.mockImplementation(
      (_sessionId: string, callback: (event: ChatStreamEvent) => void) => {
        handler = callback;
        return Promise.resolve(() => {});
      },
    );
    const client = new QueryClient();
    client.setQueryData(["messages", "session-3"], [] as ChatMessage[]);
    const invalidateSpy = vi.spyOn(client, "invalidateQueries");
    render(
      <QueryClientProvider client={client}>
        <Subscriber sessionId="session-3" />
      </QueryClientProvider>,
    );
    await waitFor(() => expect(handler).toBeDefined());

    // A terminal event flushes synchronously, so no animation-frame pump is needed.
    handler?.({ type: "completed", sessionId: "session-3", messageId: "seat-turn-1" });

    await waitFor(() =>
      expect(invalidateSpy).toHaveBeenCalledWith({ queryKey: ["messages", "session-3"] }));
  });
});
