import type { NativeSessionLogTransport } from "../../services/native-session-log-transport";
import type { SessionLogUnsubscribe } from "../../types/session-log-notice";

export interface FixtureSessionLogTransport extends NativeSessionLogTransport {
  /** Feeds one raw payload to every handler, exactly as the native event channel would. */
  publish: (payload: unknown) => void;
  /** The watermark the bootstrap command answers with. */
  setWatermark: (sequence: number) => void;
  /** Handlers currently registered. A release that did not release shows up here. */
  handlerCount: () => number;
}

/**
 * The fixture the desktop client is driven through.
 *
 * Payloads are `unknown` on purpose: the native channel delivers whatever it delivers, and the
 * client's own validation is a large part of what is under test. A fixture that handed over
 * already-typed notices would test the dispatcher and skip the parser, which is the half that
 * actually faces a wire format.
 */
export function createFixtureSessionLogTransport(): FixtureSessionLogTransport {
  const handlers = new Set<(payload: unknown) => void>();
  let watermark = 0;

  return {
    invokeSessionLog(command) {
      if (command === "get_session_log_subscription_bootstrap") {
        return Promise.resolve({ watermarkSequence: watermark, coverage: { state: "complete" } });
      }
      return Promise.reject(new Error(`unexpected command: ${command}`));
    },
    subscribeSessionLogNotices(handler): Promise<SessionLogUnsubscribe> {
      handlers.add(handler);
      return Promise.resolve(() => {
        handlers.delete(handler);
      });
    },
    publish(payload) {
      for (const handler of [...handlers]) handler(payload);
    },
    setWatermark(sequence) {
      watermark = sequence;
    },
    handlerCount: () => handlers.size,
  };
}
