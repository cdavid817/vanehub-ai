import type {
  SessionShellDescriptor,
  ShellForegroundProcessState,
} from "../types/session-workspace-shell-frames";

/** The locale key for a retained Shell's state. Kept apart from the legacy connection-state keys,
 * which describe a different lifecycle with overlapping words. */
export function shellStateKey(descriptor: SessionShellDescriptor): string {
  return `sessionTabs.shell.sessionState.${descriptor.state}`;
}

export function shellRuntimeKey(descriptor: SessionShellDescriptor): string {
  return `sessionTabs.shell.runtime.${descriptor.runtime.kind}`;
}

/** Whether a view should still bind a keyboard to this Shell. */
export function acceptsInput(descriptor: SessionShellDescriptor): boolean {
  return descriptor.state === "starting" || descriptor.state === "running";
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
