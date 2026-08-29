import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { SessionLogUnsubscribe } from "../types/session-log-notice";
import {
  SessionLogUnavailableError,
  type NativeSessionLogTransport,
  type SessionLogCommandName,
} from "./native-session-log-transport";

/**
 * The native event channel. It has to match `SESSION_LOG_EVENT` in the Rust notice publisher
 * verbatim; a mismatch produces a subscription that never fires and never errors, which is the one
 * failure mode a live view cannot detect from the inside.
 */
export const SESSION_LOG_EVENT_CHANNEL = "session-log:appended";

/**
 * The commands registered in the Rust core registry.
 *
 * A command absent from this set is refused here rather than passed to `invoke()`, because Tauri
 * answers an unregistered command with an opaque framework string that the UI cannot tell apart
 * from a genuine runtime fault.
 */
const REGISTERED: ReadonlySet<string> = new Set<SessionLogCommandName>([
  "get_session_log_subscription_bootstrap",
]);

/**
 * The desktop binding.
 *
 * Separate from `native-session-log-transport.ts` on purpose: that file defines the seam and the
 * typed unavailable binding, and this is the only implementation that touches a Tauri API. Keeping
 * them apart is what makes "which commands are registered" have exactly one place to be written.
 */
export function createNativeSessionLogTransport(): NativeSessionLogTransport {
  return {
    async invokeSessionLog(command, payload) {
      if (!REGISTERED.has(command)) {
        throw new SessionLogUnavailableError(
          "log_index_unavailable",
          `Native session-log command is not registered: ${command}`,
        );
      }
      try {
        return await invoke(command, payload as Record<string, unknown>);
      } catch (reason: unknown) {
        // Native log errors are stable codes, never prose. Forwarding the raw string would put a
        // value the UI matches on at the mercy of translation.
        throw new SessionLogUnavailableError(reasonCode(reason));
      }
    },
    async subscribeSessionLogNotices(handler): Promise<SessionLogUnsubscribe> {
      const stop = await listen<unknown>(SESSION_LOG_EVENT_CHANNEL, (event) => handler(event.payload));
      return () => stop();
    },
  };
}

/**
 * The code inside a native error string, or a generic one.
 *
 * The native side returns `validation error: log_cursor_filter_mismatch` and similar, so the code
 * is the last token. Anything unrecognised becomes `log_index_unavailable` rather than being shown:
 * an error the UI cannot classify is one it must not present as if it could.
 */
function reasonCode(reason: unknown): string {
  const text = typeof reason === "string" ? reason : reason instanceof Error ? reason.message : "";
  const match = text.match(/log_[a-z_]+/);
  return match ? match[0] : "log_index_unavailable";
}
