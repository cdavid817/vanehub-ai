// @vitest-environment jsdom

import { act, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import "../../i18n";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import type { AgentRun, AgentRunState } from "../../types/agent-run";
import { AgentRunOwnerStatus } from "./agent-run-owner-status";

const listAgentRuns = vi.fn();

vi.mock("../../services/runtime-agent-client", () => ({
  agentService: {
    cancelAgentRun: vi.fn(),
    listAgentRuns: (...args: unknown[]) => listAgentRuns(...args),
    resumeAgentRun: vi.fn(),
  },
}));

function run(state: AgentRunState): AgentRun {
  return {
    createdAt: "2026-08-06T10:00:00Z",
    id: "run-1",
    lastWitness: "witness",
    links: [],
    maxRetries: 3,
    owner: { ownerId: "m1", ownerType: "session_generation" },
    parentRunId: null,
    reasonCode: null,
    recoveryPolicy: "owner_reconciles",
    retryCount: 0,
    state,
    updatedAt: "2026-08-06T10:00:05Z",
    version: 1,
  };
}

function resolving(...pages: (AgentRun | null)[]) {
  listAgentRuns.mockImplementation(() => {
    const next = pages.length > 1 ? pages.shift() : pages[0];
    return Promise.resolve({ items: next ? [next] : [], limit: 1, offset: 0 });
  });
}

/** Lets the pending fetch settle and then runs the timers it scheduled. */
async function elapse(ms: number) {
  await act(async () => { await Promise.resolve(); });
  await act(async () => { await vi.advanceTimersByTimeAsync(ms); });
}

describe("AgentRunOwnerStatus", () => {
  beforeEach(async () => {
    vi.useFakeTimers();
    listAgentRuns.mockReset();
    await activateAppLanguage("zh-CN");
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it("stops observing once the run reaches a terminal state", async () => {
    resolving(run("completed"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(10_000);

    expect(listAgentRuns).toHaveBeenCalledTimes(1);
  });

  it("keeps the terminal run on screen after it stops observing", async () => {
    resolving(run("completed"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(10_000);

    expect(screen.getByTestId("agent-run-status").getAttribute("data-state")).toBe("completed");
  });

  it("stops observing an owner that has no run and no active work", async () => {
    resolving(null);

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(10_000);

    expect(listAgentRuns).toHaveBeenCalledTimes(1);
  });

  it("keeps observing while the owner's work is active and the run has not appeared", async () => {
    // Otherwise a message that starts streaming before its Run exists would render a status that
    // stays blank for the whole generation.
    resolving(null);

    renderWithAppProviders(<AgentRunOwnerStatus active ownerId="m1" ownerType="session_generation" />);
    await elapse(3_000);

    expect(listAgentRuns.mock.calls.length).toBeGreaterThan(1);
  });

  it("keeps observing a run that has not reached a terminal state", async () => {
    resolving(run("running"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(3_000);

    expect(listAgentRuns.mock.calls.length).toBeGreaterThan(1);
  });

  it("keeps a loaded run on screen when an observation fails", async () => {
    // A failed round trip says nothing about the Run. Treating it as "no Run" blanks a status the
    // user was already reading, and does it for a message whose Run is long since finished.
    listAgentRuns
      .mockResolvedValueOnce({ items: [run("running")], limit: 1, offset: 0 })
      .mockRejectedValue(new Error("ipc unavailable"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(3_000);

    expect(screen.getByTestId("agent-run-status").getAttribute("data-state")).toBe("running");
  });

  it("retries after a failed observation instead of giving up", async () => {
    listAgentRuns.mockRejectedValue(new Error("ipc unavailable"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(3_000);

    expect(listAgentRuns.mock.calls.length).toBeGreaterThan(1);
  });

  it("gives up on an owner whose observations keep failing", async () => {
    // Retrying forever would reintroduce the permanent per-message timer this change removes.
    listAgentRuns.mockRejectedValue(new Error("ipc unavailable"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(30_000);

    expect(listAgentRuns.mock.calls.length).toBeLessThanOrEqual(4);
  });

  it("bounds failed observations even while the owner's work is active", async () => {
    // An owner stuck reporting `streaming` -- an interrupted generation persists that way -- would
    // otherwise hold exactly the permanent per-message timer this change removes.
    listAgentRuns.mockRejectedValue(new Error("ipc unavailable"));

    renderWithAppProviders(<AgentRunOwnerStatus active ownerId="m1" ownerType="session_generation" />);
    await elapse(30_000);

    expect(listAgentRuns.mock.calls.length).toBeLessThanOrEqual(4);
  });

  it("drops the previous owner's run when the owner changes", async () => {
    // The status is reused across selections in the Loop Center, so a failed first observation for
    // the new owner must not leave the old owner's state, elapsed clock and Cancel target on screen.
    listAgentRuns.mockResolvedValueOnce({ items: [run("running")], limit: 1, offset: 0 });
    const { rerender } = renderWithAppProviders(
      <AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />,
    );
    await elapse(0);
    listAgentRuns.mockRejectedValue(new Error("ipc unavailable"));

    rerender(<AgentRunOwnerStatus ownerId="m2" ownerType="session_generation" />);
    await elapse(1_000);

    expect(screen.queryByTestId("agent-run-status")).toBeNull();
  });

  it("stops observing once a running run becomes terminal", async () => {
    resolving(run("running"), run("completed"));

    renderWithAppProviders(<AgentRunOwnerStatus ownerId="m1" ownerType="session_generation" />);
    await elapse(10_000);

    expect(listAgentRuns).toHaveBeenCalledTimes(2);
  });
});
