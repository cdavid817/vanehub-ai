// Deliberately no React/testing-library import: task 11.1 asks for a model that is testable
// without mounting dialog presentation, and this file is the evidence for that claim.
import { describe, expect, it } from "vitest";
import {
  agentSupportsRemoteWorkspace,
  createInitialCreateSessionDraft,
  createSessionDraftReducer,
} from "./create-session-draft-model";
import type { AgentRegistryEntry } from "../types/agent";

const onepiece: AgentRegistryEntry = {
  id: "onepiece",
  displayName: "OnePiece",
  provider: "VaneHub",
  launch: { kind: "api" },
  supportedInteractionModes: ["api"],
  availabilityState: "available",
  capabilityTags: ["coding", "api", "agent", "native"],
  agentOrigin: "builtin",
};

const codexCli: AgentRegistryEntry = {
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
  launch: { kind: "cli" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: ["coding", "cli", "agent"],
  agentOrigin: "builtin",
};

describe("create-session draft reducer", () => {
  it("resets to the given agent and interaction mode, blanking the rest", () => {
    const seeded = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "select-agent",
      agentId: "codex-cli",
      interactionMode: "cli",
    });
    const next = createSessionDraftReducer(seeded, {
      type: "reset",
      agentId: "claude-code",
      interactionMode: "cli",
    });

    expect(next.agentId).toBe("claude-code");
    expect(next.agentMode).toBe("single");
    expect(next.workspaceMode).toBe("local");
    expect(next.personalizationMode).toBe("standard");
  });

  it("does not reset multiSeats/worktree fields on reopen, matching the dialog's pre-extraction behavior", () => {
    const withSeats = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-seats",
      seats: [{ agentId: "claude-code", roleId: null }],
    });
    const reopened = createSessionDraftReducer(withSeats, {
      type: "reset",
      agentId: "claude-code",
      interactionMode: "cli",
    });

    expect(reopened.multiSeats).toEqual([{ agentId: "claude-code", roleId: null }]);
  });

  it("derives the title from the local project path unless the user typed their own", () => {
    const next = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-project-path",
      path: "D:/code/vanehub",
    });

    expect(next.title).toMatch(/^vanehub-\d{8}-\d{6}$/);
  });

  it("stops deriving the title once the user has typed their own", () => {
    const typed = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-title",
      title: "My custom title",
    });
    const afterPathChange = createSessionDraftReducer(typed, {
      type: "set-project-path",
      path: "D:/code/vanehub",
    });

    expect(afterPathChange.title).toBe("My custom title");
  });

  it("re-derives the title from the remote path when workspace mode switches to remote", () => {
    const withRemotePath = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-remote-path",
      value: "/srv/app",
    });
    const switched = createSessionDraftReducer(withRemotePath, {
      type: "set-workspace-mode",
      mode: "remote",
    });

    expect(switched.title).toMatch(/^app-\d{8}-\d{6}$/);
  });

  it("seeds two seats only on the first switch to multi, not on a later one", () => {
    const seedSeats = [
      { agentId: "claude-code", roleId: null },
      { agentId: "codex-cli", roleId: null },
    ];
    const firstSwitch = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-agent-mode",
      mode: "multi",
      seedSeats,
    });
    expect(firstSwitch.multiSeats).toEqual(seedSeats);

    const edited = createSessionDraftReducer(firstSwitch, {
      type: "set-seats",
      seats: [{ agentId: "gemini-cli", roleId: null }],
    });
    const backToSingle = createSessionDraftReducer(edited, {
      type: "set-agent-mode",
      mode: "single",
      seedSeats,
    });
    const backToMulti = createSessionDraftReducer(backToSingle, {
      type: "set-agent-mode",
      mode: "multi",
      seedSeats,
    });

    // Seats survived the round trip untouched -- toggling agentMode is not itself a reset.
    expect(backToMulti.multiSeats).toEqual([{ agentId: "gemini-cli", roleId: null }]);
  });

  it("clears worktreeEnabled when workspace mode changes", () => {
    const withWorktree = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-worktree-enabled",
      enabled: true,
    });
    const switched = createSessionDraftReducer(withWorktree, {
      type: "set-workspace-mode",
      mode: "remote",
    });

    expect(switched.worktreeEnabled).toBe(false);
  });

  it("begin-project-path-inspection resets worktree fields but plain typing does not", () => {
    const withWorktree = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-worktree-enabled",
      enabled: true,
    });
    const typed = createSessionDraftReducer(withWorktree, {
      type: "set-project-path",
      path: "D:/code",
    });
    expect(typed.worktreeEnabled).toBe(true);

    const inspected = createSessionDraftReducer(withWorktree, {
      type: "begin-project-path-inspection",
      path: "D:/code",
    });
    expect(inspected.worktreeEnabled).toBe(false);
  });
});

describe("agentSupportsRemoteWorkspace", () => {
  it("excludes OnePiece, matching the backend's own remote-workspace rejection", () => {
    expect(agentSupportsRemoteWorkspace(onepiece)).toBe(false);
  });

  it("allows every other Agent and a null selection", () => {
    expect(agentSupportsRemoteWorkspace(codexCli)).toBe(true);
    expect(agentSupportsRemoteWorkspace(null)).toBe(true);
  });
});
