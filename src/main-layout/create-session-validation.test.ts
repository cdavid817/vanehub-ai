import { describe, expect, it, vi } from "vitest";
import {
  canCreateSession,
  defaultSshConnectionDraft,
  submitCreateSession,
} from "./create-session-dialog-utils";
import type { AgentRegistryEntry } from "../types/agent";

vi.mock("../services/runtime-agent-client", () => ({
  agentService: { createSession: vi.fn(), getSession: vi.fn() },
}));
vi.mock("../services/runtime-ssh-connection-client", () => ({
  sshConnectionService: { createConnection: vi.fn() },
}));

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

/**
 * The state a user reaches by starting a remote session and changing their mind.
 *
 * They ticked "save this SSH connection", left the username blank, then switched to a local
 * project. The draft is still there, still incomplete, and belongs to a mode whose fields are no
 * longer on screen.
 */
function retainedRemoteDraft() {
  return {
    agentMode: "single" as const,
    multiSeats: [],
    projectPath: "D:/work/project",
    remoteHost: "",
    remotePath: "",
    remotePort: "22",
    remoteUser: "",
    // Left ticked by the abandoned remote attempt. Switching modes never cleared it.
    saveSshConnection: true,
    selectedAgent: agent,
    sshConnectionDraft: { ...defaultSshConnectionDraft, keyPath: "" },
    workspaceMode: "local" as const,
    worktreeEnabled: false,
    worktreeName: "",
  };
}

describe("create-session validation is one rule, not two", () => {
  it("enables submission for a valid local workspace despite a retained remote draft", () => {
    expect(canCreateSession(retainedRemoteDraft())).toBe(true);
  });

  it("actually submits when the control says it can", async () => {
    const { agentService } = await import("../services/runtime-agent-client");
    const createSession = vi.mocked(agentService.createSession);
    createSession.mockResolvedValue({ id: "operation-1" } as never);
    const setError = vi.fn();

    await submitCreateSession({
      ...retainedRemoteDraft(),
      interactionMode: "cli",
      remoteDisplayName: "",
      selectedSshConnectionId: "",
      setCreateOperationId: vi.fn(),
      setError,
      setHandledCreateOperationId: vi.fn(),
      setLoading: vi.fn(),
      title: "Local session",
      t: (key: string) => key,
      personalizationMode: "standard",
    });

    // The enable guard and the submit guard have to agree. When they disagree the button is live,
    // the click does nothing, and the error names a field the active workspace mode never showed.
    expect(setError).not.toHaveBeenCalledWith("sshConnections.validation.user");
    expect(createSession).toHaveBeenCalledTimes(1);
  });

  it("still refuses to save an incomplete SSH connection in remote mode", async () => {
    const { agentService } = await import("../services/runtime-agent-client");
    const createSession = vi.mocked(agentService.createSession);
    createSession.mockClear();
    const setError = vi.fn();

    await submitCreateSession({
      ...retainedRemoteDraft(),
      workspaceMode: "remote",
      remoteHost: "example.com",
      remotePath: "/srv/work",
      projectPath: "",
      interactionMode: "cli",
      remoteDisplayName: "",
      selectedSshConnectionId: "",
      setCreateOperationId: vi.fn(),
      setError,
      setHandledCreateOperationId: vi.fn(),
      setLoading: vi.fn(),
      title: "Remote session",
      t: (key: string) => key,
      personalizationMode: "standard",
    });

    // The rule is not being removed, only scoped. In remote mode the same draft is still invalid.
    expect(setError).toHaveBeenCalledWith("sshConnections.validation.user");
    expect(createSession).not.toHaveBeenCalled();
  });

  it("refuses a remote session whose own fields are incomplete, and says so through the guard", () => {
    expect(
      canCreateSession({
        ...retainedRemoteDraft(),
        workspaceMode: "remote",
        remoteHost: "example.com",
        remotePath: "/srv/work",
        projectPath: "",
      }),
    ).toBe(false);
  });
});
