// @vitest-environment jsdom

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PlanExitCard, parseProposedPlan } from "./PlanExitCard";

const { agentService } = vi.hoisted(() => ({
  agentService: { resolvePlanExit: vi.fn() },
}));
vi.mock("../../services/runtime-agent-client", () => ({ agentService }));
vi.mock("react-i18next", () => ({
  useTranslation: () => ({ t: (key: string) => key }),
}));

describe("parseProposedPlan", () => {
  it("reads the plan straight off the tool input", () => {
    expect(parseProposedPlan({ plan: "Rename the module." })).toBe("Rename the module.");
  });

  it("returns null for input that carries no plan", () => {
    expect(parseProposedPlan({ plan: "   " })).toBeNull();
    expect(parseProposedPlan({ question: "Which?" })).toBeNull();
    expect(parseProposedPlan(null)).toBeNull();
    expect(parseProposedPlan(["plan"])).toBeNull();
  });
});

describe("PlanExitCard", () => {
  beforeEach(() => {
    agentService.resolvePlanExit.mockReset();
    agentService.resolvePlanExit.mockResolvedValue(true);
  });

  it("shows the plan the user is being asked to approve", () => {
    render(<PlanExitCard callId="call-1" input={{ plan: "Rename the module." }} sessionId="session-1" />);
    expect(screen.getByText("Rename the module.")).toBeTruthy();
  });

  it("sends approval and decline as distinct decisions", async () => {
    const user = userEvent.setup();
    render(<PlanExitCard callId="call-1" input={{ plan: "Do the work." }} sessionId="session-1" />);

    await user.click(screen.getByRole("button", { name: "chat.toolPlanExit.approve" }));
    await waitFor(() => expect(agentService.resolvePlanExit).toHaveBeenCalledWith("session-1", "call-1", true));

    await user.click(screen.getByRole("button", { name: "chat.toolPlanExit.decline" }));
    await waitFor(() => expect(agentService.resolvePlanExit).toHaveBeenCalledWith("session-1", "call-1", false));
  });

  // Both buttons disable together while a decision is in flight: a second click would resolve a
  // call that is already resolved, and the two buttons carry opposite decisions.
  it("does not send a second decision while one is in flight", async () => {
    let release: (delivered: boolean) => void = () => {};
    agentService.resolvePlanExit.mockReturnValue(new Promise<boolean>((resolve) => { release = resolve; }));
    const user = userEvent.setup();
    render(<PlanExitCard callId="call-1" input={{ plan: "Do the work." }} sessionId="session-1" />);

    await user.click(screen.getByRole("button", { name: "chat.toolPlanExit.approve" }));
    await user.click(screen.getByRole("button", { name: "chat.toolPlanExit.decline" }));
    expect(agentService.resolvePlanExit).toHaveBeenCalledTimes(1);

    release(true);
  });

  it("renders nothing when the call carries no plan", () => {
    const { container } = render(<PlanExitCard callId="call-1" input={{}} sessionId="session-1" />);
    expect(container.firstChild).toBeNull();
  });
});
