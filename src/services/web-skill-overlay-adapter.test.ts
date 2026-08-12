import { describe, expect, it } from "vitest";
import type { SkillOverlayWitnesses } from "../types/skill-overlay";
import { webAgentClient } from "./web-agent-client";

describe("Web Skill Overlay adapter", () => {
  it("executes Overlay state transitions through the AgentService boundary", async () => {
    const target = { skillId: "unit-test-generation", scope: "user" as const, workspacePath: null };
    const summary = await webAgentClient.getSkillOverlaySummary(target);
    const witnesses: SkillOverlayWitnesses = {
      expectedOverlayRevision: null,
      expectedBaseInstructionHash: summary.baseInstructionHash,
      expectedBasePackageHash: summary.basePackageHash,
      expectedPayloadHash: null,
      expectedPinned: summary.pinned,
    };

    const outcome = await webAgentClient.createSkillOverlayGuidance({
      target,
      witnesses,
      guidance: "Cover boundary values.",
    });

    expect(outcome).toMatchObject({ committedRevision: 1, summary: { status: "healthy" } });
    await expect(webAgentClient.getSkillOverlayDetail(target)).resolves.toMatchObject({
      effectiveInstructions: { content: expect.stringContaining("Cover boundary values.") },
      mutations: [{ kind: "learnedGuidance", state: "active" }],
    });
    await expect(webAgentClient.getSkillOverlayHistory({ target, limit: 10 })).resolves.toMatchObject({
      integrity: "verified",
      entries: [{ action: "learn", nextRevision: 1 }],
    });
  });
});
