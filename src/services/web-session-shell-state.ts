import type {
  SessionShellDescriptor,
  ShellCloseDisposition,
  ShellCloseOutcome,
  ShellOutputFrame,
  ShellReplayGap,
} from "../types/session-workspace-shell-frames";
import { SHELL_RETAINED_OUTPUT_BYTES } from "../types/session-workspace-shell-frames";

/**
 * How much retained output the mock keeps per Shell, in characters.
 *
 * Far below the native byte bound on purpose: a browser demo that had to produce a megabyte of
 * output before a gap appeared would never show one, and the gap marker is behaviour the mock
 * exists to exercise. The native bound is re-exported alongside it so a view that explains an
 * eviction reads one number rather than inventing its own.
 */
export const MOCK_SHELL_RETAINED_CHARACTERS = 4096;

export { SHELL_RETAINED_OUTPUT_BYTES };

export interface MockShell {
  descriptor: SessionShellDescriptor;
  frames: ShellOutputFrame[];
  gap: ShellReplayGap | null;
  attachmentId: string | null;
  requestId: string | null;
  nextSequence: number;
  /**
   * How many bounded close attempts this Shell has had. One counter, shared by every attempt, as
   * the native store shares its own: two counters would let a Shell exhaust neither budget while
   * being tried indefinitely.
   */
  closeAttempts: number;
}

/**
 * How the mock is told to behave, so a component test can reach states a browser cannot produce.
 *
 * Nothing here spawns anything or claims to. A Shell that "will not close" in the mock is a counter
 * that has not run out — which is exactly the property a view has to render correctly, and exactly
 * what the native side cannot be asked to reproduce on demand.
 */
export interface WebSessionShellOptions {
  /** Refuses a create once this many Shells exist for one session. */
  perSessionCapacity?: number;
  /** How many close attempts a Shell survives before it confirms. `0` confirms on the first. */
  closeAttemptsBeforeConfirming?: number;
  /** When set, an unconfirmed close reports `close_failed` rather than `reaping`. */
  unconfirmedCloseFails?: boolean;
  /**
   * Holds a new Shell in `opening` until a view attaches to it.
   *
   * The mock otherwise creates straight into `running`, which means the startup phase — the one
   * where the Shell is addressable and not yet writable — never happens in the browser build at
   * all. A panel written against that mock will accept a keystroke the desktop build refuses.
   *
   * The attach is what commits it, because that is the one call in the mock's world that maps to
   * "something is now watching this Shell". A timer would be a second clock and would make every
   * test that waited on it flaky.
   */
  deferStartupUntilAttach?: boolean;
  /**
   * Fails startup with this reason instead of committing, when startup is deferred.
   *
   * A rolled-back startup reports `failed` with the reason it rolled back for. It never reports
   * `closed`: a Shell that never committed has nothing to confirm.
   */
  startupFailureReason?: string;
  /**
   * Output and a natural exit produced at create time, before any view can attach.
   *
   * The case a replay has to survive: a command that prints and exits faster than the UI can
   * subscribe. A view that only rendered live frames would show an empty terminal for a Shell that
   * had already said everything it was going to say.
   */
  fastExit?: { output: string; exitCode: number };
}

/**
 * Drops whole frames from the oldest end and records one contiguous gap, as the native buffer does.
 *
 * One gap rather than several: a reader that saw two could not tell which side of which it was
 * looking at.
 */
export function evictRetainedFrames(shell: MockShell): void {
  let retained = shell.frames.reduce((total, frame) => total + frame.data.length, 0);
  while (retained > MOCK_SHELL_RETAINED_CHARACTERS && shell.frames.length > 1) {
    const dropped = shell.frames.shift();
    if (!dropped) break;
    retained -= dropped.data.length;
    shell.gap = {
      fromSequence: shell.gap?.fromSequence ?? dropped.sequence,
      toSequence: dropped.sequence,
      reason: "shell_replay_evicted",
    };
  }
}

/**
 * What a close reports when there was nothing left to close.
 *
 * `already_terminal` rather than a confirmed close: this call ended nothing. Reporting it as
 * `closed` would let a retry after a partial failure look like the attempt that finished the job.
 */
export function alreadyTerminalClose(shellId: string): ShellCloseOutcome {
  return {
    shellId,
    generation: 0,
    disposition: "already_terminal",
    finalState: "closed",
    retryable: false,
    attempt: 0,
    cleanupDeadlineReached: false,
  };
}

/**
 * What a close reports when the simulated process is still there.
 *
 * Retryable, and carrying the attempt number: a view that only knew "not finished" could not tell a
 * first failure from a fifth, and the difference is whether offering the same button again is
 * helpful or is asking the reader to keep pressing.
 */
export function unconfirmedClose(
  shellId: string,
  generation: number,
  disposition: Extract<ShellCloseDisposition, "reaping" | "close_failed">,
  attempt: number,
): ShellCloseOutcome {
  return {
    shellId,
    generation,
    disposition,
    reason: "shell_close_deadline_reached",
    retryable: true,
    attempt,
    cleanupDeadlineReached: true,
  };
}

export function confirmedClose(
  shellId: string,
  generation: number,
  attempt: number,
): ShellCloseOutcome {
  return {
    shellId,
    generation,
    disposition: "closed",
    finalState: "closed",
    retryable: false,
    attempt,
    cleanupDeadlineReached: false,
  };
}
