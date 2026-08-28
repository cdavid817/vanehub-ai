import { describe, expect, it } from "vitest";
import { seatMentionOptions } from "./seat-mention-options";
import type { Session } from "../types/agent";

const session: Session = {
  id: "session-1",
  title: "Shared",
  agentId: "codex-cli",
  seats: [
    { seatId: "seat-1", agentId: "codex-cli", roleId: null, roleSnapshot: { roleName: "Reviewer", avatar: "🔍", color: "#111111", responsibility: null, agentName: "Codex", modelFamily: "openai", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-2", agentId: "gemini-cli", roleId: null, roleSnapshot: { roleName: "Reviewer", avatar: "🔍", color: "#222222", responsibility: null, agentName: "Gemini", modelFamily: "google", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-3", agentId: "claude-code", roleId: null, roleSnapshot: { roleName: "Former", avatar: "🕰️", color: "#333333", responsibility: null, agentName: "Claude", modelFamily: "anthropic", crossFamilyReviewer: false }, leftAt: "2026-08-10T00:00:00Z" },
  ],
  interactionMode: "cli", personalizationMode: "standard", lifecycleState: "idle", folder: null, projectPath: null,
  worktreePath: null, worktreeName: null, worktreeBranch: null, remoteWorkspace: null,
  remoteSshConnectionId: null, remoteSshConnectionRevision: null, runtimeSessionId: null,
  categoryId: null, source: { kind: "desktop", connector: null }, pinned: false, archived: false,
  recoveryStatus: "clean", recoveryRevision: 0, stateRevision: 0, historyRevision: 0,
  activeExecutionRunId: null,
  createdAt: "2026-08-10T00:00:00Z", updatedAt: "2026-08-10T00:00:00Z",
};

describe("seatMentionOptions", () => {
  it("creates unique handles for active participants and excludes departed ones", () => {
    expect(seatMentionOptions(session, [], []).map((option) => option.mention)).toEqual([
      "Reviewer",
      "Reviewer-2",
    ]);
  });
});
