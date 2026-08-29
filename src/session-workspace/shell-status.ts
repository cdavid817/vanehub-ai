import type {
  SessionShellDescriptor,
  ShellForegroundProcessState,
} from "../types/session-workspace-shell-frames";
import { isShellCleanupPending } from "../types/session-workspace-shell-frames";

/** The locale key for a retained Shell's state. Kept apart from the legacy connection-state keys,
 * which describe a different lifecycle with overlapping words. */
export function shellStateKey(descriptor: SessionShellDescriptor): string {
  return `sessionTabs.shell.sessionState.${descriptor.state}`;
}

export function shellRuntimeKey(descriptor: SessionShellDescriptor): string {
  return `sessionTabs.shell.runtime.${descriptor.runtime.kind}`;
}

/**
 * Whether a view should still bind a keyboard to this Shell.
 *
 * `running` alone, matching the native store. `opening` and `starting` are addressable and not
 * writable: the runtime has not committed ownership, so a keystroke accepted there races the handoff
 * that decides whether the Shell exists at all. Binding a keyboard in those states does not make the
 * keystroke arrive — it makes the refusal arrive as an error, for a key the reader pressed while the
 * terminal looked ready.
 */
export function acceptsInput(descriptor: SessionShellDescriptor): boolean {
  return descriptor.state === "running";
}

/**
 * Whether this Shell is on its way in and not yet usable.
 *
 * Distinct from `isShellCleanupPending`, which is the same shape of fact on the way out. A view uses
 * this to keep the tab visible and the terminal readable while refusing input, rather than showing
 * an enabled prompt that swallows what is typed into it.
 */
export function isShellOpening(descriptor: SessionShellDescriptor): boolean {
  return descriptor.state === "starting" || descriptor.state === "opening";
}

export type CloseWarning = "running" | "unknown" | "none";

/**
 * What a close confirmation is allowed to say about foreground work.
 *
 * Three answers rather than two. `unknown` is what an opaque runtime honestly reports, and folding
 * it into `none` would let the dialog say "nothing is running" about a shell midway through a
 * deploy — a claim the product cannot support and the user would act on.
 */
export function closeWarningFor(foreground: ShellForegroundProcessState): CloseWarning {
  if (foreground === "present") return "running";
  if (foreground === "unknown") return "unknown";
  return "none";
}

/** The suffix a finished Shell shows next to its state, when the runtime reported one. */
export function shellEndingDetail(descriptor: SessionShellDescriptor): string | null {
  if (descriptor.state === "exited" && descriptor.exitCode !== undefined) {
    return String(descriptor.exitCode);
  }
  return descriptor.reason ?? null;
}

/**
 * Which controls a Shell in this state can honestly offer.
 *
 * Derived once rather than decided at each button, because the rules are about the process rather
 * than about the widget, and three buttons deciding separately is three places to get "a close is
 * already under way" wrong.
 */
export interface ShellControls {
  /** Renaming touches the registry entry, which a Shell mid-cleanup no longer accepts changes to. */
  canRename: boolean;
  canClose: boolean;
  /** Whether the close control is starting an attempt or retrying one that failed. */
  closeIntent: "close" | "retry";
}

export function shellControls(descriptor: SessionShellDescriptor): ShellControls {
  const cleanupPending = isShellCleanupPending(descriptor.state);
  const failed = descriptor.state === "close_failed";
  return {
    canRename: !cleanupPending && descriptor.state !== "closed",
    // Not while an attempt is already running. `closing` and `reaping` are attempts in progress, and
    // the aggregate refuses a second one — so a button that stayed enabled would produce an error
    // for a press whose only honest answer is "already happening".
    //
    // A failed close is retryable in place, but only when it said so. `close_failed` without the
    // flag is a wall rather than a wait, and offering a retry there invites the reader to press
    // again for an answer nothing is going to give.
    canClose:
      descriptor.state !== "closed" &&
      descriptor.state !== "closing" &&
      descriptor.state !== "reaping" &&
      (!failed || descriptor.retryable === true),
    closeIntent: failed ? "retry" : "close",
  };
}
