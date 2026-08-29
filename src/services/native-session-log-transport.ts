import type { SessionLogUnsubscribe } from "../types/session-log-notice";

/**
 * The native commands the log-notice client calls. Named here rather than inline so the set that
 * has to stay registered is visible in one place.
 */
export type SessionLogCommandName = "get_session_log_subscription_bootstrap";

/**
 * The seam between the log-notice client and the desktop runtime.
 *
 * Injecting it is what lets the client's parsing, watermark handling and gap detection be settled
 * against a fixture, while the binding the application actually uses answers with a typed reason
 * code. Without the seam the only way to test the client would be to `invoke()` a command the
 * runtime may not know, which returns an opaque framework error — a failure the UI cannot tell
 * apart from a genuine fault.
 */
export interface NativeSessionLogTransport {
  invokeSessionLog(command: SessionLogCommandName, payload: unknown): Promise<unknown>;
  subscribeSessionLogNotices(
    handler: (payload: unknown) => void,
  ): Promise<SessionLogUnsubscribe>;
}

/**
 * A refusal the UI can localize, as distinct from a thrown framework string. `reasonCode` is one of
 * the stable codes the native surface returns, never prose: a client acts differently on an
 * unavailable index than on a stale cursor, and a sentence would make that a string-matching
 * exercise that breaks on translation.
 */
export class SessionLogUnavailableError extends Error {
  readonly reasonCode: string;

  constructor(reasonCode: string, message?: string) {
    super(message ?? reasonCode);
    this.name = "SessionLogUnavailableError";
    this.reasonCode = reasonCode;
  }
}

export function isSessionLogUnavailableError(
  value: unknown,
): value is SessionLogUnavailableError {
  return value instanceof SessionLogUnavailableError;
}

/**
 * The binding for a runtime with no native log index. Refuses uniformly rather than invoking
 * anything, so a panel shows "not available in this runtime" instead of a framework error that
 * reads like a crash.
 */
export const unavailableSessionLogTransport: NativeSessionLogTransport = {
  invokeSessionLog(command) {
    return Promise.reject(
      new SessionLogUnavailableError(
        "log_index_unavailable",
        `Native session-log command is not available in this runtime: ${command}`,
      ),
    );
  },
  subscribeSessionLogNotices() {
    // A subscription that resolves to a no-op rather than rejecting: a view that cannot receive
    // notices should render its page and say coverage is unavailable, not fail to mount.
    return Promise.resolve(() => {});
  },
};
