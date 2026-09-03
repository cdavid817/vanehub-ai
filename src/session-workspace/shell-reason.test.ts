import { describe, expect, it } from "vitest";
import { sessionShellDescriptorSchema } from "../contracts/session-workspace-shell-frames";
import { shellEndingDetail } from "./shell-status";
import { shellReasonCodes, shellReasonKey } from "./shell-reason";

const base = {
  shellId: "shell-1",
  generation: 1,
  sessionId: "session-1",
  title: "Shell",
  runtime: { kind: "native", supportsResize: true, supportsReplay: true, supportsReconnect: false },
  state: "running",
  createdAt: "2026-01-01T00:00:00.000Z",
  lastActivityAt: "2026-01-01T00:00:00.000Z",
  revision: 1,
  foregroundProcess: "absent",
};

describe("shell reason wording", () => {
  it("has a key for every code the native lifecycle can carry", () => {
    // The matching Rust list is `shell_reason_code`. Pinning the count makes adding one on either
    // side a deliberate act rather than a silent divergence.
    expect(shellReasonCodes).toHaveLength(11);
    for (const code of shellReasonCodes) {
      expect(shellReasonKey(code)).toBe(`sessionTabs.shell.reason.${code}`);
    }
  });

  it("says nothing rather than showing a token this build cannot word", () => {
    // A native build newer than this frontend. `shell_close_deadline_reached` in front of a reader
    // is an identifier this application invented, and the state beside it is still true.
    expect(shellReasonKey("shell_some_future_reason")).toBeNull();
    expect(shellReasonKey(undefined)).toBeNull();
  });
});

describe("what a Shell shows beside its state", () => {
  it("shows an exit code as a number and a reason as a translatable key", () => {
    expect(shellEndingDetail({ ...base, state: "exited", exitCode: 1 } as never)).toEqual({
      exitCode: 1,
    });
    expect(
      shellEndingDetail({
        ...base,
        state: "close_failed",
        reason: "shell_terminate_failed",
      } as never),
    ).toEqual({ reasonKey: "sessionTabs.shell.reason.shell_terminate_failed" });
  });

  it("shows nothing for a reason it cannot word", () => {
    expect(
      shellEndingDetail({ ...base, state: "failed", reason: "shell_unknown_future" } as never),
    ).toBeNull();
  });

  it("shows nothing for an exit with no code, rather than inventing zero", () => {
    // A runtime that could not report a code has not said the process succeeded, and `0` is what a
    // reader would take that for.
    expect(shellEndingDetail({ ...base, state: "exited" } as never)).toBeNull();
  });
});

describe("descriptor compatibility", () => {
  it("accepts a reason code this build has never heard of", () => {
    // Parsed rather than rejected: a stricter schema would fail the whole descriptor over one
    // unfamiliar token, and a Shell that cannot be parsed is a Shell that vanishes from the list —
    // taking the only handle on a live process with it.
    const parsed = sessionShellDescriptorSchema.parse({
      ...base,
      state: "failed",
      reason: "shell_reason_from_a_newer_build",
    });

    expect(parsed.reason).toBe("shell_reason_from_a_newer_build");
  });

  it("keeps the retryable flag when a close failed, and its absence otherwise", () => {
    const failed = sessionShellDescriptorSchema.parse({
      ...base,
      state: "close_failed",
      reason: "shell_terminate_failed",
      retryable: false,
    });
    const running = sessionShellDescriptorSchema.parse(base);

    // `false` and absent are different answers. A Shell nobody has tried to close has not answered
    // the question, and defaulting it to `false` would tell a view that a retry is pointless.
    expect(failed.retryable).toBe(false);
    expect(running.retryable).toBeUndefined();
  });

  it("refuses a state token it does not know", () => {
    // The opposite decision from the reason code, and deliberately so: a state drives what the view
    // allows — input, close, retry — and guessing at one would let a view offer an operation the
    // native side refuses. A reason only decorates.
    expect(() => sessionShellDescriptorSchema.parse({ ...base, state: "zombie" })).toThrow();
  });
});
