import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { webAgentClient } from "./web-agent-client";
import { resetWebSessionDeletions, webSessionDeletionClient } from "./web-session-deletion-client";
import { configureWebSessionDeletion, resetWebSessionDeletionScenario } from "./web-session-deletion-simulation";
import { resetWebSessionClaims } from "./web-session-deletion-state";
import type { SessionDeletionOperation } from "../types/session-deletion";

async function createWorktreeSession(name: string) {
  // The mock prepends the session synchronously; its operation settles on a timer, which these
  // tests do not need to wait for.
  const title = `${name}-${Date.now()}-${Math.random().toString(16).slice(2)}`;
  await webAgentClient.createSession({
    agentId: "codex-cli",
    interactionMode: "cli",
    title,
    projectPath: "D:\\code\\app",
    worktree: { enabled: true, name },
  });
  const created = (await webAgentClient.listSessions()).find((session) => session.title === title);
  if (!created) throw new Error("mock session missing");
  return created;
}

async function settled(operationId: string): Promise<SessionDeletionOperation> {
  await Promise.resolve();
  await Promise.resolve();
  return webSessionDeletionClient.getSessionDeletionOperation(operationId);
}

describe("web session deletion client", () => {
  beforeEach(() => {
    resetWebSessionDeletions();
    resetWebSessionDeletionScenario();
    resetWebSessionClaims();
  });

  afterEach(() => {
    resetWebSessionDeletionScenario();
  });

  it("previews as simulated, keeps by default, and deletes only the session on keep", async () => {
    const session = await createWorktreeSession("feature-keep");
    const preview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [session.id, session.id] });
    expect(preview.runtimeEffect).toBe("simulated");
    expect(preview.sessions).toHaveLength(1);
    expect(preview.worktrees[0].allowedPolicies).toEqual(["keep", "remove-safe"]);
    expect(preview.worktrees[0].worktreeId).not.toBeNull();

    const handle = await webSessionDeletionClient.executeSessionDeletion({ requestId: "r1", previewId: preview.previewId, worktreeChoices: [] });
    expect(handle.runtimeEffect).toBe("simulated");
    expect(handle.existing).toBe(false);
    const operation = await settled(handle.operationId);
    expect(operation.outcome).toBe("succeeded");
    expect(operation.groups[0].worktreeEffect).toBe("retained");
    expect(operation.groups[0].dbEffect).toBe("deleted");
    expect((await webAgentClient.listSessions()).some((item) => item.id === session.id)).toBe(false);
  });

  it("simulates a removal, dedupes shared worktrees, and never claims a native effect", async () => {
    const first = await createWorktreeSession("feature-shared");
    const preview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [first.id] });
    const key = preview.worktrees[0].worktreeKey;
    const handle = await webSessionDeletionClient.executeSessionDeletion({
      requestId: "r2",
      previewId: preview.previewId,
      worktreeChoices: [{ worktreeKey: key, policy: "remove-safe" }],
    });
    const operation = await settled(handle.operationId);
    expect(operation.runtimeEffect).toBe("simulated");
    expect(operation.groups[0].worktreeEffect).toBe("removed");
    expect(operation.outcome).toBe("succeeded");
  });

  it("refuses removals the preview did not allow and binds acknowledgements to the fingerprint", async () => {
    const dirty = await createWorktreeSession("dirty-tree");
    const dirtyPreview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [dirty.id] });
    expect(dirtyPreview.worktrees[0].allowedPolicies).toEqual(["keep"]);
    expect(dirtyPreview.worktrees[0].blockers).toContain("tracked_changes");
    await expect(webSessionDeletionClient.executeSessionDeletion({
      requestId: "r3",
      previewId: dirtyPreview.previewId,
      worktreeChoices: [{ worktreeKey: dirtyPreview.worktrees[0].worktreeKey, policy: "remove-safe" }],
    })).rejects.toThrow("deletion_policy_not_allowed");

    const ignored = await createWorktreeSession("ignored-config");
    const ignoredPreview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [ignored.id] });
    const row = ignoredPreview.worktrees[0];
    expect(row.requiresIgnoredAcknowledgement).toBe(true);
    await expect(webSessionDeletionClient.executeSessionDeletion({
      requestId: "r4",
      previewId: ignoredPreview.previewId,
      worktreeChoices: [{ worktreeKey: row.worktreeKey, policy: "remove-safe" }],
    })).rejects.toThrow("deletion_ignored_acknowledgement_required");
    await expect(webSessionDeletionClient.executeSessionDeletion({
      requestId: "r5",
      previewId: ignoredPreview.previewId,
      worktreeChoices: [{ worktreeKey: row.worktreeKey, policy: "remove-safe", ignoredFilesAcknowledgement: { fingerprint: "stale" } }],
    })).rejects.toThrow("deletion_ignored_acknowledgement_stale");
  });

  it("returns the same operation for an identical request id and conflicts on different content", async () => {
    const session = await createWorktreeSession("feature-idempotent");
    const preview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [session.id] });
    configureWebSessionDeletion({ quiesceTimeoutSessions: new Set([session.id]) });
    const first = await webSessionDeletionClient.executeSessionDeletion({ requestId: "same", previewId: preview.previewId, worktreeChoices: [] });
    await settled(first.operationId);
    const again = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [session.id] });
    const second = await webSessionDeletionClient.executeSessionDeletion({ requestId: "same", previewId: again.previewId, worktreeChoices: [] });
    expect(second.existing).toBe(true);
    expect(second.operationId).toBe(first.operationId);
    const third = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [session.id] });
    await expect(webSessionDeletionClient.executeSessionDeletion({
      requestId: "same",
      previewId: third.previewId,
      worktreeChoices: [{ worktreeKey: third.worktrees[0].worktreeKey, policy: "remove-safe" }],
    })).rejects.toThrow("deletion_request_id_conflict");
  });

  it("reports partial batches per group and retries only the unfinished group with a fresh preview", async () => {
    const good = await createWorktreeSession("feature-good");
    const refused = await createWorktreeSession("refuse-me");
    const preview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [good.id, refused.id] });
    const handle = await webSessionDeletionClient.executeSessionDeletion({
      requestId: "batch",
      previewId: preview.previewId,
      worktreeChoices: preview.worktrees.map((worktree) => ({ worktreeKey: worktree.worktreeKey, policy: "remove-safe" as const })),
    });
    const operation = await settled(handle.operationId);
    expect(operation.outcome).toBe("partial");
    const failed = operation.groups.find((group) => group.status === "awaiting_decision");
    expect(failed?.errorCode).toBe("worktree_removal_refused");
    expect((await webAgentClient.listSessions()).some((item) => item.id === refused.id)).toBe(true);
    expect((await webAgentClient.listSessions()).some((item) => item.id === good.id)).toBe(false);

    await expect(webSessionDeletionClient.retrySessionDeletion({
      operationId: operation.operationId,
      expectedRevision: operation.revision,
      retryRequestId: "retry-1",
      worktreeChoices: [{ worktreeKey: failed!.worktreeKey!, policy: "remove-safe" }],
    })).rejects.toThrow("deletion_retry_requires_preview");

    const fresh = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [refused.id] });
    const retry = await webSessionDeletionClient.retrySessionDeletion({
      operationId: operation.operationId,
      expectedRevision: operation.revision,
      retryRequestId: "retry-2",
      previewId: fresh.previewId,
      worktreeChoices: [],
    });
    const finished = await settled(retry.operationId);
    expect(finished.outcome).toBe("succeeded");
    expect(finished.groups.every((group) => group.status === "succeeded")).toBe(true);
  });

  it("leaves a finalize-pending group with its claim and finishes it on a database-only retry", async () => {
    const session = await createWorktreeSession("feature-finalize");
    configureWebSessionDeletion({ finalizeFailureSessions: [session.id] });
    const preview = await webSessionDeletionClient.previewSessionDeletion({ sessionIds: [session.id] });
    const handle = await webSessionDeletionClient.executeSessionDeletion({
      requestId: "finalize",
      previewId: preview.previewId,
      worktreeChoices: [{ worktreeKey: preview.worktrees[0].worktreeKey, policy: "remove-safe" }],
    });
    const operation = await settled(handle.operationId);
    expect(operation.outcome).toBe("needs_attention");
    expect(operation.groups[0].status).toBe("finalize_pending");
    await expect(webAgentClient.deleteSession(session.id)).rejects.toThrow("session_deletion_in_progress");
    resetWebSessionDeletionScenario();
    const retry = await webSessionDeletionClient.retrySessionDeletion({
      operationId: operation.operationId,
      expectedRevision: operation.revision,
      retryRequestId: "retry-db",
      worktreeChoices: [],
    });
    const finished = await settled(retry.operationId);
    expect(finished.outcome).toBe("succeeded");
    expect((await webAgentClient.listSessions()).some((item) => item.id === session.id)).toBe(false);
  });

  it("refuses system activity ids, empty selections and unknown sessions before touching anything", async () => {
    await expect(webSessionDeletionClient.previewSessionDeletion({ sessionIds: [] })).rejects.toThrow("deletion_empty_selection");
    await expect(webSessionDeletionClient.previewSessionDeletion({ sessionIds: ["system-activity-v1-x"] })).rejects.toThrow("system_activity_session_refused");
    await expect(webSessionDeletionClient.previewSessionDeletion({ sessionIds: ["missing"] })).rejects.toThrow("Session not found");
    expect(await webSessionDeletionClient.listPendingSessionDeletions()).toEqual([]);
  });
});
