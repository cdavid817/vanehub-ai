import { describe, expect, it } from "vitest";
import type { KnownProject, KnownRemoteWorkspace, ProjectInspection, Session } from "../types/agent";
import type { SshConnection } from "../types/ssh-connection";
import {
  buildLocalWorkspaceSummary,
  buildRemoteWorkspaceSummary,
  buildWorkspaceSummaries,
} from "./workspace-aggregation";

function project(overrides: Partial<KnownProject> = {}): KnownProject {
  return { path: "D:\\repo\\app", displayName: "app", isGit: true, lastOpenedAt: "2026-08-01T00:00:00.000Z", ...overrides };
}

function remoteWorkspace(overrides: Partial<KnownRemoteWorkspace> = {}): KnownRemoteWorkspace {
  return {
    host: "dev.example.com", port: 22, user: "vane", path: "/work/app",
    displayName: "dev.example.com:app", uri: "ssh://vane@dev.example.com/work/app",
    lastOpenedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function connection(overrides: Partial<SshConnection> = {}): SshConnection {
  return {
    id: "conn-1", name: "Dev box", host: "dev.example.com", port: 22, user: "vane",
    defaultPath: "/work/app", authMode: "key", keyPath: "/home/vane/.ssh/id_ed25519",
    hasPassword: false, revision: 1, hostTrust: null, testStatus: "not-tested",
    lastConnectedAt: null, lastError: null,
    createdAt: "2026-08-01T00:00:00.000Z", updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1", personalizationMode: "standard", title: "Session", agentId: "claude-code",
    interactionMode: "cli", lifecycleState: "idle", recoveryStatus: "clean",
    recoveryRevision: 0, stateRevision: 0, historyRevision: 0, activeExecutionRunId: null,
    folder: null, projectPath: null, worktreePath: null, worktreeName: null, worktreeBranch: null,
    remoteWorkspace: null, remoteSshConnectionId: null, remoteSshConnectionRevision: null,
    runtimeSessionId: null, categoryId: null, pinned: false, archived: false,
    createdAt: "2026-08-01T00:00:00.000Z", updatedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

function inspection(overrides: Partial<ProjectInspection> = {}): ProjectInspection {
  return { path: "D:\\repo\\app", displayName: "app", isGit: true, gitRoot: "D:\\repo\\app", ...overrides };
}

describe("buildLocalWorkspaceSummary", () => {
  it("joins the most recently updated matching session and reads git.repository from a live inspection", () => {
    const target = project();
    const older = session({ id: "s-old", projectPath: target.path, updatedAt: "2026-07-01T00:00:00.000Z" });
    const newer = session({ id: "s-new", projectPath: target.path, title: "Newest", updatedAt: "2026-08-15T00:00:00.000Z" });
    const unrelated = session({ id: "s-other", projectPath: "D:\\repo\\other" });

    const summary = buildLocalWorkspaceSummary(target, [older, newer, unrelated], inspection());

    expect(summary.workspaceId).toBe("D:\\repo\\app");
    expect(summary.kind).toBe("local");
    expect(summary.availability).toBe("available");
    expect(summary.git).toEqual({ repository: true });
    expect(summary.trust).toBeUndefined();
    expect(summary.recentSession).toEqual({ id: "s-new", title: "Newest", lifecycleState: "idle", updatedAt: "2026-08-15T00:00:00.000Z" });
  });

  it("leaves recentSession undefined when no session matches this project's path", () => {
    const summary = buildLocalWorkspaceSummary(project(), [session({ projectPath: "D:\\repo\\other" })], inspection());
    expect(summary.recentSession).toBeUndefined();
  });

  it("classifies a rejected inspection as missing and falls back to the project's own last-known isGit", () => {
    const summary = buildLocalWorkspaceSummary(project({ isGit: true }), [], null);
    expect(summary.availability).toBe("missing");
    expect(summary.git).toEqual({ repository: true });
  });

  it("does not claim a repository the live inspection says is not one, even if isGit once said so", () => {
    const summary = buildLocalWorkspaceSummary(project({ isGit: true }), [], inspection({ gitRoot: null }));
    expect(summary.git).toEqual({ repository: false });
  });
});

describe("buildRemoteWorkspaceSummary", () => {
  it("reports trusted/available only once a matching connection has a confirmed host and a succeeded test", () => {
    const remote = remoteWorkspace();
    const conn = connection({ hostTrust: { host: "dev.example.com", port: 22, algorithm: "ssh-ed25519", fingerprint: "abc", confirmedAt: "2026-08-01T00:00:00.000Z" }, testStatus: "succeeded" });
    const linked = session({ id: "s-linked", remoteSshConnectionId: conn.id, updatedAt: "2026-08-10T00:00:00.000Z" });

    const summary = buildRemoteWorkspaceSummary(remote, [conn], [linked]);

    expect(summary.workspaceId).toBe(remote.uri);
    expect(summary.kind).toBe("ssh");
    expect(summary.trust).toBe("trusted");
    expect(summary.availability).toBe("available");
    expect(summary.recentSession?.id).toBe("s-linked");
  });

  it("reports unknown/disconnected for a matched-but-untested connection rather than guessing available", () => {
    const remote = remoteWorkspace();
    const conn = connection({ testStatus: "not-tested" });

    const summary = buildRemoteWorkspaceSummary(remote, [conn], []);

    expect(summary.trust).toBe("unknown");
    expect(summary.availability).toBe("disconnected");
  });

  it("falls back to the session's own embedded remoteWorkspace.uri when no connection profile matches at all", () => {
    const remote = remoteWorkspace({ host: "orphan.example.com", uri: "ssh://vane@orphan.example.com/work/app" });
    const embedded = session({ id: "s-embedded", remoteWorkspace: { host: remote.host, port: remote.port, user: remote.user, path: remote.path, displayName: remote.displayName, uri: remote.uri } });
    const unrelated = session({ id: "s-unrelated", remoteWorkspace: { host: "other.example.com", port: 22, user: null, path: "/x", displayName: "other", uri: "ssh://other.example.com/x" } });

    const summary = buildRemoteWorkspaceSummary(remote, [connection({ host: "not-a-match.example.com" })], [embedded, unrelated]);

    expect(summary.trust).toBe("unknown");
    expect(summary.availability).toBe("disconnected");
    expect(summary.recentSession?.id).toBe("s-embedded");
  });
});

describe("buildWorkspaceSummaries", () => {
  it("combines local and remote rows using each row's own inspection result", () => {
    const local = project({ path: "D:\\repo\\missing", displayName: "missing" });
    const remote = remoteWorkspace();
    const summaries = buildWorkspaceSummaries({
      connections: [],
      inspections: new Map([[local.path, null]]),
      projects: [local],
      remoteWorkspaces: [remote],
      sessions: [],
    });

    expect(summaries).toHaveLength(2);
    expect(summaries.find((item) => item.kind === "local")?.availability).toBe("missing");
    expect(summaries.find((item) => item.kind === "ssh")?.workspaceId).toBe(remote.uri);
  });
});
