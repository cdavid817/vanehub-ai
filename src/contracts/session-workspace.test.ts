import { describe, expect, it } from "vitest";
import { normalizeShellRuntimeDescriptor } from "./session-workspace";

/**
 * The native runtime already returns a `remote` Session Shell for SSH-backed sessions. The old
 * frontend union admitted only `native | simulated`, so the value that actually crossed the
 * boundary was the one shape TypeScript said could not occur. These cases pin the wire shape of
 * every descriptor variant against the transport, not against the type declaration.
 *
 * They target the descriptor normalizer directly now that the one-view `ShellSession` wrapper is
 * gone: the descriptor is what the retained Session Shell still carries, and it is the part that
 * decides whether the view may offer resize or reconnect.
 */
describe("Session Shell runtime descriptor transport", () => {
  it("accepts a local shell and reports the capabilities a PTY actually has", () => {
    expect(normalizeShellRuntimeDescriptor({ kind: "native" })).toEqual({
      kind: "native",
      supportsResize: true,
      supportsReplay: true,
      supportsReconnect: false,
    });
  });

  it("accepts the native remote descriptor with its connection witnesses", () => {
    expect(
      normalizeShellRuntimeDescriptor({
        kind: "remote",
        connectionId: "connection-7",
        profileRevision: 3,
        supportsReconnect: false,
      }),
    ).toEqual({
      kind: "remote",
      connectionId: "connection-7",
      profileRevision: 3,
      supportsResize: true,
      supportsReplay: true,
      supportsReconnect: false,
    });
  });

  it("does not let a simulated shell claim resize support", () => {
    expect(normalizeShellRuntimeDescriptor({ kind: "simulated" })).toEqual({
      kind: "simulated",
      supportsResize: false,
      supportsReplay: true,
      supportsReconnect: false,
    });
  });

  it("carries a stable reason code and optional remediation when a shell cannot open", () => {
    expect(
      normalizeShellRuntimeDescriptor({
        kind: "unavailable",
        reasonCode: "workspace_provider_unavailable",
        remediation: "Reconnect the SSH profile.",
      }),
    ).toEqual({
      kind: "unavailable",
      reasonCode: "workspace_provider_unavailable",
      remediation: "Reconnect the SSH profile.",
    });
  });

  it("omits remediation rather than inventing one", () => {
    expect(
      normalizeShellRuntimeDescriptor({ kind: "unavailable", reasonCode: "shell_not_found" }),
    ).toEqual({ kind: "unavailable", reasonCode: "shell_not_found" });
  });

  it("rejects an unknown runtime kind instead of widening the union at runtime", () => {
    expect(() => normalizeShellRuntimeDescriptor({ kind: "quantum" })).toThrow(
      "Invalid shell runtime descriptor.",
    );
  });

  it("rejects a remote descriptor that is missing its connection witnesses", () => {
    expect(() =>
      normalizeShellRuntimeDescriptor({ kind: "remote", supportsReconnect: false }),
    ).toThrow("Invalid shell runtime descriptor.");
  });
});
