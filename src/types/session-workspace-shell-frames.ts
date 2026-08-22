import type { ShellConnectionState, ShellRuntimeDescriptor } from "./session-workspace";

/**
 * `pty` is not a synonym for stdout. A PTY merges both streams before the runtime ever sees them,
 * so a frame that came from one says `pty` rather than guessing which side it belonged to.
 */
export type ShellStream = "pty" | "stdout" | "stderr" | "system";

export interface ShellOutputFrame {
  shellId: string;
  sequence: number;
  occurredAt: string;
  stream: ShellStream;
  data: string;
}

/** Emitted once when retained output was evicted, so a reattaching view can say so rather than
 * presenting a silently shortened scrollback as continuous. */
export interface ShellReplayGap {
  fromSequence: number;
  toSequence: number;
  reason: string;
}

export interface SessionShellDescriptor {
  shellId: string;
  sessionId: string;
  seatId?: string;
  title: string;
  runtime: ShellRuntimeDescriptor;
  state: ShellConnectionState;
  createdAt: string;
  lastActivityAt: string;
}

export interface ShellAttachSnapshot {
  descriptor: SessionShellDescriptor;
  replay: ShellOutputFrame[];
  /** The first sequence the caller has not seen. Live frames continue monotonically from here. */
  nextSequence: number;
  gap?: ShellReplayGap;
}

/** Retained output bound, in bytes. Enforced by the native registry; stated here so a view can
 * explain an eviction without hard-coding its own number. */
export const SHELL_RETAINED_OUTPUT_BYTES = 1024 * 1024;
