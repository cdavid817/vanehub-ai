// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import "../../i18n";
import type { Session } from "../../types/agent";
import type { SessionDeletionOperation, SessionDeletionPreview } from "../../types/session-deletion";
import { SessionDeletionDialog } from "./session-deletion-dialog";
import type { DeletionDialogState } from "./session-deletion-model";
import type { SessionDeletionController } from "./use-session-deletion";

function session(id: string): Session {
  return {
    id,
    title: `会话 ${id}`,
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard",
    lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "/repo",
    projectPath: "/repo",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-09-05T00:00:00Z",
    updatedAt: "2026-09-05T00:00:00Z",
  };
}

const worktreePreview: SessionDeletionPreview = {
  previewId: "pv-1",
  runtimeEffect: "simulated",
  createdAt: "t",
  expiresAt: "t",
  sessions: [{ sessionId: "s1", title: "会话 s1", archived: false, active: false, workspaceKind: "worktree", worktreeKey: "wt-1", displayPath: "\\\\?\\D:\\code\\app-feature" }],
  worktrees: [{
    worktreeKey: "wt-1",
    worktreeId: "wt-1",
    displayPath: "\\\\?\\D:\\code\\app-feature",
    branch: "vanehub/feature",
    sessionIds: ["s1"],
    externalReferences: [],
    allowedPolicies: ["keep", "remove-safe"],
    blockers: [],
    checks: "complete",
    changes: { trackedModified: 0, staged: 0, conflicted: 0, untracked: 0 },
    ignored: { totalEntries: 2, samples: [{ path: ".env", kind: "file", size: 12, modifiedUnix: 0 }], samplesTruncated: false, completeness: "complete", fingerprint: "fp-a" },
    requiresIgnoredAcknowledgement: true,
    origin: "ordinary_session",
    provenance: "verified",
    resourceStatus: "attached",
  }],
};

function controller(state: DeletionDialogState, overrides: Partial<SessionDeletionController> = {}): SessionDeletionController {
  return {
    state,
    busy: false,
    request: vi.fn(),
    close: vi.fn(),
    toggleWorktree: vi.fn(),
    acknowledgeIgnored: vi.fn(),
    refresh: vi.fn(),
    confirm: vi.fn(async () => undefined),
    retry: vi.fn(async () => undefined),
    ...overrides,
  };
}

