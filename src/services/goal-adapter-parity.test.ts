import { beforeEach, describe, expect, it, vi } from "vitest";

const { invokeMock } = vi.hoisted(() => ({ invokeMock: vi.fn() }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: invokeMock }));

import { tauriGoalClient } from "./tauri-goal-client";
import { resetWebGoalClient, webGoalClient } from "./web-goal-client";

async function activeGoal(title = "Ship the goal system") {
  const draft = await webGoalClient.createGoal({ title });
  return webGoalClient.activateGoal(draft.id);
}

describe("Goal adapter parity", () => {
  beforeEach(() => {
    invokeMock.mockReset();
    resetWebGoalClient();
  });

  it("passes the same argument shape across every command", async () => {
    const goal = await activeGoal();
    invokeMock.mockResolvedValue(goal);

    await tauriGoalClient.getGoal(goal.id);
    await tauriGoalClient.createGoal({ title: "x" });
    await tauriGoalClient.updateGoal(goal.id, { title: "x" });
    await tauriGoalClient.linkGoalTarget(goal.id, "plan", "plan-1");
    await tauriGoalClient.unlinkGoalTarget(goal.id, "plan", "plan-1");
    await tauriGoalClient.acceptGoal(goal.id);

    expect(invokeMock.mock.calls).toEqual([
      ["get_goal", { goalId: goal.id }],
      ["create_goal", { input: { title: "x" } }],
      ["update_goal", { goalId: goal.id, input: { title: "x" } }],
      ["link_goal_target", { goalId: goal.id, targetKind: "plan", targetId: "plan-1" }],
      ["unlink_goal_target", { goalId: goal.id, targetKind: "plan", targetId: "plan-1" }],
      ["accept_goal", { goalId: goal.id }],
    ]);
  });

  it("returns an identical goal contract from desktop and Web", async () => {
    const web = await activeGoal();
    invokeMock.mockResolvedValueOnce(web);

    await expect(tauriGoalClient.getGoal(web.id)).resolves.toEqual(web);
    expect(web.status).toBe("active");
    expect(web.derivedStatus).toBe("active");
    expect(web.links).toEqual([]);
  });

  it("never reports a stored status of awaiting acceptance", async () => {
    const goal = await activeGoal();
    const ready = await webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-completed");

    expect(ready.derivedStatus).toBe("awaiting_acceptance");
    expect(ready.status).toBe("active");
  });

  it("keeps a retryable plan failure from promoting a goal, unlike a loop failure", async () => {
    const planGoal = await activeGoal("plan side");
    const withFailedPlan = await webGoalClient.linkGoalTarget(planGoal.id, "plan", "web-plan-failed");
    expect(withFailedPlan.derivedStatus).toBe("active");

    const loopGoal = await activeGoal("loop side");
    const withFailedLoop = await webGoalClient.linkGoalTarget(loopGoal.id, "loop", "web-loop-failed");
    expect(withFailedLoop.derivedStatus).toBe("awaiting_acceptance");
  });

  it("does not promote a goal whose child is parked at its own acceptance", async () => {
    const goal = await activeGoal();
    const parked = await webGoalClient.linkGoalTarget(goal.id, "loop", "web-loop-awaiting");

    expect(parked.derivedStatus).toBe("active");
  });

  it("drops an unresolvable child from the denominator instead of blocking acceptance", async () => {
    const goal = await activeGoal();
    await webGoalClient.linkGoalTarget(goal.id, "plan", "deleted-plan");
    const ready = await webGoalClient.linkGoalTarget(goal.id, "loop", "web-loop-succeeded");

    expect(ready.unresolvable).toBe(1);
    expect(ready.counted).toBe(1);
    expect(ready.derivedStatus).toBe("awaiting_acceptance");
  });

  it("never lets sessions alone carry a goal to acceptance", async () => {
    const goal = await activeGoal();
    const withSession = await webGoalClient.linkGoalTarget(goal.id, "session", "web-session-open");

    expect(withSession.counted).toBe(0);
    expect(withSession.derivedStatus).toBe("active");
  });

  it("pulls a goal back out of acceptance when a child reopens", async () => {
    const goal = await activeGoal();
    const ready = await webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-completed");
    expect(ready.derivedStatus).toBe("awaiting_acceptance");

    const reopened = await webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-running");
    expect(reopened.derivedStatus).toBe("active");
  });

  it("rejects acceptance while work is outstanding", async () => {
    const goal = await activeGoal();
    await webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-running");

    await expect(webGoalClient.acceptGoal(goal.id)).rejects.toThrow(
      "A goal can only be accepted while it is awaiting acceptance.",
    );
  });

  it("rejects a duplicate link and a blank title", async () => {
    const goal = await activeGoal();
    await webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-completed");

    await expect(webGoalClient.linkGoalTarget(goal.id, "plan", "web-plan-completed")).rejects.toThrow(
      "This plan is already linked to the goal.",
    );
    await expect(webGoalClient.createGoal({ title: "   " })).rejects.toThrow(
      "Goal title is required.",
    );
  });

  it("rejects an illegal transition with both ends named", async () => {
    const draft = await webGoalClient.createGoal({ title: "draft only" });

    await expect(webGoalClient.acceptGoal(draft.id)).rejects.toThrow(
      "A goal can only be accepted while it is awaiting acceptance.",
    );
    await expect(webGoalClient.reopenGoal(draft.id)).resolves.toMatchObject({ status: "active" });
    await expect(webGoalClient.activateGoal(draft.id)).rejects.toThrow(
      'A goal cannot move from "active" to "active".',
    );
  });

  it("reports a missing goal the same way on both sides", async () => {
    await expect(webGoalClient.getGoal("nope")).rejects.toThrow("The goal was not found.");

    invokeMock.mockRejectedValueOnce("The goal was not found.");
    await expect(tauriGoalClient.getGoal("nope")).rejects.toBe("The goal was not found.");
  });
});
