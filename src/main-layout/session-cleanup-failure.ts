/**
 * Why archiving or deleting a session was refused, when the reason is one the user can act on.
 *
 * Archive and delete became strict: a session whose retained Shells are not all confirmed gone is
 * not a session that finished archiving, because removing it would delete the last thing that could
 * reach a process still running behind it. That is the right refusal, but the native code arrives
 * as a bare token inside a validation message, and both mutations had no error handler at all — so
 * the user clicked, the backend refused, and nothing happened. Silence is a worse answer than a
 * generic failure: it looks like the click was not registered, so the natural response is to click
 * again.
 */

/** The stable code `WorkspaceApi::kill_shells_for_session` refuses with. */
const SESSION_SHELL_CLEANUP_INCOMPLETE = "session_shell_cleanup_incomplete";

/**
 * Whether this failure is the strict-cleanup refusal.
 *
 * Substring rather than equality: the code crosses two contexts and a command boundary, and each
 * layer is entitled to wrap it in a sentence. Matching the whole message would mean this check
 * breaks the first time somebody adds a prefix, and the failure mode of that is the silence this
 * module exists to remove.
 */
export function isSessionShellCleanupIncomplete(reason: unknown): boolean {
  const message = reason instanceof Error ? reason.message : String(reason);
  return message.includes(SESSION_SHELL_CLEANUP_INCOMPLETE);
}

/**
 * The i18n key describing a failed archive or delete.
 *
 * The cleanup case gets its own copy because the two are different situations for the user: one is
 * "something went wrong", the other is "this is still finishing, and trying again shortly will
 * work". Only the second is actionable, and only the second is true here.
 */
export function sessionCleanupFailureKey(reason: unknown, operation: "archive" | "delete"): string {
  if (!isSessionShellCleanupIncomplete(reason)) return "app.error.title";
  return operation === "archive"
    ? "layout.archiveBlockedByShellCleanup"
    : "layout.deleteBlockedByShellCleanup";
}
