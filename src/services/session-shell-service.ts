import type {
  SessionShellDescriptor,
  SessionShellNotice,
  ShellAttachSnapshot,
  ShellReplayGap,
} from "../types/session-workspace-shell-frames";

/**
 * What an attached view receives.
 *
 * Wider than the wire notice by one case: a gap is detected here, not sent. The registry publishes
 * a contiguous stream and cannot know what a subscriber failed to receive, so the client that spots
 * the jump is the only thing that can report it — and a view that saw the frames close up without
 * the marker would read a shortened transcript as complete.
 */
export type SessionShellEvent =
  | SessionShellNotice
  | { type: "gap"; shellId: string; gap: ShellReplayGap };

export interface CreateSessionShellInput {
  sessionId: string;
  rows: number;
  cols: number;
  seatId?: string;
  /**
   * The client's idempotency key. Absent asks for "the default Shell for this session and seat",
   * which is what a tab opening for the first time wants: two of those racing must produce one
   * Shell. Present means the user pressed Add, and a retried press must not produce a second one.
   */
  requestId?: string;
  title?: string;
}

export interface AttachSessionShellInput {
  shellId: string;
  /** The last sequence this view consumed. Omitted asks for everything still retained. */
  afterSequence?: number;
}

export interface ShellAttachmentScope {
  shellId: string;
  attachmentId: string;
}

export interface WriteSessionShellInput extends ShellAttachmentScope {
  content: string;
}

export interface ResizeSessionShellInput extends ShellAttachmentScope {
  rows: number;
  cols: number;
}

/**
 * One attached view. `detach` releases the claim and stops the listener; it is idempotent, because
 * a React cleanup can run more than once and a second detach must not disturb whatever attached
 * after it.
 */
export interface ShellAttachment extends ShellAttachSnapshot {
  detach(): Promise<void>;
}

/**
 * The retained Session Shell surface.
 *
 * Every Shell here outlives the view that opened it. Nothing in this interface closes a Shell as a
 * side effect: hiding a tab, switching sessions, and unmounting all detach, and `closeSessionShell`
 * is the only way a process ends. That asymmetry is the capability — a build survives a tab switch
 * precisely because leaving is not the same call as stopping.
 */
export interface SessionShellService {
  listSessionShells(sessionId: string): Promise<SessionShellDescriptor[]>;

  createSessionShell(input: CreateSessionShellInput): Promise<SessionShellDescriptor>;

  /**
   * Registers the listener before claiming the Shell and replays whatever arrived in between.
   *
   * Attaching first and subscribing after would lose every frame emitted in that window, and
   * nothing downstream could detect the loss: the sequences would be contiguous from the
   * subscriber's point of view, so the gap would render as ordinary output.
   */
  attachSessionShell(
    input: AttachSessionShellInput,
    listener: (event: SessionShellEvent) => void,
  ): Promise<ShellAttachment>;

  detachSessionShell(scope: ShellAttachmentScope): Promise<void>;

  writeSessionShell(input: WriteSessionShellInput): Promise<void>;

  resizeSessionShell(input: ResizeSessionShellInput): Promise<void>;

  renameSessionShell(input: { shellId: string; title: string }): Promise<SessionShellDescriptor>;

  /** Ends the process. The only call that does. */
  closeSessionShell(shellId: string): Promise<void>;
}
