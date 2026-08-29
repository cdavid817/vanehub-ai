import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  publishWebCuratorNotification,
  subscribeWebSkillCuratorNotifications,
} from "./web-skill-curator-notifications";
import {
  ensureWorkspace,
  getPolicy,
  resetWebSkillCuratorForTest,
  setPolicy,
} from "./web-skill-curator-state";

describe("Web Skill Curator notifications", () => {
  beforeEach(() => resetWebSkillCuratorForTest());

  it("deduplicates the safe projection by candidate revision and event kind", async () => {
    const candidate = ensureWorkspace("mock://deterministic")[0];
    const handler = vi.fn();
    await subscribeWebSkillCuratorNotifications(handler);

    publishWebCuratorNotification(candidate, "pending_review");
    publishWebCuratorNotification(candidate, "pending_review");

    expect(handler).toHaveBeenCalledOnce();
    expect(handler.mock.calls[0][0]).toEqual({
      schemaVersion: 1,
      eventKind: "pending_review",
      candidateId: candidate.detail.candidateId,
      candidateRevision: candidate.detail.revision,
      workspaceId: candidate.detail.workspaceId,
      skillId: candidate.detail.targetSkillId,
      overlayScope: candidate.detail.overlayScope,
      state: candidate.detail.state,
      risk: candidate.detail.risk,
      route: candidate.detail.route,
      navigationTarget: { kind: "candidate_review", candidateId: candidate.detail.candidateId },
    });
    expect(JSON.stringify(handler.mock.calls[0][0])).not.toContain("rationale");
  });

  it("suppresses disabled notifications and isolates subscriber failures", async () => {
    const disabled = ensureWorkspace("mock://disabled")[0];
    setPolicy({ ...getPolicy(disabled.detail.workspaceId), notificationsEnabled: false });
    const observer = vi.fn();
    await subscribeWebSkillCuratorNotifications(() => {
      throw new Error("consumer failed");
    });
    await subscribeWebSkillCuratorNotifications(observer);

    expect(() => publishWebCuratorNotification(disabled, "rejection")).not.toThrow();
    expect(observer).not.toHaveBeenCalled();

    const enabled = ensureWorkspace("mock://enabled")[0];
    expect(() => publishWebCuratorNotification(enabled, "rejection")).not.toThrow();
    expect(observer).toHaveBeenCalledOnce();
  });
});
