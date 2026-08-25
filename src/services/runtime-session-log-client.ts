import { detectRuntimeKind } from "./runtime-adapter";
import { createNativeSessionLogTransport } from "./tauri-native-session-log-transport";
import { createTauriSessionLogClient } from "./tauri-session-log-client";
import { createWebSessionLogClient } from "./web-session-log-client";
import type { SessionLogNoticeStream } from "../types/session-log-notice";

/**
 * The live log stream for whichever runtime is hosting the UI.
 *
 * A separate binding rather than a branch inside the panel, for the same reason every other
 * adapter here is: an `isTauri` check in a component is exactly what ARCH-FE-002 exists to stop,
 * and it would put the decision in as many places as there are views.
 *
 * Both runtimes share the dispatcher, so de-duplication and gap detection cannot drift between
 * them. Only the delivery differs — a Tauri event channel or a fixture emitter.
 */
export function createSessionLogNoticeStream(): SessionLogNoticeStream {
  return detectRuntimeKind() === "tauri"
    ? createTauriSessionLogClient(createNativeSessionLogTransport())
    : createWebSessionLogClient();
}

export const sessionLogNoticeStream = createSessionLogNoticeStream();
