import { renderToString } from "react-dom/server";
import { describe, expect, it } from "vitest";
import "../i18n";
import type { Session } from "../types/agent";
import {
  ParticipantAvatar,
  participantRoleKind,
  SessionRosterAvatars,
  SessionRosterChips,
} from "./session-roster-presence";

const session: Session = {
  id: "session-roster", title: "Shared", agentId: "codex-cli", interactionMode: "cli",
  lifecycleState: "running", folder: null, projectPath: null, worktreePath: null,
  worktreeName: null, worktreeBranch: null, remoteWorkspace: null, remoteSshConnectionId: null,
  remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null,
  source: { kind: "desktop", connector: null }, pinned: false, archived: false,
  createdAt: "2026-08-10T00:00:00Z", updatedAt: "2026-08-10T00:00:00Z",
  seats: [
    { seatId: "seat-1", agentId: "codex-cli", roleId: null, roleSnapshot: { roleName: "Reviewer", avatar: "🔍", color: "#111111", responsibility: null, agentName: "Codex", modelFamily: "openai", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-2", agentId: "gemini-cli", roleId: null, roleSnapshot: { roleName: "Architect", avatar: "🏗️", color: "#222222", responsibility: null, agentName: "Gemini", modelFamily: "google", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-3", agentId: "claude-code", roleId: "builtin-implementer", roleSnapshot: { roleName: "实现者", avatar: "🔧", color: "#333333", responsibility: null, agentName: "Claude", modelFamily: "anthropic", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-4", agentId: "opencode", roleId: "custom-ops", roleSnapshot: { roleName: "运维", avatar: "🧭", color: "#444444", responsibility: null, agentName: "OpenCode", modelFamily: "unknown", crossFamilyReviewer: false }, leftAt: null },
    { seatId: "seat-5", agentId: "antigravity-cli", roleId: null, leftAt: null },
    { seatId: "seat-old", agentId: "claude-code", roleId: null, leftAt: "2026-08-09T00:00:00Z" },
  ],
};

describe("session roster presence", () => {
  it("shows only active participants and marks the current turn holder", () => {
    const html = renderToString(<><SessionRosterAvatars session={session} /><SessionRosterChips currentSeatId="seat-2" session={session} /></>);
    expect(html).toContain("Reviewer");
    expect(html).toContain("Architect");
    expect(html).not.toContain("seat-old");
    expect(html).toContain('aria-current="true"');
    expect(html).toContain('data-role-icon="architect"');
    expect(html).toContain('data-role-icon="implementer"');
    expect(html).toContain('data-role-icon="reviewer"');
    expect(html).toContain('data-role-icon="custom"');
    expect(html).toContain('data-role-icon="agent"');
    expect(html).toContain('data-layout="single-row"');
    expect(html).toContain("grid-cols-[1.75rem_minmax(0,1fr)_3.25rem]");
    expect(html.match(/data-testid="participant-speaking-state"/g)).toHaveLength(5);
    expect(html.match(/aria-hidden="true" class="[^"]*invisible/g)).toHaveLength(4);
    const architectLabel = html.indexOf(">Architect</span>");
    const architectCli = html.indexOf(">gemini-cli</span>", architectLabel);
    const speakingState = html.indexOf(">处理中</span>", architectCli);
    expect(architectLabel).toBeGreaterThan(-1);
    expect(architectCli).toBeGreaterThan(architectLabel);
    expect(speakingState).toBeGreaterThan(architectCli);
  });

  it("recognizes built-in roles by stable id or captured localized name", () => {
    expect(participantRoleKind("builtin-architect", null)).toBe("architect");
    expect(participantRoleKind(null, "实现者")).toBe("implementer");
    expect(participantRoleKind(null, "代码审查")).toBe("reviewer");
    expect(participantRoleKind("custom-security", "安全专家")).toBeNull();
    const html = renderToString(
      <ParticipantAvatar agentId="codex-cli" label="安全专家 · Codex" roleAvatar="🛡️" roleName="安全专家" />,
    );
    expect(html).toContain('data-role-icon="custom"');
  });

  it("uses a built-in role id as the primary label when the snapshot is absent", () => {
    const html = renderToString(
      <SessionRosterChips
        session={{
          ...session,
          seats: [
            { seatId: "seat-architect", agentId: "codex-cli", roleId: "builtin-architect", leftAt: null },
            { seatId: "seat-implementer", agentId: "claude-code", roleId: "builtin-implementer", leftAt: null },
          ],
        }}
      />,
    );

    expect(html).toContain("架构师");
    expect(html).toContain("实现者");
    expect(html).toMatch(/>架构师<\/span><span[^>]*>codex-cli<\/span>/);
  });
});
