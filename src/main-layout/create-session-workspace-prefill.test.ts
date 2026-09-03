import { describe, expect, it, vi } from "vitest";
import type { KnownRemoteWorkspace } from "../types/agent";
import { applyRemoteWorkspacePrefill, findPrefillRemoteWorkspace } from "./create-session-workspace-prefill";

function remoteWorkspace(overrides: Partial<KnownRemoteWorkspace> = {}): KnownRemoteWorkspace {
  return {
    host: "dev.example.com", port: 22, user: "vane", path: "/work/app",
    displayName: "dev.example.com:app", uri: "ssh://vane@dev.example.com/work/app",
    lastOpenedAt: "2026-08-01T00:00:00.000Z", ...overrides,
  };
}

describe("findPrefillRemoteWorkspace", () => {
  it("finds the known remote workspace matching a validated ssh prefill's workspaceId", () => {
    const target = remoteWorkspace();
    const other = remoteWorkspace({ uri: "ssh://vane@other.example.com/work/app", host: "other.example.com" });

    const match = findPrefillRemoteWorkspace({ kind: "ssh", workspaceId: target.uri }, [other, target]);

    expect(match).toBe(target);
  });

  it("returns undefined when the remembered path is no longer in the fetched list", () => {
    const match = findPrefillRemoteWorkspace({ kind: "ssh", workspaceId: "ssh://vane@gone.example.com/app" }, [remoteWorkspace()]);
    expect(match).toBeUndefined();
  });

  it("returns undefined for a local prefill, even if a remote workspace happens to share the id", () => {
    const target = remoteWorkspace();
    const match = findPrefillRemoteWorkspace({ kind: "local", workspaceId: target.uri }, [target]);
    expect(match).toBeUndefined();
  });

  it("returns undefined when there is no prefill at all", () => {
    expect(findPrefillRemoteWorkspace(null, [remoteWorkspace()])).toBeUndefined();
    expect(findPrefillRemoteWorkspace(undefined, [remoteWorkspace()])).toBeUndefined();
  });
});

describe("applyRemoteWorkspacePrefill", () => {
  it("dispatches the same field set create-session-remote-workspace-section.tsx's own selectHistory dispatches, defaulting a missing port to 22", () => {
    const dispatch = vi.fn();
    const match = remoteWorkspace({ port: undefined });

    applyRemoteWorkspacePrefill(match, dispatch);

    expect(dispatch).toHaveBeenCalledWith({ type: "set-workspace-mode", mode: "remote" });
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-host", value: "dev.example.com" });
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-port", value: "22" });
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-user", value: "vane" });
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-path", value: "/work/app" });
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-display-name", value: "dev.example.com:app" });
    expect(dispatch).toHaveBeenCalledTimes(6);
    // Deliberately never dispatched: selectedSshConnectionId stays whatever `reset` already set --
    // a remembered path and a saved connection profile are different concepts (see this module's
    // own doc comment).
    expect(dispatch).not.toHaveBeenCalledWith(expect.objectContaining({ type: "set-selected-ssh-connection-id" }));
  });

  it("defaults a null user to an empty string, matching selectHistory exactly", () => {
    const dispatch = vi.fn();
    applyRemoteWorkspacePrefill(remoteWorkspace({ user: null }), dispatch);
    expect(dispatch).toHaveBeenCalledWith({ type: "set-remote-user", value: "" });
  });
});
