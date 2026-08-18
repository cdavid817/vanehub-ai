// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { act, renderHook, waitFor } from "@testing-library/react";
import type { ReactNode } from "react";
import { describe, expect, it, vi } from "vitest";
import type { AgentRegistryEntry, Session } from "../types/agent";
import type { AgentRunnerDescriptor } from "../types/agent-runner";
import { useRunnerSelection } from "./use-runner-selection";

const session = { id: "session-1", agentId: "codex-cli" } as Session;
const local: AgentRunnerDescriptor = { selection: { kind: "local" }, label: "Local", hostLabel: null, available: true, unavailableReason: null, simulated: false, capabilities: { interactiveInput: true, pty: false, cancellation: true, inspection: true, recovery: "none" } };
const ssh = (revision: number): AgentRunnerDescriptor => ({ selection: { kind: "ssh", targetId: "ssh-1", targetRevision: revision }, label: "SSH", hostLabel: "host", available: true, unavailableReason: null, simulated: false, capabilities: { interactiveInput: true, pty: true, cancellation: true, inspection: true, recovery: "inspect_only" } });

function wrapper({ children }: { children: ReactNode }) {
  return <QueryClientProvider client={new QueryClient({ defaultOptions: { queries: { retry: false } } })}>{children}</QueryClientProvider>;
}

describe("useRunnerSelection", () => {
  it("defaults Local and revalidates a stale SSH revision back to Local", async () => {
    const listAgentRunners = vi.fn().mockResolvedValueOnce([local, ssh(1)]).mockResolvedValueOnce([local, ssh(2)]);
    const { result } = renderHook(() => useRunnerSelection(session, [], { listAgentRunners }), { wrapper });
    await waitFor(() => expect(result.current.descriptors).toHaveLength(2));
    act(() => result.current.setSelection(ssh(1).selection));
    expect(result.current.selection).toEqual(ssh(1).selection);
    await act(() => result.current.refetch());
    await waitFor(() => expect(result.current.selection).toEqual({ kind: "local" }));
  });

  it("exposes non-Local choices as unavailable for API Agents", async () => {
    const agent = { id: "codex-cli", launch: { kind: "api" } } as AgentRegistryEntry;
    const { result } = renderHook(() => useRunnerSelection(session, [agent], { listAgentRunners: vi.fn().mockResolvedValue([local, ssh(1)]) }), { wrapper });
    await waitFor(() => expect(result.current.descriptors).toHaveLength(2));
    expect(result.current.descriptors[1]).toMatchObject({ available: false, unavailableReason: "runner_api_local_only" });
  });
});
