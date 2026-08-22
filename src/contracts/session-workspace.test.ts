import { describe, expect, it } from "vitest";
import { normalizeShellSession } from "./session-workspace";

/**
 * The native runtime already returns a `remote` Session Shell for SSH-backed sessions. The old
 * frontend union admitted only `native | simulated`, so the value that actually crossed the
 * boundary was the one shape TypeScript said could not occur. These cases pin the wire shape of
 * every descriptor variant against the transport, not against the type declaration.
 */
describe("Session Shell runtime descriptor transport", () => {
  it("accepts a local shell and reports the capabilities a PTY actually has", () => {
    const shell = normalizeShellSession({
      shellId: "shell-1",
      sessionId: "session-1",
      state: "connected",
      runtime: { kind: "native" },
    });

    expect(shell).toEqual({
      shellId: "shell-1",
      sessionId: "session-1",
      state: "connected",
      runtime: {
        kind: "native",
        supportsResize: true,
        supportsReplay: true,
        supportsReconnect: false,
      },
    });
  });

  it("accepts the native remote descriptor with its connection witnesses", () => {
    const shell = normalizeShellSession({
      shellId: "shell-2",
      sessionId: "session-1",
      state: "connected",
      runtime: {
        kind: "remote",
        connectionId: "connection-7",
        profileRevision: 3,
        supportsReconnect: false,
      },
    });

    expect(shell.runtime).toEqual({
      kind: "remote",
      connectionId: "connection-7",
      profileRevision: 3,
      supportsResize: true,
      supportsReplay: true,
      supportsReconnect: false,
    });
  });

  it("does not let a simulated shell claim resize support", () => {
    const shell = normalizeShellSession({
      shellId: "shell-3",
      sessionId: "session-1",
      state: "connected",
      runtime: { kind: "simulated" },
    });

    expect(shell.runtime).toEqual({
      kind: "simulated",
      supportsResize: false,
      supportsReplay: true,
      supportsReconnect: false,
    });
  });

  it("carries a stable reason code and optional remediation when a shell cannot open", () => {
    const shell = normalizeShellSession({
      shellId: "shell-4",
      sessionId: "session-1",
      state: "failed",
      runtime: {
        kind: "unavailable",
        reasonCode: "workspace_provider_unavailable",
        remediation: "Reconnect the SSH profile.",
      },
    });

    expect(shell.runtime).toEqual({
      kind: "unavailable",
      reasonCode: "workspace_provider_unavailable",
      remediation: "Reconnect the SSH profile.",
    });
  });

  it("omits remediation rather than inventing one", () => {
    const shell = normalizeShellSession({
      shellId: "shell-5",
      sessionId: "session-1",
      state: "failed",
      runtime: { kind: "unavailable", reasonCode: "shell_not_found" },
    });

    expect(shell.runtime).toEqual({ kind: "unavailable", reasonCode: "shell_not_found" });
  });

  it("rejects an unknown runtime kind instead of widening the union at runtime", () => {
    expect(() => normalizeShellSession({
      shellId: "shell-6",
      sessionId: "session-1",
      state: "connected",
      runtime: { kind: "quantum" },
    })).toThrow("Invalid shell runtime descriptor.");
  });

  it("rejects a remote descriptor that is missing its connection witnesses", () => {
    expect(() => normalizeShellSession({
      shellId: "shell-7",
      sessionId: "session-1",
      state: "connected",
      runtime: { kind: "remote", supportsReconnect: false },
    })).toThrow("Invalid shell runtime descriptor.");
  });

  it("rejects an unknown connection state", () => {
    expect(() => normalizeShellSession({
      shellId: "shell-8",
      sessionId: "session-1",
      state: "hibernating",
      runtime: { kind: "native" },
    })).toThrow("Invalid shell session.");
  });
});
