import type { ShellRuntimeDescriptor } from "./session-workspace";

/**
 * `pty` is not a synonym for stdout. A PTY merges both streams before the runtime ever sees them,
 * so a frame that came from one says `pty` rather than guessing which side it belonged to.
 */
export type ShellStream = "pty" | "stdout" | "stderr" | "system";

/**
 * Where a retained Shell is in its life.
 *
 * `exited` and `closed` are different endings and stay apart: a process that ended on its own is
 * still worth reading and stays attachable for replay, whereas a Shell the user closed is gone.
 * Distinct from `ShellConnectionState`, which describes the older one-view shell that is torn down
 * with its tab — collapsing the two would let a tab switch read as a process ending.
 */
export type SessionShellState =
  | "starting"
  /** Registered and addressable, but the runtime has not committed ownership. Not writable. */
  | "opening"
  | "running"
  /** Close was asked for and one bounded attempt is under way. Not terminal. */
  | "closing"
  /** A close attempt ran out of time; cleanup continues under retained ownership. Not terminal. */
  | "reaping"
  /** Cleanup failed with a reason and the resources are still owned. Not terminal. */
  | "close_failed"
  | "exited"
  | "disconnected"
  | "failed"
  | "closed";

/**
 * The three states in which a Shell has been asked to end and has not.
 *
 * A view needs this predicate rather than `state !== "closed"`, because the difference it protects
 * is the one the user acts on: the tab is still there, the process may still be running, and
 * removing it from the list would take away the only way to retry.
 */
export function isShellCleanupPending(state: SessionShellState): boolean {
  return state === "closing" || state === "reaping" || state === "close_failed";
}

/**
 * What a close attempt achieved.
 *
 * Four values rather than a resolved promise, because "the call returned" and "the process is gone"
 * are different facts. `reaping` and `closeFailed` both resolve successfully and both mean a
 * process may still exist.
 */
export type ShellCloseDisposition = "closed" | "reaping" | "close_failed" | "already_terminal";

export interface ShellCloseOutcome {
  shellId: string;
  generation: number;
  disposition: ShellCloseDisposition;
  /** Present only for a settled disposition; an unsettled close observed nothing final. */
  finalState?: SessionShellState;
  reason?: string;
  retryable: boolean;
  attempt: number;
  cleanupDeadlineReached: boolean;
}

/** Whether the Shell can be considered finished with. Not "the call did not throw". */
export function isCloseSettled(disposition: ShellCloseDisposition): boolean {
  return disposition === "closed" || disposition === "already_terminal";
}

/**
 * Whether something is running in the foreground. Three values, not two: `unknown` is what an
 * opaque runtime honestly reports, and rendering it as `absent` would let a close confirmation say
 * "nothing is running" about a shell midway through a deploy.
 */
export type ShellForegroundProcessState = "present" | "absent" | "unknown";

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
  /**
   * Which life of this Shell id the descriptor describes. Compared against an arriving notice so a
   * completion for a Shell this view has already replaced is discarded rather than applied.
   */
  generation: number;
  sessionId: string;
  seatId?: string;
  title: string;
  runtime: ShellRuntimeDescriptor;
  state: SessionShellState;
  /** Present only for the states that carry one. */
  reason?: string;
  exitCode?: number;
  createdAt: string;
  lastActivityAt: string;
  /**
   * Counts descriptor changes, not frames. A view compares revisions to decide whether a state
   * notice is newer than what it holds; a timestamp cannot answer that when two changes land inside
   * one clock tick, and the output sequence cannot either — output moves while the descriptor
   * stands still.
   */
  revision: number;
  foregroundProcess: ShellForegroundProcessState;
}

export interface ShellAttachSnapshot {
  /**
   * This view's claim on the Shell. Every later write, resize, and detach carries it back, which is
   * what lets a late cleanup detach without tearing down the attachment that replaced it.
   */
  attachmentId: string;
  descriptor: SessionShellDescriptor;
  replay: ShellOutputFrame[];
  /** The first sequence the caller has not seen. Live frames continue monotonically from here. */
  nextSequence: number;
  gap?: ShellReplayGap;
}

/** A state change published while a view is attached. */
export interface SessionShellStateNotice {
  shellId: string;
  generation: number;
  sessionId: string;
  state: SessionShellState;
  reason?: string;
  exitCode?: number;
  revision: number;
  occurredAt: string;
}

export type SessionShellNotice =
  | ({ type: "output" } & ShellOutputFrame & { sessionId: string })
  | ({ type: "state" } & SessionShellStateNotice);

/** Retained output bound, in bytes. Enforced by the native registry; stated here so a view can
 * explain an eviction without hard-coding its own number. */
export const SHELL_RETAINED_OUTPUT_BYTES = 1024 * 1024;
