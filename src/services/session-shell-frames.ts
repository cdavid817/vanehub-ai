import type {
  SessionShellNotice,
  ShellOutputFrame,
} from "../types/session-workspace-shell-frames";
import type { SessionShellEvent } from "./session-shell-service";

export interface ShellFrameDispatcher {
  /** Feeds one notice through de-duplication and gap detection. */
  accept(notice: SessionShellNotice): void;
}

export interface ShellFrameDispatcherOptions {
  shellId: string;
  /** The first sequence this view has not seen, from the attach snapshot. */
  fromSequence: number;
  listener(event: SessionShellEvent): void;
}

/**
 * The one place replay and live frames are reconciled.
 *
 * Written once and shared by both runtimes on purpose: if each client did its own de-duplication,
 * the desktop and the mock would drift on exactly the behaviour that is hardest to see — a frame
 * delivered twice looks like the shell echoed, and a frame skipped looks like the command produced
 * less output than it did.
 */
export function createShellFrameDispatcher(
  options: ShellFrameDispatcherOptions,
): ShellFrameDispatcher {
  // The last sequence handed to the listener. Frames at or below it are already rendered, which is
  // the normal case for the overlap between an attach snapshot and the events that raced it.
  let lastSequence = Math.max(0, Math.trunc(options.fromSequence) - 1);
  return {
    accept(notice: SessionShellNotice): void {
      if (notice.shellId !== options.shellId) return;
      if (notice.type === "state") {
        options.listener(notice);
        return;
      }
      if (notice.sequence <= lastSequence) return;
      if (notice.sequence > lastSequence + 1) {
        // Announced before the frame that revealed it, so the scrollback reads in order: the
        // missing range, then what came after it.
        options.listener({
          type: "gap",
          shellId: options.shellId,
          gap: {
            fromSequence: lastSequence + 1,
            toSequence: notice.sequence - 1,
            reason: "shell_frame_gap",
          },
        });
      }
      lastSequence = notice.sequence;
      options.listener(notice);
    },
  };
}

/**
 * Orders buffered frames by sequence before they are replayed.
 *
 * Arrival order is not sequence order once frames have been held: two events that raced into the
 * buffer can land either way round, and replaying them as they arrived would report a gap that
 * never existed and then swallow the frame that filled it.
 */
export function orderBufferedNotices(notices: SessionShellNotice[]): SessionShellNotice[] {
  return [...notices].sort((left, right) => sequenceOf(left) - sequenceOf(right));
}

function sequenceOf(notice: SessionShellNotice): number {
  return notice.type === "output" ? notice.sequence : Number.MAX_SAFE_INTEGER;
}

/** Concatenates replay frames into the text a terminal writes, in sequence order. */
export function replayText(frames: ShellOutputFrame[]): string {
  return [...frames].sort((left, right) => left.sequence - right.sequence).map((frame) => frame.data).join("");
}

/**
 * Makes an unsubscribe safe to call more than once.
 *
 * A React effect cleanup can run twice in development and once more on unmount; a second teardown
 * of a listener that has already been replaced would silence the view that replaced it.
 */
export function onceDetach(detach: () => Promise<void>): () => Promise<void> {
  let done: Promise<void> | null = null;
  return () => {
    done ??= detach();
    return done;
  };
}
