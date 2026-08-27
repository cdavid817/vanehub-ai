import type { SessionRecoverySummary } from "../services/agent-service";
import type { Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { SessionRecoveryNotice } from "./session-recovery-notice";
import { SessionRuntimeFailureNotice } from "./session-runtime-failure-notice";

/**
 * Two independent notices, kept independent. Crash recovery reconciles business evidence and asks
 * for an acknowledgement decision; runtime failure asks for a retry. Each renders only for its own
 * condition, so a session can show either, both, or neither.
 */
export function SessionNotices({
  acknowledging,
  messages,
  onAcknowledge,
  onRecover,
  recovering,
  recoverySummary,
  session,
}: {
  acknowledging: boolean;
  messages: ChatMessage[];
  onAcknowledge: () => Promise<void>;
  onRecover: (session: Session) => void;
  recovering: boolean;
  recoverySummary: SessionRecoverySummary | null;
  session: Session | null;
}) {
  return (
    <>
      <SessionRecoveryNotice
        acknowledging={acknowledging}
        onAcknowledge={onAcknowledge}
        session={session}
        summary={recoverySummary}
      />
      <SessionRuntimeFailureNotice
        messages={messages}
        onRecover={() => { if (session) onRecover(session); }}
        recovering={recovering}
        session={session}
      />
    </>
  );
}
