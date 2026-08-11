import { describe, expect, it, vi } from "vitest";
import { canCreateSession, defaultSshConnectionDraft, firstMode, resolveCreatedSession, sshConnectionSaveErrorKey } from "./create-session-dialog-utils";
import { defaultSessionAgent, groupSessionAgents, isSessionAgentSelectable, selectSessionAgents } from "./create-session-agents";
import type { AgentRegistryEntry, Session } from "../types/agent";

const agent = {
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
  launch: { kind: "cli" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: ["cli"],
  agentOrigin: "builtin",
} satisfies AgentRegistryEntry;

function canCreate(saveSshConnection: boolean, authMode: "key" | "password") {
  return canCreateSession({
    agentMode: "single",
    multiSeats: [],
    projectPath: "",
    remoteHost: "host",
    remotePath: "/work",
    remotePort: "22",
    remoteUser: "dev",
    saveSshConnection,
    selectedAgent: agent,
    sshConnectionDraft: {
      ...defaultSshConnectionDraft,
      authMode,
      keyPath: "",
      password: "",
    },
    workspaceMode: "remote",
    worktreeEnabled: false,
    worktreeName: "",
  });
}

describe("create-session SSH connection validation", () => {
  it("does not block a temporary remote session on profile authentication fields", () => {
    expect(canCreate(false, "key")).toBe(true);
  });

  it("requires the selected save authentication secret", () => {
    expect(canCreate(true, "key")).toBe(false);
    expect(canCreate(true, "password")).toBe(false);
    expect(
      sshConnectionSaveErrorKey("dev", {
        ...defaultSshConnectionDraft,
        authMode: "password",
        password: "secret",
      }),
    ).toBeNull();
  });
});

describe("create-session agent discovery", () => {
  const apiAgent: AgentRegistryEntry = {
    id: "custom-api",
    displayName: "Custom API",
    provider: "Provider",
    launch: { kind: "api" },
    supportedInteractionModes: ["api"],
    availabilityState: "available",
    capabilityTags: ["api"],
    agentOrigin: "user",
  };
  const onepiece: AgentRegistryEntry = {
    ...apiAgent,
    id: "onepiece",
    displayName: "OnePiece",
    agentOrigin: "builtin",
  };

  it("discovers CLI and API candidates without a fixed id allowlist", () => {
    expect(selectSessionAgents([apiAgent, onepiece, agent]).map((value) => value.id))
      .toEqual(["codex-cli", "onepiece", "custom-api"]);
  });

  it("orders built-in CLIs before OnePiece and defaults to Codex CLI", () => {
    const claude = { ...agent, id: "claude-code", displayName: "Claude Code" };
    const gemini = { ...agent, id: "gemini-cli", displayName: "Gemini CLI" };
    const opencode = { ...agent, id: "opencode", displayName: "OpenCode" };
    const candidates = selectSessionAgents([onepiece, opencode, gemini, claude, agent]);

    expect(candidates.map((value) => value.id)).toEqual([
      "codex-cli",
      "claude-code",
      "gemini-cli",
      "opencode",
      "onepiece",
    ]);
    expect(defaultSessionAgent(candidates, null)?.id).toBe("codex-cli");
  });

  it("keeps Claude Code and Codex CLI selectable when only their optional SDK is missing", () => {
    for (const [id, sdkId] of [
      ["claude-code", "claude-sdk"],
      ["codex-cli", "codex-sdk"],
    ] as const) {
      const cliAgent: AgentRegistryEntry = {
        ...agent,
        id,
        managedSdkDependencyId: sdkId,
        availabilityState: "unavailable",
        unavailableReason: `Managed SDK dependency '${sdkId}' is not installed.`,
      };

      expect(isSessionAgentSelectable(cliAgent)).toBe(true);
      expect(canCreateSession({
        agentMode: "single",
        multiSeats: [],
        projectPath: "D:/project",
        remoteHost: "",
        remotePath: "",
        remotePort: "22",
        remoteUser: "",
        saveSshConnection: false,
        selectedAgent: cliAgent,
        sshConnectionDraft: defaultSshConnectionDraft,
        workspaceMode: "local",
        worktreeEnabled: false,
        worktreeName: "",
      })).toBe(true);
    }
  });

  it("keeps the CLI default unless ready OnePiece was selected previously", () => {
    const candidates = selectSessionAgents([onepiece, agent]);
    expect(defaultSessionAgent(candidates, null)?.id).toBe("codex-cli");
    expect(defaultSessionAgent(candidates, "onepiece")?.id).toBe("onepiece");
    expect(defaultSessionAgent(
      [{ ...onepiece, availabilityState: "needs-auth" }, agent],
      "onepiece",
    )?.id).toBe("codex-cli");
  });

  it("groups native, built-in CLI, and custom API candidates without changing ids", () => {
    const groups = groupSessionAgents(selectSessionAgents([apiAgent, onepiece, agent]));

    expect(groups.map((group) => group.id)).toEqual([
      "builtin-cli",
      "native",
      "custom-api",
    ]);
    expect(groups.map((group) => group.agents.map((value) => value.id))).toEqual([
      ["codex-cli"],
      ["onepiece"],
      ["custom-api"],
    ]);
    expect(firstMode(onepiece)).toBe("api");
    expect(firstMode(agent)).toBe("cli");
  });

  it("allows OnePiece local folders and worktrees", () => {
    expect(canCreateSession({
      agentMode: "single",
      multiSeats: [],
      projectPath: "D:/project",
      remoteHost: "",
      remotePath: "",
      remotePort: "22",
      remoteUser: "",
      saveSshConnection: false,
      selectedAgent: onepiece,
      sshConnectionDraft: defaultSshConnectionDraft,
      workspaceMode: "local",
      worktreeEnabled: true,
      worktreeName: "onepiece-worktree",
    })).toBe(true);
  });

  it("rejects remote OnePiece submission", () => {
    expect(canCreateSession({
      agentMode: "single",
      multiSeats: [],
      projectPath: "",
      remoteHost: "host",
      remotePath: "/work",
      remotePort: "22",
      remoteUser: "dev",
      saveSshConnection: false,
      selectedAgent: onepiece,
      sshConnectionDraft: defaultSshConnectionDraft,
      workspaceMode: "remote",
      worktreeEnabled: false,
      worktreeName: "",
    })).toBe(false);
  });
});

describe("canCreateSession in multi-Agent mode", () => {
  function canCreateMulti(multiSeats: { agentId: string; roleId: string | null }[]) {
    return canCreateSession({
      agentMode: "multi",
      multiSeats,
      projectPath: "D:/work",
      remoteHost: "",
      remotePath: "",
      remotePort: "22",
      remoteUser: "",
      saveSshConnection: false,
      selectedAgent: agent,
      sshConnectionDraft: defaultSshConnectionDraft,
      workspaceMode: "local",
      worktreeEnabled: false,
      worktreeName: "",
    });
  }

  it("allows submitting once two seats are bound to Agents", () => {
    expect(canCreateMulti([
      { agentId: "claude-code", roleId: "builtin-architect" },
      { agentId: "codex-cli", roleId: "builtin-reviewer" },
    ])).toBe(true);
  });

  // One seat is a single-Agent session wearing the wrong mode, so it must not submit as multi.
  it("blocks a single seat", () => {
    expect(canCreateMulti([{ agentId: "claude-code", roleId: null }])).toBe(false);
  });

  it("blocks a seat with no Agent chosen", () => {
    expect(canCreateMulti([
      { agentId: "claude-code", roleId: null },
      { agentId: "  ", roleId: null },
    ])).toBe(false);
  });

  // Roles are optional: a seat may be a plain Agent with no role assigned.
  it("does not require every seat to carry a role", () => {
    expect(canCreateMulti([
      { agentId: "claude-code", roleId: null },
      { agentId: "codex-cli", roleId: null },
    ])).toBe(true);
  });
});

describe("resolveCreatedSession", () => {
  const canonicalSession = {
    id: "session-multi",
    title: "Shared implementation",
    agentId: "codex-cli",
    seats: [
      { seatId: "seat-architect", agentId: "codex-cli", roleId: "builtin-architect", joinedAt: "2026-08-10T00:00:00Z", leftAt: null },
      { seatId: "seat-implementer", agentId: "claude-code", roleId: "builtin-implementer", joinedAt: "2026-08-10T00:00:00Z", leftAt: null },
    ],
    interactionMode: "cli",
    lifecycleState: "idle",
    folder: "D:\\work\\app",
    projectPath: "D:\\work\\app",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    source: { kind: "desktop", connector: null },
    pinned: false,
    archived: false,
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    createdAt: "2026-08-10T00:00:00Z",
    updatedAt: "2026-08-10T00:00:00Z",
  } satisfies Session;

  it("replaces a desktop operation projection with the canonical multi-Agent roster", async () => {
    const loadSession = vi.fn().mockResolvedValue(canonicalSession);

    const result = await resolveCreatedSession(
      {
        id: "session-multi",
        title: "Shared implementation",
        agentId: "codex-cli",
        seats: [
          { agentId: "codex-cli", roleId: "builtin-architect" },
          { agentId: "claude-code", roleId: "builtin-implementer" },
        ],
        interactionMode: "cli",
      },
      loadSession,
    );

    expect(loadSession).toHaveBeenCalledWith("session-multi");
    expect(result?.seats).toEqual(canonicalSession.seats);
  });

  it("does not query sessions for an invalid operation result", async () => {
    const loadSession = vi.fn();

    await expect(resolveCreatedSession({ id: "session-multi" }, loadSession)).resolves.toBeNull();
    expect(loadSession).not.toHaveBeenCalled();
  });
});
