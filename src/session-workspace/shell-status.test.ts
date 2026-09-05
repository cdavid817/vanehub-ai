import { describe, expect, it } from "vitest";
import type {
  SessionShellDescriptor,
  SessionShellState,
} from "../types/session-workspace-shell-frames";
import { acceptsInput, isShellOpening, shellControls } from "./shell-status";

function shell(
  state: SessionShellState,
  overrides: Partial<SessionShellDescriptor> = {},
): SessionShellDescriptor {
  return {
    shellId: "shell-1",
    generation: 1,
    sessionId: "session-1",
    title: "Shell",
    runtime: { kind: "native", supportsResize: true, supportsReplay: true, supportsReconnect: false },
    state,
    createdAt: "2026-01-01T00:00:00.000Z",
    lastActivityAt: "2026-01-01T00:00:00.000Z",
    revision: 1,
    foregroundProcess: "absent",
    ...overrides,
  };
}

describe("binding a keyboard to a Shell", () => {
  it("accepts input only once the runtime has committed", () => {
    expect(acceptsInput(shell("running"))).toBe(true);

    // Addressable and not writable. A keystroke accepted here races the handoff that decides
    // whether the Shell exists at all, and the native store refuses it — so binding a keyboard does
    // not make the key arrive, it makes an error arrive for a key pressed at a terminal that looked
    // ready.
    expect(acceptsInput(shell("opening"))).toBe(false);
    expect(acceptsInput(shell("starting"))).toBe(false);
  });

  it("refuses input for every state on the way out", () => {
    for (const state of ["closing", "reaping", "close_failed", "exited", "closed"] as const) {
      expect(acceptsInput(shell(state))).toBe(false);
    }
  });

  it("names the way in separately from the way out", () => {
    expect(isShellOpening(shell("opening"))).toBe(true);
    expect(isShellOpening(shell("starting"))).toBe(true);
    // Both refuse input; only one of them is going to start accepting it.
    expect(isShellOpening(shell("reaping"))).toBe(false);
  });
});

describe("which controls a Shell can offer", () => {
  it("offers both while the Shell is running", () => {
    expect(shellControls(shell("running"))).toEqual({
      canRename: true,
      canClose: true,
      closeIntent: "close",
    });
  });

  it("withholds close while an attempt is already under way", () => {
    // The aggregate refuses a second attempt from `closing` or `reaping`, so a button that stayed
    // enabled would produce an error for a press whose only honest answer is "already happening".
    for (const state of ["closing", "reaping"] as const) {
      const controls = shellControls(shell(state));
      expect(controls.canClose).toBe(false);
      expect(controls.canRename).toBe(false);
    }
  });

  it("offers a retry only when the failure said one would work", () => {
    const retryable = shellControls(shell("close_failed", { retryable: true }));
    expect(retryable.canClose).toBe(true);
    // A different word, because the same button doing a different thing under the same label would
    // let a reader press it believing nothing had been tried yet.
    expect(retryable.closeIntent).toBe("retry");

    // A wall rather than a wait. Offering a retry here invites the reader to press again for an
    // answer nothing is going to give.
    const permanent = shellControls(shell("close_failed", { retryable: false }));
    expect(permanent.canClose).toBe(false);
    // And a descriptor that did not answer the question is treated as the wall, not the wait: an
    // absent flag is "unknown", and guessing "retryable" would be the guess that wastes the press.
    expect(shellControls(shell("close_failed")).canClose).toBe(false);
  });

  it("offers nothing for a Shell that is gone", () => {
    expect(shellControls(shell("closed"))).toEqual({
      canRename: false,
      canClose: false,
      closeIntent: "close",
    });
  });

  it("still lets an ended Shell be closed, because its entry is still there", () => {
    // `exited` is a process that stopped, not a Shell that was dismissed. Its transcript is what the
    // reader came back for, and closing is what finally removes it.
    const controls = shellControls(shell("exited", { exitCode: 1 }));
    expect(controls.canClose).toBe(true);
    expect(controls.canRename).toBe(true);
  });
});
