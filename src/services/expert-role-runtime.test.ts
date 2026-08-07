import { describe, expect, it } from "vitest";
import { validateExpertRoleInput } from "./expert-role-runtime";
import type { SaveExpertRoleInput } from "../types/expert-role";

function input(overrides: Partial<SaveExpertRoleInput> = {}): SaveExpertRoleInput {
  return {
    displayName: "架构师",
    avatar: "🏛",
    color: "#9B7EBD",
    responsibility: "负责系统设计与技术选型",
    instruction: "你是本次会话的架构师…",
    skillIds: [],
    reviewPolicy: { peerReviewer: false, requireDifferentFamily: false },
    preferredProviders: [],
    ...overrides,
  };
}

describe("validateExpertRoleInput", () => {
  it("accepts a complete role", () => {
    expect(validateExpertRoleInput(input())).toEqual([]);
  });

  it("requires a display name", () => {
    expect(validateExpertRoleInput(input({ displayName: "  " }))).toContain("displayName is required");
  });

  // The responsibility is published to other Agents as the basis for choosing whom to hand off to,
  // so an empty one silently breaks routing rather than just looking untidy.
  it("requires a responsibility", () => {
    expect(validateExpertRoleInput(input({ responsibility: "" }))).toContain("responsibility is required");
  });

  it("requires an instruction", () => {
    expect(validateExpertRoleInput(input({ instruction: "" }))).toContain("instruction is required");
  });

  it("reports every missing field at once rather than stopping at the first", () => {
    const errors = validateExpertRoleInput(input({ displayName: "", responsibility: "", instruction: "" }));
    expect(errors).toHaveLength(3);
  });

  it("rejects a colour that is not a hex value", () => {
    expect(validateExpertRoleInput(input({ color: "purple" }))).toContain("color must be a hex value");
  });

  it("rejects duplicate skill references", () => {
    expect(validateExpertRoleInput(input({ skillIds: ["a", "a"] }))).toContain("skillIds must not repeat");
  });

  // requireDifferentFamily only means anything for a role that can be recommended as a reviewer.
  it("rejects requiring a different family when the role is not review-eligible", () => {
    const errors = validateExpertRoleInput(
      input({ reviewPolicy: { peerReviewer: false, requireDifferentFamily: true } }),
    );
    expect(errors).toContain("requireDifferentFamily needs peerReviewer");
  });
});
