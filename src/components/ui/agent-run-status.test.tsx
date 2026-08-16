// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AgentRun } from "../../types/agent-run";
import { AgentRunStatus } from "./agent-run-status";

function run(state: AgentRun["state"]): AgentRun {
  return {
    id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
    owner: { ownerType: "test", ownerId: "owner" },
    links: [],
    parentRunId: null,
    state,
    recoveryPolicy: "not_recoverable",
    retryCount: 1,
    maxRetries: 2,
    reasonCode: state === "waiting_approval" ? "permission_required" : null,
    createdAt: "2026-08-16T00:00:00Z",
    updatedAt: "2026-08-16T00:00:01Z",
    version: 3,
    lastWitness: "test",
  };
}

describe("AgentRunStatus", () => {
  it("exposes waiting reason, retry count, and cancel without relying on color", () => {
    const cancel = vi.fn();
    render(<AgentRunStatus elapsed="0:01" onCancel={cancel} run={run("waiting_approval")} />);
    expect(screen.getByRole("status").getAttribute("data-state")).toBe("waiting_approval");
    expect(screen.getByRole("status").getAttribute("data-reason-code")).toBe("permission_required");
    fireEvent.click(screen.getByRole("button", { name: /cancel/i }));
    expect(cancel).toHaveBeenCalledOnce();
  });

  it("offers resume only for resumable states and hides actions for terminal states", () => {
    const resume = vi.fn();
    const view = render(<AgentRunStatus elapsed="1:00" onResume={resume} run={run("paused")} />);
    fireEvent.click(screen.getByRole("button", { name: /resume/i }));
    expect(resume).toHaveBeenCalledOnce();
    view.rerender(<AgentRunStatus elapsed="1:01" onResume={resume} run={run("completed")} />);
    expect(screen.queryByRole("button")).toBeNull();
  });
});
