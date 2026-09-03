/**
 * The reason codes a Shell's lifecycle can carry, and the wording this build has for them.
 *
 * Pinned here rather than accepted as an open string, because the whole point of a code is that the
 * frontend owns the sentence — and a code with no sentence renders as a raw token. Before this,
 * `shell_close_deadline_reached` went straight onto the strip beside the state.
 *
 * The matching Rust list is `shell_reason_code`. A build newer than this frontend can send a code
 * that is not here; that degrades to showing nothing beside the state, which is vaguer and true,
 * rather than showing a reader an identifier.
 */
export const shellReasonCodes = [
  "shell_capacity_exhausted",
  "shell_open_setup_failed",
  "shell_startup_cleanup_pending",
  "shell_close_deadline_reached",
  "shell_terminate_failed",
  "shell_worker_completion_pending",
  "shell_reap_deadline_reached",
  "shell_reaper_capacity_exhausted",
  "shell_generation_stale",
  "session_shell_cleanup_incomplete",
  "shell_startup_buffer_overflow",
] as const;

export type ShellReasonCode = (typeof shellReasonCodes)[number];

/** The i18n key for a Shell reason, or `null` when this build has no wording for it. */
export function shellReasonKey(reasonCode: string | undefined): string | null {
  if (!reasonCode) return null;
  return (shellReasonCodes as readonly string[]).includes(reasonCode)
    ? `sessionTabs.shell.reason.${reasonCode}`
    : null;
}
