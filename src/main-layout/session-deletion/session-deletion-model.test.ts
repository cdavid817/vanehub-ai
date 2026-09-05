import { describe, expect, it } from "vitest";
import type {
  DeletionGroupResult,
  DeletionPreviewWorktree,
  SessionDeletionOperation,
  SessionDeletionPreview,
} from "../../types/session-deletion";
import {
  anyRemovalChosen,
  buildChoices,
  canSubmit,
  confirmLabelKey,
  emptyChoices,
  remainingSessions,
  retryAllowed,
  retryNeedsPreview,
  setAcknowledgement,
  toggleRemove,
} from "./session-deletion-model";
import type { Session } from "../../types/agent";

function worktree(overrides: Partial<DeletionPreviewWorktree> = {}): DeletionPreviewWorktree {
  return {
    worktreeKey: "wt-1",
    worktreeId: "wt-1",
    displayPath: "/repo-feature",
    branch: "vanehub/feature",
    sessionIds: ["s1"],
    externalReferences: [],
    allowedPolicies: ["keep", "remove-safe"],
    blockers: [],
    checks: "complete",
    changes: { trackedModified: 0, staged: 0, conflicted: 0, untracked: 0 },
    ignored: null,
    requiresIgnoredAcknowledgement: false,
    origin: "ordinary_session",
    provenance: "verified",
    resourceStatus: "attached",
    ...overrides,
  };
}

function preview(worktrees: DeletionPreviewWorktree[]): SessionDeletionPreview {
  return {
    previewId: "pv-1",
    runtimeEffect: "native",
    createdAt: "t",
    expiresAt: "t",
    sessions: [{ sessionId: "s1", title: "S1", archived: false, active: false, workspaceKind: "worktree", worktreeKey: "wt-1", displayPath: "/repo-feature" }],
    worktrees,
  };
}

function group(overrides: Partial<DeletionGroupResult>): DeletionGroupResult {
  return {
    groupId: "g",
    worktreeKey: null,
    worktreeId: null,
    policy: "keep",
    sessionIds: ["s1"],
    status: "succeeded",
    phase: "completed",
    worktreeEffect: "not_requested",
    dbEffect: "deleted",
    errorCode: null,
    retainedPath: null,
    attempt: 1,
    revision: 2,
    ...overrides,
  };
}

function operation(groups: DeletionGroupResult[]): SessionDeletionOperation {
  return {
    operationId: "op",
    requestId: "r",
    outcome: "partial",
    phase: "completed",
    revision: 3,
    runtimeEffect: "native",
    createdAt: "t",
    updatedAt: "t",
    completedAt: "t",
    groups,
    errorCode: null,
    operationTaskId: null,
  };
}

describe("session deletion choices", () => {
  it("default to keep for every worktree and never persist a destructive choice", () => {
    const choices = emptyChoices(preview([worktree()]));
    expect(choices["wt-1"]).toEqual({ remove: false, acknowledgedFingerprint: null });
    expect(anyRemovalChosen(choices)).toBe(false);
    expect(confirmLabelKey(false)).toBe("sessionDeletion.confirmSessionOnly");
    expect(buildChoices(preview([worktree()]), choices)).toEqual([{ worktreeKey: "wt-1", policy: "keep" }]);
  });

  it("ignore a toggle on a worktree that only allows keep", () => {
    const row = worktree({ allowedPolicies: ["keep"], blockers: ["tracked_changes"] });
    const choices = toggleRemove(emptyChoices(preview([row])), row);
    expect(choices["wt-1"].remove).toBe(false);
  });

  it("switch the confirm label and require the acknowledgement bound to the preview fingerprint", () => {
    const row = worktree({
      requiresIgnoredAcknowledgement: true,
      ignored: { totalEntries: 2, samples: [], samplesTruncated: false, completeness: "complete", fingerprint: "fp-a" },
    });
    const removed = toggleRemove(emptyChoices(preview([row])), row);
    expect(anyRemovalChosen(removed)).toBe(true);
    expect(confirmLabelKey(true)).toBe("sessionDeletion.confirmWithWorktree");
    expect(canSubmit(preview([row]), removed)).toBe(false);
    const acknowledged = setAcknowledgement(removed, row, true);
    expect(canSubmit(preview([row]), acknowledged)).toBe(true);
    expect(buildChoices(preview([row]), acknowledged)).toEqual([
      { worktreeKey: "wt-1", policy: "remove-safe", ignoredFilesAcknowledgement: { fingerprint: "fp-a" } },
    ]);
    // A changed inventory invalidates the acknowledgement the user gave.
    const stale = { ...row, ignored: { ...row.ignored!, fingerprint: "fp-b" } };
    expect(canSubmit(preview([stale]), acknowledged)).toBe(false);
    // Unticking removal drops the acknowledgement rather than remembering it.
    expect(toggleRemove(acknowledged, row)["wt-1"]).toEqual({ remove: false, acknowledgedFingerprint: null });
  });

  it("refuse to submit a removal the preview no longer allows", () => {
    const row = worktree();
    const removed = toggleRemove(emptyChoices(preview([row])), row);
    const blocked = worktree({ allowedPolicies: ["keep"] });
    expect(canSubmit(preview([blocked]), removed)).toBe(false);
  });
});

describe("session deletion retry rules", () => {
  it("allow a retry only for unfinished groups that are not parked", () => {
    expect(retryAllowed(operation([group({ status: "succeeded" })]))).toBe(false);
    expect(retryAllowed(operation([group({ status: "succeeded" }), group({ status: "failed", dbEffect: "retained" })]))).toBe(true);
    expect(retryAllowed(operation([group({ status: "needs_attention", worktreeEffect: "removal_unknown", dbEffect: "retained" })]))).toBe(false);
  });

  it("need a new preview unless only the database step is left", () => {
    expect(retryNeedsPreview(operation([group({ status: "finalize_pending", worktreeEffect: "removed", dbEffect: "pending" })]))).toBe(false);
    expect(retryNeedsPreview(operation([group({ status: "awaiting_decision", worktreeEffect: "retained", dbEffect: "retained" })]))).toBe(true);
  });

  it("keep only the sessions the operation did not delete", () => {
    const sessions = [{ id: "s1" }, { id: "s2" }] as Session[];
    const remaining = remainingSessions(sessions, operation([
      group({ sessionIds: ["s1"], status: "succeeded", dbEffect: "deleted" }),
      group({ sessionIds: ["s2"], status: "failed", dbEffect: "retained" }),
    ]));
    expect(remaining.map((session) => session.id)).toEqual(["s2"]);
  });
});
