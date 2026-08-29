import type {
  SessionShellDescriptor,
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
