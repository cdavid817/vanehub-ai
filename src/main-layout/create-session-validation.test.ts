import { describe, expect, it } from "vitest";
import { createInitialCreateSessionDraft, createSessionDraftReducer } from "./create-session-draft-model";
import { validateCreateSessionDraft } from "./create-session-validation";
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

const claudeCode: AgentRegistryEntry = {
  id: "claude-code",
  displayName: "Claude Code",
  provider: "Anthropic",
  launch: { kind: "cli" },
  supportedInteractionModes: ["cli"],
  availabilityState: "available",
  capabilityTags: ["coding", "cli", "agent"],
  agentOrigin: "builtin",
};

const codexCli: AgentRegistryEntry = {
  ...claudeCode,
  id: "codex-cli",
  displayName: "Codex CLI",
  provider: "OpenAI",
};

describe("validateCreateSessionDraft", () => {
  it("names the remote-workspace-unsupported Agent as the reason, not just a false canSubmit", () => {
    const draft = createSessionDraftReducer(
      createSessionDraftReducer(createInitialCreateSessionDraft(), {
        type: "set-workspace-mode",
        mode: "remote",
      }),
      { type: "set-remote-host", value: "host" },
    );
    const withPath = createSessionDraftReducer(draft, { type: "set-remote-path", value: "/work" });

    const result = validateCreateSessionDraft(withPath, onepiece, [onepiece, claudeCode]);

    expect(result.canSubmit).toBe(false);
    expect(result.agent).toBe("agent-remote-unsupported");
    // The workspace fields themselves are fine; only the Agent/workspace-mode pairing is not.
    expect(result.workspace).toBeNull();
  });

  it("flags a multi-seat draft whose seat Agent is no longer selectable (task 11.2's agentMode gap)", () => {
    const staleAgent: AgentRegistryEntry = {
      ...codexCli,
      availabilityState: "unavailable",
      unavailableReason: "Session expired.",
    };
    let draft = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-project-path",
      path: "D:/work",
    });
    draft = createSessionDraftReducer(draft, {
      type: "set-agent-mode",
      mode: "multi",
      seedSeats: [
        { agentId: "claude-code", roleId: null },
        { agentId: "codex-cli", roleId: null },
      ],
    });

    const result = validateCreateSessionDraft(draft, claudeCode, [claudeCode, staleAgent]);

    expect(result.seats).toBe("seats-agent-unselectable");
    expect(result.canSubmit).toBe(false);
  });

  it("allows a multi-seat draft whose seats are all still selectable", () => {
    let draft = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-project-path",
      path: "D:/work",
    });
    draft = createSessionDraftReducer(draft, {
      type: "set-agent-mode",
      mode: "multi",
      seedSeats: [
        { agentId: "claude-code", roleId: null },
        { agentId: "codex-cli", roleId: null },
      ],
    });

    const result = validateCreateSessionDraft(draft, claudeCode, [claudeCode, codexCli]);

    expect(result.seats).toBeNull();
    expect(result.canSubmit).toBe(true);
  });

  it("reports a missing local project path with a distinct reason from a missing worktree name", () => {
    const bare = validateCreateSessionDraft(createInitialCreateSessionDraft(), claudeCode, [claudeCode]);
    expect(bare.workspace).toBe("workspace-path-missing");

    let draft = createSessionDraftReducer(createInitialCreateSessionDraft(), {
      type: "set-project-path",
      path: "D:/work",
    });
    draft = createSessionDraftReducer(draft, { type: "set-worktree-enabled", enabled: true });
    const missingWorktreeName = validateCreateSessionDraft(draft, claudeCode, [claudeCode]);

    expect(missingWorktreeName.workspace).toBe("workspace-worktree-name-missing");
  });
});
