import { describe, expect, it } from "vitest";
import type { PlanDraft } from "../types/plan";
import { validatePlanDraftReview } from "./plan-draft-editor";

function draft(): PlanDraft {
  return {
    id: "plan-1", versionId: "version-1", version: 1, goal: "Ship", projectPath: "D:/app",
    baseRef: "main", plannerProfileId: "profile-1",
    discovery: { status: "complete", limitations: [] },
    executionPolicy: { maxAttemptsPerSubtask: 3, repairEligibleClasses: ["verification_failed"], finalValidationCommands: [{ id: "final", program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }] },
    subtasks: ["a", "b"].map((id, ordinal) => ({
      id, ordinal, title: id, description: `Do ${id}`, acceptanceCriteria: ["passes"],
      criterionEvidence: [{ criterionIndex: 0, kind: "automated", commandId: `test-${id}` }],
      assignedRole: "worker", limits: { tokenBudget: 1000, toolCallLimit: 10, timeoutSeconds: 60 },
      validationCommands: [{ id: `test-${id}`, program: "npm", args: ["test"], workingDirectory: null, timeoutSeconds: 60, required: true }],
    })),
    dependencies: [{ predecessorId: "a", successorId: "b" }],
  };
}

describe("Plan draft review validation", () => {
  it("accepts a complete DAG and reports cycles or invalid acceptance criteria", () => {
    const valid = draft();
    expect(validatePlanDraftReview(valid)).toBeNull();
    expect(validatePlanDraftReview({ ...valid, dependencies: [...valid.dependencies, ...valid.dependencies] })).toBe("plans.validation.edge");
    expect(validatePlanDraftReview({ ...valid, dependencies: [...valid.dependencies, { predecessorId: "b", successorId: "a" }] })).toBe("plans.validation.cycle");
    valid.subtasks[0]!.acceptanceCriteria = [];
    expect(validatePlanDraftReview(valid)).toBe("plans.validation.criteria");
  });
});
