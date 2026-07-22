import { describe, expect, it } from "vitest";
import {
  canCreateSession,
  defaultSshConnectionDraft,
  sshConnectionSaveErrorKey,
} from "./create-session-dialog-utils";
import type { AgentRegistryEntry } from "../types/agent";

const agent = {
  id: "codex-cli",
} as AgentRegistryEntry;

function canCreate(saveSshConnection: boolean, authMode: "key" | "password") {
  return canCreateSession({
    agentMode: "single",
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