describe("SessionDeletionDialog", () => {
  it("shows the project note without any worktree option for an ordinary session", () => {
    const preview: SessionDeletionPreview = {
      ...worktreePreview,
      sessions: [{ sessionId: "s1", title: "会话 s1", archived: false, active: false, workspaceKind: "project", worktreeKey: null, displayPath: "/repo" }],
      worktrees: [],
    };
    render(<SessionDeletionDialog controller={controller({ status: "ready", sessions: [session("s1")], preview, choices: {}, requestId: "r", error: null, retryOf: null })} />);
    expect(screen.getByRole("dialog", { name: "删除会话「会话 s1」？" })).toBeTruthy();
    expect(screen.getByTestId("session-deletion-project-note").textContent).toContain("项目目录及其中的文件不会被删除");
    expect(screen.queryByTestId("session-deletion-remove-worktree")).toBeNull();
    expect(screen.getByTestId("session-deletion-confirm").textContent).toBe("仅删除会话");
    expect(screen.getByTestId("session-deletion-cancel")).toBe(document.activeElement);
  });

  it("defaults to keep, switches the confirm label, and gates removal on the ignored acknowledgement", async () => {
    const user = userEvent.setup();
    const toggleWorktree = vi.fn();
    const base = { status: "ready" as const, sessions: [session("s1")], preview: worktreePreview, requestId: "r", error: null, retryOf: null };
    const { rerender } = render(<SessionDeletionDialog controller={controller({ ...base, choices: { "wt-1": { remove: false, acknowledgedFingerprint: null } } }, { toggleWorktree })} />);
    const checkbox = screen.getByTestId<HTMLInputElement>("session-deletion-remove-worktree");
    expect(checkbox.checked).toBe(false);
    expect(screen.getByTestId("session-deletion-confirm").textContent).toBe("仅删除会话");
    expect(screen.getByTestId("session-deletion-worktree-path").textContent).toBe("D:\\code\\app-feature");
    expect(screen.getByTestId("session-deletion-simulated")).toBeTruthy();
    await user.click(checkbox);
    expect(toggleWorktree).toHaveBeenCalledTimes(1);

    rerender(<SessionDeletionDialog controller={controller({ ...base, choices: { "wt-1": { remove: true, acknowledgedFingerprint: null } } })} />);
    expect(screen.getByTestId("session-deletion-confirm").textContent).toBe("删除会话及 worktree");
    expect((screen.getByTestId("session-deletion-confirm") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByTestId("session-deletion-ignored").textContent).toContain(".env");

    rerender(<SessionDeletionDialog controller={controller({ ...base, choices: { "wt-1": { remove: true, acknowledgedFingerprint: "fp-a" } } })} />);
    expect((screen.getByTestId("session-deletion-confirm") as HTMLButtonElement).disabled).toBe(false);
  });

  it("keeps a blocked worktree visible with its reasons and the option disabled", () => {
    const preview: SessionDeletionPreview = {
      ...worktreePreview,
      worktrees: [{ ...worktreePreview.worktrees[0], allowedPolicies: ["keep"], blockers: ["tracked_changes", "external_references"], requiresIgnoredAcknowledgement: false, ignored: null }],
    };
    render(<SessionDeletionDialog controller={controller({ status: "ready", sessions: [session("s1")], preview, choices: { "wt-1": { remove: false, acknowledgedFingerprint: null } }, requestId: "r", error: null, retryOf: null })} />);
    expect(screen.getByTestId<HTMLInputElement>("session-deletion-remove-worktree").disabled).toBe(true);
    expect(screen.getByTestId("session-deletion-worktree-blockers").textContent).toContain("有未提交的已跟踪修改");
    expect(screen.getByTestId("session-deletion-worktree-blockers").textContent).toContain("其他会话或任务仍在使用该目录");
  });

  it("blocks closing while executing and renders per-group results afterwards", () => {
    const close = vi.fn();
    const executing = controller({ status: "executing", sessions: [session("s1")], preview: worktreePreview, requestId: "r", operationId: "op", operation: null }, { close });
    const { rerender } = render(<SessionDeletionDialog controller={executing} />);
    expect((screen.getByTestId("session-deletion-cancel") as HTMLButtonElement).disabled).toBe(true);
    expect(screen.queryByTestId("session-deletion-confirm")).toBeNull();

    const operation: SessionDeletionOperation = {
      operationId: "op", requestId: "r", outcome: "partial", phase: "completed", revision: 3, runtimeEffect: "simulated",
      createdAt: "t", updatedAt: "t", completedAt: "t", errorCode: "worktree_removal_refused", operationTaskId: null,
      groups: [
        { groupId: "g1", worktreeKey: "wt-1", worktreeId: "wt-1", policy: "remove-safe", sessionIds: ["s1"], status: "awaiting_decision", phase: "completed", worktreeEffect: "retained", dbEffect: "retained", errorCode: "worktree_removal_refused", retainedPath: "/repo-feature", attempt: 1, revision: 2 },
        { groupId: "g2", worktreeKey: null, worktreeId: null, policy: "keep", sessionIds: ["s2"], status: "succeeded", phase: "completed", worktreeEffect: "not_requested", dbEffect: "deleted", errorCode: null, retainedPath: null, attempt: 1, revision: 2 },
      ],
    };
    rerender(<SessionDeletionDialog controller={controller({ status: "settled", sessions: [session("s1"), session("s2")], preview: worktreePreview, operation })} />);
    const result = screen.getByTestId("session-deletion-result");
    expect(result.getAttribute("data-outcome")).toBe("partial");
    expect(result.textContent).toContain("部分完成");
    expect(result.textContent).toContain("Git 拒绝移除该 worktree");
    expect(screen.getByTestId("session-deletion-retry")).toBeTruthy();
    expect((screen.getByTestId("session-deletion-cancel") as HTMLButtonElement).disabled).toBe(false);
  });
});
