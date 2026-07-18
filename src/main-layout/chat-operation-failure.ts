import type { ClientLogEvent } from "../types/settings";

export function createChatOperationFailureEvent(source: string, reason: unknown): ClientLogEvent {
  return {
    level: "error",
    kind: "critical-operation-failure",
    message: reason instanceof Error ? reason.message : String(reason),
    source,
  };
}
