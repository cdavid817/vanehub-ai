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
});
