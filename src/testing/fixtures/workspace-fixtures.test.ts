import { describe, expect, it } from "vitest";
import {
  EMPTY_WORKSPACE_LIST,
  generateWorkspaceSummaries,
  localGitWorkspace,
  missingWorkspace,
  nonGitWorkspace,
  remoteConnectedWorkspace,
  remoteDisconnectedWorkspace,
  revokedWorkspace,
  untrustedWorkspace,
} from "./workspace-fixtures";

describe("workspace fixtures (task 13.13)", () => {
  it("shapes localGitWorkspace as an available local Git repository", () => {
    const workspace = localGitWorkspace();
    expect(workspace).toMatchObject({ availability: "available", git: { repository: true }, kind: "local" });
  });

  it("shapes nonGitWorkspace as an available local plain folder, not an absent git context", () => {
    const workspace = nonGitWorkspace();
    expect(workspace).toMatchObject({ availability: "available", git: { repository: false }, kind: "local" });
  });

  it("shapes missingWorkspace as a local row whose inspection rejected", () => {
    const workspace = missingWorkspace();
    expect(workspace).toMatchObject({ availability: "missing", kind: "local" });
    // Historical isGit carried through, not dropped -- buildLocalWorkspaceSummary's own fallback.
    expect(workspace.git?.repository).toBe(true);
  });

  it("shapes remoteConnectedWorkspace as a trusted, available SSH row with a matched connection", () => {
    const workspace = remoteConnectedWorkspace();
    expect(workspace).toMatchObject({ availability: "available", kind: "ssh", trust: "trusted" });
    expect(workspace.connectionId).toBeTruthy();
  });

  it("shapes remoteDisconnectedWorkspace as an honestly-unknown-trust, disconnected SSH row", () => {
    const workspace = remoteDisconnectedWorkspace();
    expect(workspace).toMatchObject({ availability: "disconnected", kind: "ssh", trust: "unknown" });
  });

  it("shapes untrustedWorkspace/revokedWorkspace as the type-reachable-but-never-produced trust states (13.10)", () => {
    expect(untrustedWorkspace().trust).toBe("untrusted");
    expect(revokedWorkspace().trust).toBe("revoked");
  });

  it("applies overrides on top of the named defaults rather than ignoring them", () => {
    const workspace = localGitWorkspace({ workspaceId: "D:/custom/path", displayName: "custom" });
    expect(workspace.workspaceId).toBe("D:/custom/path");
    expect(workspace.displayName).toBe("custom");
    // Everything not overridden keeps the named scenario's own real shape.
    expect(workspace.availability).toBe("available");
  });

  it("returns a fresh object per call rather than a shared mutable reference", () => {
    const first = localGitWorkspace();
    const second = localGitWorkspace();
    expect(first).not.toBe(second);
    expect(first.git).not.toBe(second.git);
  });

  it("keeps EMPTY_WORKSPACE_LIST genuinely empty", () => {
    expect(EMPTY_WORKSPACE_LIST).toHaveLength(0);
  });

  describe("generateWorkspaceSummaries", () => {
    it("produces exactly `count` rows with unique workspaceId values", () => {
      const rows = generateWorkspaceSummaries(23);
      expect(rows).toHaveLength(23);
      expect(new Set(rows.map((row) => row.workspaceId)).size).toBe(23);
    });

    it("is deterministic for a given seed", () => {
      expect(generateWorkspaceSummaries(15, 7)).toEqual(generateWorkspaceSummaries(15, 7));
    });

    it("cycles through every named scenario's availability/trust shape, never a fabricated one", () => {
      const rows = generateWorkspaceSummaries(14);
      const availabilities = new Set(rows.map((row) => row.availability));
      const trustValues = new Set(rows.map((row) => row.trust).filter((trust) => trust !== undefined));
      expect(availabilities).toEqual(new Set(["available", "missing", "disconnected"]));
      expect(trustValues).toEqual(new Set(["trusted", "unknown", "untrusted", "revoked"]));
    });
  });
});
